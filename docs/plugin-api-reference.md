# Plugin API Reference

Work Tools 插件 API 的完整参考文档。涵盖 `Plugin` trait、`PluginStorage`、`PluginError`、相关宏和类型定义。

## 目录

- [Plugin Trait](#plugin-trait)
- [PluginStorage](#pluginstorage)
- [PluginError](#pluginerror)
- [辅助宏](#辅助宏)
- [PluginInfo](#plugininfo)
- [PluginCreateFn](#plugincreatefn)
- [escape_xml](#escape_xml)
- [相关类型定义](#相关类型定义)

---

## Plugin Trait

所有插件必须实现的统一接口。定义于 `shared/plugin-api/src/lib.rs`。

```rust
pub trait Plugin: Send + Sync { ... }
```

Trait bound `Send + Sync` 要求插件实例可以安全地在线程间传递和共享，这是 Tauri 异步运行时的前提。

### 必须实现的方法

#### `id()`

```rust
fn id(&self) -> &str;
```

插件的唯一标识符。

- **返回**: 以 kebab-case 格式命名的字符串，如 `"password-manager"`、`"cron-tools"`。
- **用途**: 在插件注册表中作为 key，存储路径基于此 ID，前端 `pluginAPI.call()` 的第一个参数。
- **约束**: 全局唯一，不可与其他插件冲突。
- **示例**:

```rust
fn id(&self) -> &str {
    "hello-world"
}
```

#### `name()`

```rust
fn name(&self) -> &str;
```

插件的显示名称。

- **返回**: 中文字符串，在前端侧边栏中展示。
- **示例**:

```rust
fn name(&self) -> &str {
    "密码管理器"
}
```

#### `description()`

```rust
fn description(&self) -> &str;
```

插件的功能描述。

- **返回**: 插件功能的简要说明。
- **示例**:

```rust
fn description(&self) -> &str {
    "Cron表达式解析、人类可读描述、下次执行时间预览"
}
```

#### `version()`

```rust
fn version(&self) -> &str;
```

插件版本号。

- **返回**: 遵循语义化版本规范 (SemVer) 的字符串，如 `"1.0.0"`。
- **示例**:

```rust
fn version(&self) -> &str {
    "1.0.0"
}
```

#### `icon()`

```rust
fn icon(&self) -> &str;
```

插件图标。

- **返回**: 一个 emoji 字符或图标名称，在前端侧边栏中展示。
- **示例**:

```rust
fn icon(&self) -> &str {
    "⏱"
}
```

#### `get_view()`

```rust
fn get_view(&self) -> String;
```

获取插件 UI 的 HTML 内容。

- **返回**: HTML 字符串，会被嵌入到前端的 iframe 中展示。
- **说明**: 当插件有 `assets/` 目录时，通常返回占位 HTML。主程序通过 iframe srcdoc 机制加载 `assets/` 中的实际前端资源。
- **示例**:

```rust
fn get_view(&self) -> String {
    "<div>插件前端资源加载中...</div>".to_string()
}
```

### 可选覆盖的方法

以下方法有默认实现，插件可以按需覆盖。

#### `init()`

```rust
fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
```

插件初始化回调。在插件被加载时调用（`PluginManager::load_plugin` 中），用于执行启动时的设置工作。

- **参数**: `&mut self` -- 可变引用，允许在初始化时修改插件状态。
- **返回**: `Ok(())` 表示初始化成功，`Err(...)` 会导致插件加载失败。
- **默认行为**: 什么也不做，直接返回 `Ok(())`。
- **使用场景**: 建立网络连接、加载配置、初始化内部状态。
- **示例**:

```rust
fn init(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("插件 {} 初始化开始", self.id());
    // 执行初始化逻辑...
    tracing::info!("插件 {} 初始化完成", self.id());
    Ok(())
}
```

#### `destroy()`

```rust
fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    Ok(())
}
```

插件销毁回调。在插件被卸载时调用（`PluginManager::unload_plugin` 中），用于释放资源。

- **参数**: `&mut self` -- 可变引用。
- **返回**: `Ok(())` 表示清理成功。即使返回 `Err`，插件仍会被卸载（错误仅记录警告日志）。
- **默认行为**: 什么也不做。
- **使用场景**: 关闭网络连接、释放文件句柄、保存最终状态。
- **示例**:

```rust
fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing::info!("插件 {} 正在销毁", self.id());
    // 释放资源...
    Ok(())
}
```

#### `handle_call()`

```rust
fn handle_call(
    &mut self,
    _method: &str,
    _params: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    Err("method not implemented".into())
}
```

处理来自前端的方法调用。这是前后端通信的核心方法。

- **参数**:
  - `method: &str` -- 方法名，由前端调用时指定。如 `"greet"`、`"parse_cron"`。
  - `params: Value` -- JSON 格式的参数，类型为 `serde_json::Value`。
- **返回**:
  - `Ok(Value)` -- 成功时返回 JSON 值。
  - `Err(Box<dyn Error>)` -- 失败时返回错误。
- **默认行为**: 返回 `"method not implemented"` 错误。
- **说明**: 前端通过 `window.pluginAPI.call(pluginId, method, params)` 发起的请求，会经过 Tauri command 路由到此方法。
- **示例**:

```rust
fn handle_call(
    &mut self,
    method: &str,
    params: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    match method {
        "list_items" => {
            let data = Self::load_data()?;
            Ok(serde_json::to_value(data)?)
        }
        "save_item" => {
            let name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or("缺少 name 参数")?;
            // 处理保存逻辑...
            Ok(serde_json::json!({ "success": true }))
        }
        _ => Err(format!("未知方法: {method}").into()),
    }
}
```

**推荐模式**（来自 db-router 插件）：将各个方法的处理逻辑拆分到独立的辅助方法，然后在 `handle_call` 中通过 match 分发：

```rust
fn handle_call(
    &mut self,
    method: &str,
    params: Value,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let result = match method {
        "list_rules" => self.handle_list_rules(),
        "save_rule" => self.handle_save_rule(&params),
        "delete_rule" => self.handle_delete_rule(&params),
        "parse_route" => self.handle_parse_route(&params),
        _ => Err(anyhow::anyhow!("未知方法: {method}")),
    };
    result.map_err(|e| e.into())
}
```

#### `get_assets_path()`

```rust
fn get_assets_path(&self) -> &str {
    "assets"
}
```

获取插件前端资源路径（相对于插件目录）。

- **返回**: 资源目录名，默认为 `"assets"`。
- **说明**: 如果前端资源放在其他目录（如 `"dist"`），可以覆盖此方法。
- **示例**:

```rust
fn get_assets_path(&self) -> &str {
    "assets"  // 绝大多数插件使用默认值即可
}
```

---

## PluginStorage

基于 JSON 文件的持久化存储工具。定义于 `shared/plugin-api/src/storage.rs`。

```rust
pub struct PluginStorage {
    plugin_id: String,
    data_filename: String,
}
```

数据保存在 `~/.worktools/history/plugins/` 目录下，使用原子写入防止文件损坏。

### 构造方法

#### `new()`

```rust
pub fn new(plugin_id: &str, data_filename: &str) -> Self
```

创建新的存储实例。

- **参数**:
  - `plugin_id: &str` -- 插件 ID，用于日志标识。
  - `data_filename: &str` -- 数据文件名，建议使用 `<plugin-id>.json` 格式。
- **返回**: `PluginStorage` 实例。
- **示例**:

```rust
let storage = PluginStorage::new("hello-world", "hello-world.json");
```

### 路径方法

#### `get_data_path()`

```rust
pub fn get_data_path(&self) -> Result<PathBuf>
```

获取数据文件的完整路径。

- **返回**: `Result<PathBuf>` -- 数据文件的绝对路径。
- **路径**: `~/.worktools/history/plugins/<data_filename>`
- **行为**: 如果目录不存在会自动创建（`create_dir_all`）。
- **错误**: 找不到用户主目录时返回错误。
- **示例**:

```rust
let path = storage.get_data_path()?;
// /home/user/.worktools/history/plugins/hello-world.json
```

#### `get_alternative_data_path()`

```rust
pub fn get_alternative_data_path(&self) -> Result<PathBuf>
```

获取备选数据文件路径（系统数据目录）。当主路径不可用时作为 fallback。

- **返回**: `Result<PathBuf>` -- 备选路径。
- **路径**:
  - Windows: `C:\Users\<user>\AppData\Local\worktools\data\<data_filename>`
  - macOS: `/Users/<user>/Library/Application Support/worktools/data/<data_filename>`
  - Linux: `/home/<user>/.local/share/worktools/data/<data_filename>`
- **示例**:

```rust
let alt_path = storage.get_alternative_data_path()?;
```

### 读写方法

#### `load_json()`

```rust
pub fn load_json<T>(&self) -> Result<T>
where
    T: for<'de> serde::Deserialize<'de> + Default,
```

从文件加载 JSON 数据并反序列化。

- **泛型约束**: `T` 必须实现 `Deserialize` 和 `Default`。
- **返回**: `Result<T>` -- 反序列化后的数据。
- **行为**: 文件不存在时返回 `T::default()`（例如空 Vec、空字符串），不会报错。
- **示例**:

```rust
#[derive(Deserialize, Default)]
struct MyData {
    items: Vec<String>,
}

let data: MyData = storage.load_json()?;
// 如果文件不存在，data.items 为空 Vec
```

#### `save_json()`

```rust
pub fn save_json<T>(&self, data: &T) -> Result<()>
where
    T: serde::Serialize,
```

将数据序列化为 JSON 并写入文件（原子写入）。

- **泛型约束**: `T` 必须实现 `Serialize`。
- **原子写入流程**:
  1. 数据写入 `.tmp` 临时文件。
  2. 调用 `sync_all()` 确保数据刷到磁盘。
  3. `rename()` 原子性地替换原文件。
- **格式**: 使用 `to_writer_pretty` 输出带缩进的 JSON，方便人工查看。
- **示例**:

```rust
let data = MyData {
    items: vec!["hello".to_string(), "world".to_string()],
};
storage.save_json(&data)?;
```

#### `save_json_preserving()`

```rust
pub fn save_json_preserving<T>(&self, data: &T, preserve_fields: &[&str]) -> Result<()>
where
    T: serde::Serialize,
```

保存 JSON 数据并保留指定字段不被覆盖。

- **参数**:
  - `data: &T` -- 要保存的数据。
  - `preserve_fields: &[&str]` -- 需要从现有文件中保留的字段名列表。
- **行为**:
  1. 读取现有 JSON 文件（如果存在）。
  2. 将新数据转为 JSON Value。
  3. 用现有数据覆盖 `preserve_fields` 中指定的字段。
  4. 写入合并后的结果。
- **使用场景**: 保护不应被覆盖的字段，如加密 salt、validation token。
- **示例**:

```rust
// 保存密码数据时，保留 salt 和 validation_token 不被覆盖
storage.save_json_preserving(&password_data, &["salt", "validation_token"])?;
```

---

## PluginError

插件系统的统一错误类型。定义于 `shared/plugin-api/src/error.rs`。

```rust
#[derive(Debug)]
pub enum PluginError {
    NotFound(String),
    LoadFailed(String),
    InitializationFailed(String),
    MethodCallFailed { method: String, message: String },
    StorageFailed(String),
    SerializationFailed(String),
    InvalidParameter(String),
    MethodNotImplemented(String),
    Other(String),
}
```

### 变体说明

| 变体 | 字段 | Display 输出 | 用途 |
|------|------|-------------|------|
| `NotFound(String)` | `id` | `"插件未找到: {id}"` | 插件不存在 |
| `LoadFailed(String)` | `msg` | `"插件加载失败: {msg}"` | 动态库加载失败 |
| `InitializationFailed(String)` | `msg` | `"插件初始化失败: {msg}"` | init() 返回错误 |
| `MethodCallFailed` | `method`, `message` | `"插件方法调用失败 [{method}]: {message}"` | handle_call() 执行出错 |
| `StorageFailed(String)` | `msg` | `"数据存储失败: {msg}"` | 文件读写失败 |
| `SerializationFailed(String)` | `msg` | `"序列化失败: {msg}"` | JSON 序列化/反序列化失败 |
| `InvalidParameter(String)` | `msg` | `"参数错误: {msg}"` | 参数缺失或类型错误 |
| `MethodNotImplemented(String)` | `method` | `"方法未实现: {method}"` | 调用了未实现的方法 |
| `Other(String)` | `msg` | `"插件错误: {msg}"` | 其他未分类错误 |

### Trait 实现

- `impl fmt::Display` -- 提供中文用户可读的错误描述。
- `impl std::error::Error` -- 可以被 `?` 操作符传播，可被 `anyhow::Error` 包装。
- `impl From<String> for PluginError` -- `String` 自动转换为 `PluginError::Other`。
- `impl From<&str> for PluginError` -- `&str` 自动转换为 `PluginError::Other`。

### PluginResult

```rust
pub type PluginResult<T> = Result<T, PluginError>;
```

类型别名，简化返回类型声明。

```rust
fn do_something() -> PluginResult<String> {
    Ok("success".to_string())
}
```

---

## 辅助宏

### `method_error!`

创建 `PluginError::MethodCallFailed` 错误。

```rust
#[macro_export]
macro_rules! method_error {
    ($method:expr, $msg:expr) => { ... };
    ($method:expr, $fmt:expr, $($arg:tt)*) => { ... };
}
```

**用法**:

```rust
use worktools_plugin_api::method_error;

// 简单字符串
return Err(method_error!("parse_cron", "表达式格式错误").into());

// 格式化字符串
return Err(method_error!("save_data", "保存失败: {}", err).into());
```

### `param_error!`

创建 `PluginError::InvalidParameter` 错误。

```rust
#[macro_export]
macro_rules! param_error {
    ($msg:expr) => { ... };
    ($fmt:expr, $($arg:tt)*) => { ... };
}
```

**用法**:

```rust
use worktools_plugin_api::param_error;

// 简单字符串
return Err(param_error!("缺少 name 参数").into());

// 格式化字符串
return Err(param_error!("无效的类型: {}", type_name).into());
```

---

## PluginInfo

插件的元信息结构体，用于向前端暴露插件基本信息。定义于 `shared/types/src/lib.rs`。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
}
```

### 字段说明

| 字段 | 类型 | 来源 | 说明 |
|------|------|------|------|
| `id` | `String` | `Plugin::id()` | 唯一标识符，kebab-case |
| `name` | `String` | `Plugin::name()` | 显示名称 |
| `version` | `String` | `Plugin::version()` | 版本号 |
| `description` | `String` | `Plugin::description()` | 功能描述 |
| `icon` | `String` | `Plugin::icon()` | 图标 emoji |

### 构建过程

`PluginManager::load_plugin()` 在加载插件时，从 Plugin trait 方法中提取信息构建 `PluginInfo`：

```rust
let info = PluginInfo {
    id: plugin.id().to_string(),
    name: plugin.name().to_string(),
    description: plugin.description().to_string(),
    version: plugin.version().to_string(),
    icon: plugin.icon().to_string(),
};
```

`PluginInfo` 通过 Tauri command 序列化为 JSON 传给前端。

---

## PluginCreateFn

插件工厂函数的类型定义。

```rust
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn Plugin>;
```

- **ABI**: `extern "C"` -- C 调用约定，动态库互操作的标准。
- **安全性**: `unsafe` -- 跨 FFI 边界的调用。
- **返回**: `*mut Box<dyn Plugin>` -- 原始指针，指向堆上的 trait object。

### 导出函数

每个插件必须导出一个名为 `plugin_create` 的函数，签名匹配 `PluginCreateFn`：

```rust
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin: Box<Box<dyn Plugin>> = Box::new(Box::new(HelloWorldPlugin));
    Box::leak(plugin) as *mut Box<dyn Plugin>
}
```

要点：
- `#[no_mangle]` -- 保持符号名为 `plugin_create`，主程序通过此名称查找。
- 两层 `Box`: 外层提供固定大小的指针以跨越 FFI 边界，内层存储实际的插件实例。
- `Box::leak()` -- 泄漏堆内存，返回原始指针。主程序侧通过 `Box::from_raw()` 恢复所有权。

---

## escape_xml

XML/HTML 特殊字符转义工具函数。定义于 `shared/plugin-api/src/utils.rs`。

```rust
pub fn escape_xml(s: &str) -> String
```

转义 `&`、`<`、`>`、`"`、`'` 为对应的 XML entity。

- **参数**: `s: &str` -- 原始字符串。
- **返回**: `String` -- 转义后的字符串。
- **示例**:

```rust
use worktools_plugin_api::escape_xml;

let escaped = escape_xml("<script>alert('xss')</script>");
// &lt;script&gt;alert(&apos;xss&apos;)&lt;/script&gt;
```

---

## 相关类型定义

### 动态库加载流程

主程序（`PluginManager`）加载插件的关键步骤：

```
1. Library::new(lib_path)
   -> 加载 .dylib/.so/.dll 文件

2. library.get::<PluginCreateFn>(b"plugin_create")
   -> 查找导出函数符号

3. create()
   -> 调用 plugin_create()，获得 *mut Box<dyn Plugin>

4. Box::from_raw(plugin_ptr)
   -> 从原始指针恢复 Rust 所有权

5. plugin.init()
   -> 调用插件初始化方法

6. 存入 HashMap<String, LoadedPlugin>
   -> 注册到插件管理器
```

### LoadedPlugin 结构

```rust
pub struct LoadedPlugin {
    pub info: PluginInfo,
    pub instance: Box<dyn Plugin>,
    _library: Library,  // RAII guard，drop 时卸载动态库
}
```

### 方法调用流程

```
前端 iframe
  -> window.pluginAPI.call("plugin-id", "method", params)
  -> Tauri command: call_plugin_method
  -> PluginManager::call_plugin_method()
  -> plugins.write().await  (获取写锁)
  -> plugin.instance.handle_call(method, params)
  -> 返回 Result<Value>
  -> 序列化为 JSON 传回前端
```

### 依赖关系

```
worktools-shared-types  (PluginInfo)
         ^
         |
worktools-plugin-api    (Plugin, PluginStorage, PluginError)
         ^
         |
各插件 crate             (实现 Plugin trait)
         ^
         |
tauri-app/src-tauri      (PluginManager, 通过 libloading 加载)
```

`worktools-shared-types` 是独立 crate，主程序和插件都可以依赖，避免循环依赖。
