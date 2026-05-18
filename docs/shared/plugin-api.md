# shared/plugin-api -- 插件 API 核心

## 概述

`shared/plugin-api` 定义了整个插件系统的基础接口。所有插件必须实现 `Plugin` trait，主程序通过此 trait 与插件交互。

模块结构：

```
shared/plugin-api/src/
├── lib.rs       # Plugin trait、PluginCreateFn 类型定义、模块导出
├── error.rs     # PluginError 枚举、PluginResult 类型别名、辅助宏
├── storage.rs   # PluginStorage -- JSON 文件持久化
└── utils.rs     # 工具函数（escape_xml）
```

## 模块总览

### error -- 错误处理

定义插件系统中所有可能的错误类型、结果类型别名和两个辅助宏。

#### PluginError 枚举

```rust
#[derive(Debug)]
pub enum PluginError {
    NotFound(String),                          // 插件未找到
    LoadFailed(String),                        // 插件加载失败
    InitializationFailed(String),              // 初始化失败
    MethodCallFailed { method: String, message: String },  // 方法调用失败
    StorageFailed(String),                     // 数据存储失败
    SerializationFailed(String),               // 序列化/反序列化失败
    InvalidParameter(String),                  // 参数错误
    MethodNotImplemented(String),              // 方法未实现
    Other(String),                             // 其他错误
}
```

实现了以下 trait：
- `std::fmt::Display` -- 用户可读的错误描述
- `std::error::Error` -- 可被 `?` 操作符传播
- `From<String>` -- 从 String 自动转换为 `PluginError::Other`
- `From<&str>` -- 从 &str 自动转换为 `PluginError::Other`

#### PluginResult 类型别名

```rust
pub type PluginResult<T> = Result<T, PluginError>;
```

所有插件操作的标准返回类型。

#### 辅助宏

**method_error!** -- 创建方法调用失败错误：

```rust
// 简单消息
method_error!("get_password", "密码不存在")

// 格式化消息
method_error!("decrypt", "解密失败: error code {}", code)
```

展开为 `PluginError::MethodCallFailed { method, message }`。

**param_error!** -- 创建参数错误：

```rust
// 简单消息
param_error!("缺少 password_id 参数")

// 格式化消息
param_error!("无效的长度: {}", length)
```

展开为 `PluginError::InvalidParameter(msg)`。

### storage -- 数据存储

基于 JSON 文件的持久化存储，使用原子写入防止数据损坏。

#### PluginStorage

```rust
pub struct PluginStorage {
    plugin_id: String,
    data_filename: String,
}
```

##### 创建实例

```rust
let storage = PluginStorage::new("password-manager", "password-manager.json");
```

- `plugin_id` -- 插件唯一标识符，用于日志
- `data_filename` -- 数据文件名，建议 `<plugin-id>.json`

##### 存储路径

数据文件存储在 `~/.worktools/history/plugins/<data_filename>`。

如果主路径不可用，fallback 到系统数据目录：
- Windows: `C:\Users\<用户>\AppData\Local\worktools\data\`
- macOS: `/Users/<用户>/Library/Application Support/worktools/data/`
- Linux: `/home/<用户>/.local/share/worktools/data/`

##### 核心方法

**load_json\<T\>()**

```rust
let data: MyData = storage.load_json()?;
```

加载 JSON 数据并反序列化为 `T`。文件不存在时返回 `T::default()`。

约束：`T: for<'de> Deserialize<'de> + Default`

**save_json\<T\>(data: &T)**

```rust
storage.save_json(&data)?;
```

保存数据到 JSON 文件。使用原子写入：
1. 写入 `.tmp` 临时文件
2. `sync_all()` 刷盘
3. `rename()` 原子替换原文件

约束：`T: Serialize`

**save_json_preserving\<T\>(data: &T, preserve_fields: &[&str])**

```rust
storage.save_json_preserving(&data, &["salt", "validation_token"])?;
```

保存数据但保留指定字段的现有值。适用于加密 salt 等不应被覆盖的字段。

实现原理：读取现有 JSON -> 用现有值覆盖指定字段 -> 原子写入。

##### 使用示例

```rust
use worktools_plugin_api::storage::PluginStorage;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Default)]
struct PasswordEntries {
    entries: Vec<PasswordEntry>,
}

#[derive(Serialize, Deserialize)]
struct PasswordEntry {
    id: String,
    title: String,
    encrypted_password: String,
}

// 创建存储
let storage = PluginStorage::new("password-manager", "password-manager.json");

// 加载数据（首次返回默认值）
let mut data: PasswordEntries = storage.load_json()?;

// 修改数据
data.entries.push(new_entry);

// 保存
storage.save_json(&data)?;
```

### utils -- 工具函数

#### escape_xml

```rust
pub fn escape_xml(s: &str) -> String
```

转义 XML/HTML 特殊字符：

| 字符 | 转义结果 |
|------|----------|
| `&` | `&amp;` |
| `<` | `&lt;` |
| `>` | `&gt;` |
| `"` | `&quot;` |
| `'` | `&apos;` |

使用场景：插件返回 HTML 内容时转义用户输入，防止 XSS。

## 顶层导出

`lib.rs` 通过 `pub use` 将常用类型导出到 crate 根，简化导入路径：

```rust
// 以下两种写法等价
use worktools_plugin_api::PluginError;
use worktools_plugin_api::error::PluginError;

use worktools_plugin_api::PluginResult<T>;
use worktools_plugin_api::error::PluginResult<T>;

use worktools_plugin_api::escape_xml;
use worktools_plugin_api::utils::escape_xml;
```

## Plugin Trait

所有插件必须实现的核心接口：

```rust
pub trait Plugin: Send + Sync {
    // 必须实现
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;
    fn icon(&self) -> &str;
    fn get_view(&self) -> String;

    // 可选覆盖（有默认实现）
    fn init(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> { Ok(()) }
    fn destroy(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> { Ok(()) }
    fn handle_call(&mut self, _method: &str, _params: Value) -> Result<Value, Box<dyn Error + Send + Sync>>;
    fn get_assets_path(&self) -> &str { "assets" }
}
```

### trait bound: Send + Sync

`Send + Sync` 约束确保插件实例可以安全地在 Tauri 的异步运行时中跨线程使用。

### PluginCreateFn

动态库导出的工厂函数类型：

```rust
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn Plugin>;
```

每个插件动态库必须导出：

```rust
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin = Box::new(MyPlugin::new());
    Box::into_raw(Box::new(plugin))
}
```

使用 `extern "C"` 调用约定确保动态库互操作兼容性。双层 `Box` 设计是为了将 fat pointer（数据指针 + vtable 指针）转为固定大小的裸指针，安全跨越 FFI 边界。

## 最小插件示例

```rust
use worktools_plugin_api::{Plugin, PluginError, PluginResult, PluginStorage};
use serde_json::Value;

pub struct MyPlugin;

impl MyPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for MyPlugin {
    fn id(&self) -> &str { "my-plugin" }
    fn name(&self) -> &str { "我的插件" }
    fn description(&self) -> &str { "一个示例插件" }
    fn version(&self) -> &str { "1.0.0" }
    fn icon(&self) -> &str { "📦" }
    fn get_view(&self) -> String {
        "<div id=\"app\">Loading...</div>".to_string()
    }

    fn handle_call(&mut self, method: &str, params: Value) -> PluginResult<Value> {
        match method {
            "get_data" => {
                let storage = PluginStorage::new(self.id(), "my-plugin.json");
                let data = storage.load_json::<serde_json::Value>()
                    .map_err(|e| PluginError::StorageFailed(e.to_string()))?;
                Ok(data)
            }
            "save_data" => {
                let storage = PluginStorage::new(self.id(), "my-plugin.json");
                storage.save_json(&params)
                    .map_err(|e| PluginError::StorageFailed(e.to_string()))?;
                Ok(serde_json::json!({"success": true}))
            }
            _ => Err(method_error!(method, "未知方法")),
        }
    }
}

#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    Box::into_raw(Box::new(Box::new(MyPlugin::new())))
}
```
