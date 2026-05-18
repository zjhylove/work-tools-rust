# 架构设计 (Architecture)

Work Tools Platform 的技术架构文档。本文档面向开发者，详细描述各子系统的设计和实现。

## 1. 项目概述

Work Tools Platform 是一个基于 **Tauri 2.x + Rust** 的可扩展桌面工具平台，采用**动态库插件架构**。主程序通过 `libloading` 在同进程内加载编译为 `cdylib` 的插件，实现热插拔和独立编译。

**核心技术栈**:
- 后端: Rust 1.70+, Tauri 2.x, libloading, tokio (异步运行时), tracing (结构化日志)
- 前端: React 19 + TypeScript + Vite 6
- 插件: 每个 Plugin 同时包含 Rust 后端 (cdylib) 和 React 前端 (iframe srcdoc)
- 数据存储: JSON 文件（原子写入）

**设计哲学**:
- 插件独立编译，不需要重新编译主程序
- 同进程加载（非多进程），通过 Rust 的类型安全降低风险
- 插件间完全隔离，各自管理数据和前端 UI

## 2. 目录结构

```
work-tools-rust/
├── tauri-app/                    # Tauri 主应用
│   ├── src/                      # React 前端
│   │   ├── App.tsx               # 应用入口，主题管理，插件路由
│   │   ├── components/           # UI 组件
│   │   └── styles/
│   │       └── tokens.css        # 全局设计令牌 (浅色 + 暗色)
│   └── src-tauri/src/            # Rust 后端
│       ├── lib.rs                # 应用初始化、Tauri builder 配置
│       ├── commands.rs           # 21 个 Tauri command (IPC 接口)
│       ├── plugin_manager.rs     # 动态库加载、插件生命周期
│       ├── plugin_package.rs     # .wtplugin.zip 解析、验证、安装
│       ├── plugin_registry.rs    # 插件注册表 (持久化元数据)
│       ├── logger.rs             # 日志系统 (tracing 三层架构)
│       ├── config.rs             # 插件配置持久化
│       ├── paths.rs              # 工作目录路径管理
│       └── tray.rs               # 系统托盘
├── plugins/                      # 13 个插件 (各有独立 frontend/)
│   ├── password-manager/         # 密码管理器 (AES 加密)
│   ├── json-tools/               # JSON 工具
│   ├── auth-plugin/              # 双因素验证 (TOTP)
│   ├── text-diff/                # 文本比对 (Monaco Editor)
│   ├── db-doc/                   # 数据库文档生成 (MySQL/PostgreSQL)
│   ├── k8s-forward/              # K8s 端口转发 (SSH 隧道 + HTTP 代理)
│   ├── db-router/                # 数据库路由 (Rhai 脚本解析)
│   ├── object-storage/           # 对象存储 (阿里云 OSS + 腾讯云 COS)
│   ├── timestamp-converter/      # Unix 时间戳转换 (多时区/批量)
│   ├── cron-tools/               # Cron 表达式解析/可视化
│   ├── redis-client/             # Redis 客户端 (Key/多类型操作)
│   ├── api-doc/                  # API 文档生成 (Spring Boot JAR 解析)
│   └── ...                       # 更多插件
├── shared/
│   ├── types/                    # 共享数据类型 (PluginInfo 等)
│   └── plugin-api/               # Plugin trait + PluginStorage + 错误处理
│       └── src/
│           ├── lib.rs            # Plugin trait 定义
│           ├── storage.rs        # PluginStorage (JSON 文件持久化)
│           ├── error.rs          # PluginError + method_error! + param_error!
│           └── utils.rs          # 工具函数
└── scripts/                      # 构建/打包脚本
    ├── build-plugins.sh          # 一键编译+打包所有插件 (macOS/Linux)
    └── build-plugins.ps1         # Windows PowerShell 版本
```

### Workspace 结构

根 `Cargo.toml` 使用 `exclude = ["tauri-app"]` 但同时将 `tauri-app/src-tauri` 列为 workspace member。

- `tauri-app/` 被 exclude 是因为它包含前端项目 (package.json, node_modules)，不属于 Cargo workspace 的管理范围
- `tauri-app/src-tauri/` 作为 Rust crate 仍然是 workspace member
- `cargo test` 在根目录运行时测试所有 workspace members（2 shared + 13 plugins）
- `cargo check/build` 只编译 workspace members（不包含 tauri-app 前端）

## 3. 插件加载机制

插件通过 `libloading` crate 在运行时加载动态库，整个流程由 `PluginManager` (`tauri-app/src-tauri/src/plugin_manager.rs`) 管理。

### 3.1 动态库加载流程

```
PluginManager::init()
  ├── 扫描 ~/.worktools/plugins/ 下每个子目录
  ├── 读取 manifest.json 获取动态库文件名
  │   └── 根据平台选择: macos/linux/windows 字段
  ├── Library::new(lib_path)  // 加载 .dylib/.so/.dll
  ├── library.get(b"plugin_create")  // 查找工厂函数符号
  ├── create()  // 调用工厂函数，获得 *mut Box<dyn Plugin>
  ├── Box::from_raw(plugin_ptr)  // 从原始指针恢复 Rust 所有权
  ├── plugin.init()  // 调用插件初始化方法
  └── 保存到 HashMap<String, LoadedPlugin>
```

### 3.2 核心数据结构

```rust
// 已加载的插件实例
pub struct LoadedPlugin {
    pub info: PluginInfo,          // 插件元数据（给前端展示用）
    pub instance: Box<dyn Plugin>, // 插件实例（trait object）
    _library: Library,             // 动态库句柄（RAII，drop 时自动卸载）
}

// 插件管理器
pub struct PluginManager {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,  // plugin_id → LoadedPlugin
    plugin_dir: PathBuf,                              // ~/.worktools/plugins/
}
```

**RwLock 的选择**: 使用 `tokio::sync::RwLock` 而非 `Mutex`，因为"列出插件"（读操作）比"修改插件列表"（写操作）频繁得多，RwLock 允许多个读操作并发。

### 3.3 Plugin trait

定义在 `shared/plugin-api/src/lib.rs`：

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
    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn Error + Send + Sync>> { ... }
    fn get_assets_path(&self) -> &str { "assets" }
}
```

**Send + Sync bound**: 插件实例需要在 Tauri 的异步运行时中跨线程使用，因此必须实现 `Send + Sync`。

### 3.4 工厂函数

每个插件必须导出一个 C ABI 的工厂函数：

```rust
#[no_mangle]
pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
    let plugin = Box::new(MyPlugin::new());
    Box::into_raw(Box::new(plugin))
}
```

类型定义：

```rust
pub type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn Plugin>;
```

两层 Box 的原因：外层 Box 提供固定大小的 fat pointer（数据指针 + vtable 指针），内层 Box 存储大小未知的 `dyn Plugin` 数据。这样 `*mut Box<dyn Plugin>` 是已知大小的指针，可以安全跨越 FFI 边界。

### 3.5 插件生命周期

```
加载 (load_plugin)
  → Library::new()           // dlopen
  → library.get("plugin_create")  // dlsym
  → create()                 // 调用工厂函数
  → Box::from_raw()          // 恢复所有权
  → plugin.init()            // 初始化
  → 保存到 HashMap

运行时 (handle_call)
  → plugin.handle_call(method, params)  // 前端方法调用
  → 返回 JSON 结果

卸载 (unload_plugin)
  → plugin.destroy()         // 清理
  → plugins.remove()         // 从 HashMap 移除
  → LoadedPlugin drop        // 自动触发
    → instance drop           // 释放插件实例
    → _library drop           // dlclose，释放动态库
```

## 4. 插件渲染机制

插件前端通过 **iframe srcdoc** 方式渲染，实现样式和脚本的完全隔离。

### 4.1 渲染流程

1. `PluginPlaceholder` 组件读取已安装插件的三个前端文件：
   - `assets/index.html`
   - `assets/main.js`
   - `assets/styles.css`
2. 将这三个文件的内容内联拼接为一个完整的 HTML 字符串
3. 注入到 `<iframe srcdoc="...">` 中
4. iframe 加载完成后，通过 JavaScript 注入 `window.pluginAPI` 对象

### 4.2 pluginAPI 接口

iframe 内的插件前端通过 `window.pluginAPI` 与 Rust 后端通信：

```typescript
interface PluginAPI {
  call(method: string, params?: object): Promise<any>;
  get_plugin_config(): Promise<object>;
  set_plugin_config(config: object): Promise<void>;
  open_url(url: string): Promise<void>;
  open_folder_dialog(title?: string): Promise<string | null>;
  open_file_dialog(title?: string, filters?: object): Promise<string | null>;
  write_file(path: string, content: string): Promise<void>;
}
```

### 4.3 主题传递

- 插件 iframe 通过 `INJECTED_TOKENS` 接收完整的 CSS 变量定义（包含 `:root` 浅色和 `[data-theme="dark"]` 暗色两套）
- 注入顺序：先插入插件的 `styles.css`，再注入令牌样式，确保令牌优先级最高
- 主题切换时，主窗口通过 `postMessage({ type: "theme", theme })` 通知所有已打开的 iframe 实时更新

## 5. 数据流

前端调用到后端的完整数据通路：

```
前端 iframe (React)
  → window.pluginAPI.call("password-manager", "list_passwords", {})
  → Tauri IPC (invoke)
  → #[tauri::command] call_plugin_method(plugin_id, method, params)
  → PluginManager::call_plugin_method("password-manager", "list_passwords", {})
  → plugins.write().await  // 获取写锁（因为 handle_call 需要 &mut self）
  → plugin.instance.handle_call("list_passwords", {})
  → 返回 serde_json::Value
  → 前端接收 JSON 结果
```

### Tauri Commands (21 个)

定义在 `commands.rs`，按功能分组：

| 分组 | 命令 | 说明 |
|------|------|------|
| 插件核心 | `get_installed_plugins` | 获取已加载插件列表 |
| | `call_plugin_method` | 调用插件方法（核心 API） |
| | `read_plugin_asset` | 读取插件前端资源文件 |
| 配置 | `get_plugin_config` | 读取插件配置 |
| | `set_plugin_config` | 保存插件配置 |
| 插件商店 | `import_plugin_package` | 导入 .wtplugin.zip |
| | `get_available_plugins` | 扫描可用插件 |
| | `get_installed_plugins_from_registry` | 从注册表读取已安装插件 |
| | `install_plugin` | 安装插件 |
| | `uninstall_plugin` | 卸载插件 |
| 系统交互 | `open_url` | 打开外部链接 |
| | `open_folder_dialog` | 文件夹选择对话框 |
| | `open_file_dialog` | 文件选择对话框 |
| | `write_file` | 写入文件 |
| 日志 | `get_logs` | 查询日志（支持 level/plugin/since 过滤） |
| | `clear_logs` | 清空日志缓冲 |
| 主题 | `set_window_theme` | 设置窗口主题（同步原生标题栏） |

## 6. 日志系统

日志系统定义在 `logger.rs`，采用 `tracing_subscriber::registry()` 三层架构：

### 6.1 三层结构

| 层 | 类型 | 输出目标 | 特性 |
|---|------|---------|------|
| 层 1 | `fmt::layer` (stdout) | 控制台 | ANSI 颜色，不显示 target |
| 层 2 | `fmt::layer` (non_blocking_file) | `~/.worktools/logs/` | 按天滚动，无颜色，显示 target |
| 层 3 | `LogRingLayer` (自定义) | `LOG_RING` 内存缓冲 | 最多 1000 条，环形覆盖 |

### 6.2 环形缓冲区

```rust
pub static LOG_RING: Mutex<VecDeque<LogEntry>> = Mutex::new(VecDeque::new());
```

- 容量 1000 条，达到上限时移除最旧条目（`pop_front` + `push_back`）
- 使用 `VecDeque` 而非 `Vec`，因为两端操作都是 O(1)
- 前端通过 `get_logs` command 查询，支持按 level / plugin / since 过滤
- 查询使用 `iter().rev().filter().take(100).cloned().collect()` 避免克隆全部数据

### 6.3 LogEntry 结构

```rust
pub struct LogEntry {
    pub timestamp: String,  // RFC 3339, 毫秒精度
    pub level: String,      // TRACE, DEBUG, INFO, WARN, ERROR
    pub target: String,     // 来源模块，如 "work_tools::plugin_manager"
    pub message: String,    // 结构化字段拼接后的消息
}
```

### 6.4 日志过滤

```rust
tracing_subscriber::filter::Targets::new()
    .with_default(tracing::Level::DEBUG)           // 默认 DEBUG 级别
    .with_target("winit", tracing::Level::ERROR)   // 抑制 winit 噪音
    .with_target("tao", tracing::Level::ERROR)     // 抑制 tao 噪音
```

所有 13 个插件均接入 tracing，关键操作记录 `info!` / `warn!` / `error!` 日志。

### 6.5 文件日志的内存安全

文件日志使用 `tracing_appender::non_blocking` 实现非阻塞写入，返回的 `WorkerGuard` 通过 `Box::leak` 故意泄漏，确保后台写入线程在程序整个生命周期内持续运行。

## 7. 主题系统

支持浅色/暗色双主题，通过 CSS 变量 + `data-theme` 属性驱动。

### 7.1 设计令牌

定义在 `tauri-app/src/styles/tokens.css`：

- `:root` -- 浅色主题的变量定义
- `[data-theme="dark"]` -- 暗色主题的变量覆盖

### 7.2 主题管理

- `App.tsx` 管理 `theme` state，持久化到 `localStorage`
- 通过设置 `<html data-theme="light|dark">` 切换主题
- 侧边栏底部有 moon/sun 图标按钮用于切换

### 7.3 原生窗口主题同步

`set_window_theme` 命令 (`commands.rs`) 通过 Tauri 的 `set_theme()` API 同步原生窗口标题栏主题：
- Windows 10 1809+ 支持原生暗色标题栏
- macOS 在透明标题栏模式下，通过 `set_background_color()` 使标题栏区域与内容区颜色匹配

### 7.4 插件主题支持

1. 主窗口将完整的 CSS 变量（包含浅色和暗色两套）通过 `INJECTED_TOKENS` 注入 iframe
2. 注入顺序：插件 `styles.css` -> 设计令牌样式，确保令牌优先级最高
3. 主题切换时，通过 `postMessage({ type: "theme", theme })` 实时通知所有 iframe
4. 插件 CSS **必须使用** `var(--xxx)` 令牌，禁止硬编码颜色值

## 8. 数据存储

### 8.1 应用目录结构

由 `paths.rs` 管理，统一位于 `~/.worktools/`：

```
~/.worktools/
├── plugins/                    # 已安装插件
│   └── <plugin-id>/
│       ├── manifest.json       # 插件元数据
│       ├── lib<name>.dylib     # 动态库
│       └── assets/             # 前端资源
│           ├── index.html
│           ├── main.js
│           └── styles.css
├── config/
│   └── installed-plugins.json  # 插件注册表
├── logs/
│   └── work-tools.log.YYYY-MM-DD  # 按天滚动日志
└── history/
    └── plugins/
        └── <plugin-id>.json    # 插件持久化数据
```

### 8.2 PluginStorage (插件数据存储)

定义在 `shared/plugin-api/src/storage.rs`，提供基于 JSON 文件的持久化存储：

```rust
let storage = PluginStorage::new("password-manager", "password-manager.json");
let data: Vec<Password> = storage.load_json()?;  // 文件不存在时返回 Default
storage.save_json(&data)?;                       // 原子写入
```

**原子写入机制**:
1. 将数据序列化为 JSON 并写入 `.tmp` 临时文件
2. 调用 `sync_all()` 确保数据刷到磁盘
3. 用 `rename()` 原子性地替换原文件

这保证了即使在写入过程中程序崩溃，也不会导致文件损坏（要么是旧文件，要么是新文件，不存在中间状态）。

### 8.3 插件配置 (config.rs)

每个插件的配置存储在 `~/.worktools/history/plugins/<plugin-id>.json`，通过 `get_plugin_config` / `set_plugin_config` 命令读写。配置使用 `serde_json::Value` 类型，结构不固定。

### 8.4 插件注册表 (plugin_registry.rs)

`PluginRegistry` 管理已安装插件的持久化元数据，存储在 `~/.worktools/config/installed-plugins.json`。

```rust
pub struct InstalledPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub icon: Option<String>,
    pub author: Option<String>,
    pub homepage: Option<String>,
    pub installed_at: DateTime<Utc>,
    pub enabled: bool,
    pub assets_path: PathBuf,    // 前端资源目录的绝对路径
    pub library_path: PathBuf,   // 动态库文件的绝对路径
}
```

**与 PluginManager 的区别**:
- `PluginRegistry`: 持久化元数据（JSON 文件），记录"哪些插件已安装"
- `PluginManager`: 运行时实例管理（内存），管理"哪些插件已加载"

## 9. 插件包格式

### 9.1 .wtplugin.zip 结构

```
plugin.zip
├── manifest.json                # 插件元数据（必需）
├── libplugin.dylib/.so/.dll     # 动态库（按平台）
└── assets/                      # 前端资源
    ├── index.html               # 入口 HTML
    ├── main.js                  # 脚本
    └── styles.css               # 样式
```

### 9.2 manifest.json 格式

```json
{
  "id": "password-manager",
  "name": "密码管理器",
  "description": "安全地管理你的密码",
  "version": "1.0.0",
  "icon": "🔐",
  "author": "WorkTools Team",
  "homepage": "https://github.com/example",
  "minAppVersion": "1.0.0",
  "license": "MIT",
  "files": {
    "macos": "libpassword_manager.dylib",
    "linux": "libpassword_manager.so",
    "windows": "password_manager.dll"
  },
  "assets": {
    "entry": "index.html"
  },
  "permissions": [],
  "screenshots": []
}
```

### 9.3 安装流程

定义在 `commands.rs` 的 `import_plugin_package` 命令：

1. 从 ZIP 文件加载插件包 (`PluginPackage::from_zip`)
2. 验证插件包完整性 (`pkg.validate()`)：
   - ID 非空，且只包含小写字母、数字、连字符
   - 当前平台的动态库文件已配置
   - ZIP 中包含 manifest.json、动态库、前端入口文件
3. 解压到 `~/.worktools/plugins/<plugin-id>/` (`pkg.install`)
4. 注册到插件注册表 (`registry.register`)
5. 重新加载插件管理器 (`manager.init`)

### 9.4 卸载流程

卸载必须按特定顺序执行（特别是 Windows 上 DLL 会被文件锁锁定）：

1. 从内存中卸载插件 (`manager.unload_plugin`) -- 释放 DLL 文件锁
2. 删除插件目录文件 -- 带重试机制（最多 3 次，递增延迟 200ms * attempt）
3. 从注册表移除 (`registry.unregister`)

### 9.5 平台适配

动态库文件名根据平台不同：

| 平台 | 前缀 | 扩展名 | 示例 |
|------|------|--------|------|
| macOS | `lib` | `.dylib` | `libpassword_manager.dylib` |
| Linux | `lib` | `.so` | `libpassword_manager.so` |
| Windows | (无) | `.dll` | `password_manager.dll` |

插件目录名中的连字符在动态库名中转为下划线（如 `password-manager` -> `libpassword_manager.dylib`）。

## 10. 应用启动流程

定义在 `lib.rs` 的 `run()` 函数：

```
run()
  → init_logging()              // 初始化三层日志系统
  → PluginManager::new()        // 创建插件管理器（确定插件目录路径）
  → tauri::Builder::default()
      .plugin(opener, shell, dialog, fs)  // 注册 Tauri 插件
      .setup(|app| {
          app.manage(plugin_manager)      // 注入到 Tauri 状态管理
          tray::start_tray(app)           // 初始化系统托盘
          tokio::spawn(async {
              manager.init().await        // 异步加载所有插件
              emit("plugins-ready")       // 通知前端插件已就绪
              sleep(300ms)                // 短暂延迟避免白屏闪烁
              window.show()              // 显示主窗口
          })
      })
      .invoke_handler(...)               // 注册 21 个 Tauri command
      .run(tauri_build_context)          // 启动应用
```

启动时窗口先隐藏，等插件加载完成并短暂延迟后再显示，避免白屏闪烁。
