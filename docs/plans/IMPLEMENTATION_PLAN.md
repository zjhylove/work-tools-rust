# 动态库插件架构实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**目标:** 将插件从独立进程 IPC 架构改造为同进程动态库架构,实现与 Java 版本一致的用户体验和开发模式。

**架构:**
- 插件编译为动态库 (.dylib/.so/.dll)
- 主程序通过 libloading 动态加载插件
- 插件通过 WebView 渲染 HTML UI
- 插件实现统一的 Plugin trait

**技术栈:**
- libloading (动态库加载)
- serde (序列化)
- Tauri WebView (UI 渲染)
- Solid.js (前端框架)


## 架构对比

### 当前架构 (IPC)
```
主进程 (Tauri)          插件进程 (独立可执行文件)
┌─────────────┐        ┌─────────────────────┐
│ PluginMgr   │───────▶│ password-manager    │
│ JSON-RPC    │ IPC    │ get_view() → {}     │
└─────────────┘        └─────────────────────┘
       │
       ▼
┌─────────────┐
│ App.tsx     │ 硬编码组件
│ <Password   │
│  Manager/>  │
└─────────────┘
```

### 目标架构 (动态库)
```
主进程 (Tauri)
┌─────────────────────────────────────────┐
│ PluginManager                            │
│  1. 扫描 ~/.worktools/plugins/          │
│  2. libloading::Library::load("plugin") │
│  3. 获取 plugin_create 函数指针          │
│         │                                │
│         ▼                                │
│  Plugin Trait (Box<dyn Plugin>)         │
│  - id(), name(), get_view()             │
│  - init(), destroy()                    │
└─────────────────────────────────────────┘
         │
         ▼
┌─────────────────────────────────────────┐
│ App.tsx (动态渲染)                       │
│  <PluginView                             │
│    html={plugin.get_view()}              │
│  />                                      │
└─────────────────────────────────────────┘
```

## 核心变化

| 组件 | 当前 | 目标 |
|------|------|------|
| 插件形式 | 独立可执行文件 | 动态库 (.dylib/.so/.dll) |
| 通信方式 | JSON-RPC over stdin/stdout | 直接函数调用 (同进程) |
| UI 渲染 | 硬编码 Solid.js 组件 | 动态渲染 HTML |
| 生命周期 | get_info, get_view | init, get_view, destroy |


## API 设计

### Plugin Trait (shared/plugin-api/src/lib.rs)

```rust
use serde_json::Value;

pub trait Plugin {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;
    fn icon(&self) -> &str;
    fn get_view(&self) -> String;
    
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    fn destroy(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    
    fn handle_call(&mut self, method: &str, params: Value) 
        -> Result<Value, Box<dyn std::error::Error>> {
        Err("method not implemented".into())
    }
}

pub type PluginCreateFn = unsafe extern "C" fn() -> *mut Box<dyn Plugin>;
```

### Tauri Commands

```rust
#[tauri::command]
async fn get_installed_plugins(
    manager: State<'_, PluginManager>,
) -> Result<Vec<PluginInfo>, String>;

#[tauri::command]
async fn get_plugin_view(
    plugin_id: String,
    manager: State<'_, PluginManager>,
) -> Result<String, String>;

#[tauri::command]
async fn call_plugin_method(
    plugin_id: String,
    method: String,
    params: Value,
    manager: State<'_, PluginManager>,
) -> Result<Value, String>;
```


## 实施任务

### Phase 1: 基础设施 (2-3小时)

#### Task 1.1: 创建 plugin-api 共享库

1. 创建 `shared/plugin-api/Cargo.toml`
2. 创建 `shared/plugin-api/src/lib.rs` (定义 Plugin trait)
3. 更新 `shared/Cargo.toml` 添加 workspace 成员
4. 编译测试: `cargo build -p worktools-plugin-api`
5. 提交: `feat: 创建共享插件 API 库`

#### Task 1.2: 添加依赖

修改 `tauri-app/src-tauri/Cargo.toml`:
```toml
[dependencies]
libloading = "0.8"
worktools-plugin-api = { path = "../../../shared/plugin-api" }
```

提交: `build: 添加 libloading 依赖`

### Phase 2: 插件管理器重构 (4-5小时)

#### Task 2.1: 重写 PluginManager

核心代码 (tauri-app/src-tauri/src/plugin_manager.rs):

```rust
use libloading::{Library, Symbol};
use worktools_plugin_api::{Plugin, PluginCreateFn};

pub struct LoadedPlugin {
    pub info: PluginInfo,
    pub instance: Box<dyn Plugin>,
    _library: Library,
}

pub struct PluginManager {
    plugins: RwLock<HashMap<String, LoadedPlugin>>,
    plugin_dir: PathBuf,
}

impl PluginManager {
    pub async fn init(&self) -> Result<()> {
        // 扫描 ~/.worktools/plugins/
        for entry in std::fs::read_dir(&self.plugin_dir)? {
            let lib_path = entry.path();
            unsafe {
                let library = Library::new(&lib_path)?;
                let create: Symbol<PluginCreateFn> = library.get(b"plugin_create")?;
                let plugin_ptr = create();
                let mut plugin = Box::from_raw(plugin_ptr);
                plugin.init()?;
                
                let info = PluginInfo {
                    id: plugin.id().to_string(),
                    name: plugin.name().to_string(),
                    // ...
                };
                
                self.plugins.write().await.insert(info.id.clone(), LoadedPlugin {
                    info,
                    instance: *plugin,
                    _library: library,
                });
            }
        }
        Ok(())
    }
}
```

提交: `refactor: 重写 PluginManager 支持动态库`

#### Task 2.2: 添加 get_plugin_view Command

修改 `tauri-app/src-tauri/src/commands.rs`:
```rust
#[tauri::command]
async fn get_plugin_view(
    plugin_id: String,
    manager: State<'_, PluginManager>,
) -> Result<String, String> {
    manager.get_plugin_view(&plugin_id).await.map_err(|e| e.to_string())
}
```

提交: `feat: 添加 get_plugin_view Command`

### Phase 3: 前端动态渲染 (3-4小时)

#### Task 3.1: 创建 PluginView 组件

创建 `tauri-app/src/components/PluginView.tsx`:

```tsx
import { createSignal, onMount, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";

export default (props: { pluginId: string }) => {
  const [html, setHtml] = createSignal<string>("");
  const [loading, setLoading] = createSignal(true);
  
  onMount(async () => {
    const viewHtml = await invoke<string>("get_plugin_view", {
      pluginId: props.pluginId,
    });
    setHtml(viewHtml);
    setLoading(false);
  });
  
  return (
    <Show when={loading()}>
      <div>加载中...</div>
    </Show>
    <Show when={!loading() && html()}>
      <div innerHTML={html()} />
    </Show>
  );
};
```

更新 `App.tsx`:
```tsx
<Show when={selectedPlugin()}>
  <PluginView pluginId={selectedPlugin()!} />
</Show>
```

提交: `feat: 创建 PluginView 动态渲染组件`

#### Task 3.2: 实现插件通信 Bridge

创建 `tauri-app/src/utils/pluginBridge.ts`:

```ts
export class PluginBridge {
  constructor(private pluginId: string) {}
  
  async call(method: string, params: any = {}) {
    return await invoke("call_plugin_method", {
      pluginId: this.pluginId,
      method,
      params,
    });
  }
  
  exposeToWindow() {
    (window as any).pluginAPI = {
      call: this.call.bind(this),
    };
  }
}
```

提交: `feat: 实现插件通信桥`

### Phase 4: 插件迁移 (4-6小时)

#### Task 4.1: 迁移 password-manager

1. 修改 `plugins/password-manager/Cargo.toml`:
   ```toml
   [lib]
   crate-type = ["cdylib"]
   ```

2. 创建 `plugins/password-manager/src/lib.rs`:
   ```rust
   use worktools_plugin_api::Plugin;
   
   pub struct PasswordManager;
   
   impl Plugin for PasswordManager {
       fn id(&self) -> &str { "password-manager" }
       fn name(&self) -> &str { "密码管理器" }
       fn get_view(&self) -> String {
           r#"<div id='app'>...</div>"#.to_string()
       }
       
       fn handle_call(&mut self, method: &str, params: Value) -> Result<Value> {
           match method {
               "list_passwords" => Ok(self.list_passwords()?),
               _ => Err("unknown method".into()),
           }
       }
   }
   
   #[no_mangle]
   pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
       let plugin: Box<dyn Plugin> = Box::new(PasswordManager);
       Box::leak(plugin)
   }
   ```

3. 编译安装:
   ```bash
   cargo build --release
   mkdir -p ~/.worktools/plugins/password-manager
   cp target/release/libpassword_manager.dylib ~/.worktools/plugins/password-manager/
   ```

提交: `refactor: 迁移 password-manager 到动态库架构`

#### Task 4.2: 迁移 auth-plugin

重复 Task 4.1 的步骤迁移 auth-plugin。

提交: `refactor: 迁移 auth-plugin 到动态库架构`

### Phase 5: 清理和优化 (2-3小时)

#### Task 5.1: 删除旧代码

删除:
- `shared/rpc-protocol/`
- `plugins/*/src/main.rs`
- 更新依赖引用

提交: `chore: 删除废弃的 JSON-RPC 代码`

#### Task 5.2: 错误处理增强

- 插件加载失败继续加载其他插件
- 前端错误提示 UI
- 日志记录优化

提交: `feat: 增强错误处理`

### Phase 6: 测试和文档 (2-3小时)

#### Task 6.1: 编写单元测试

创建 `tauri-app/src-tauri/src/plugin_manager/tests.rs`

提交: `test: 添加 PluginManager 单元测试`

#### Task 6.2: 更新文档

修改 `CLAUDE.md` 更新架构说明

提交: `docs: 更新架构文档`

## 总计时间

- Phase 1: 2-3 小时
- Phase 2: 4-5 小时
- Phase 3: 3-4 小时
- Phase 4: 4-6 小时
- Phase 5: 2-3 小时
- Phase 6: 2-3 小时

**总计: 17-24 小时**


## 插件迁移指南

### 从旧架构迁移的步骤

1. **修改 Cargo.toml**
   ```toml
   [lib]
   crate-type = ["cdylib"]  # 替换 [[bin]]
   ```

2. **重命名文件**
   ```bash
   mv src/main.rs src/lib.rs
   ```

3. **实现 Plugin trait**
   ```rust
   use worktools_plugin_api::Plugin;
   
   pub struct MyPlugin;
   
   impl Plugin for MyPlugin {
       fn id(&self) -> &str { "my-plugin" }
       fn name(&self) -> &str { "我的插件" }
       fn get_view(&self) -> String { r#"<div>...</div>"#.to_string() }
       fn handle_call(&mut self, method: &str, params: Value) -> Result<Value> {
           // 处理前端调用
       }
   }
   ```

4. **导出工厂函数**
   ```rust
   #[no_mangle]
   pub extern "C" fn plugin_create() -> *mut Box<dyn Plugin> {
       let plugin: Box<dyn Plugin> = Box::new(MyPlugin);
       Box::leak(plugin)
   }
   ```

5. **更新前端调用**
   ```javascript
   // 旧: invoke('plugin_method', { pluginId, method, params })
   // 新: window.pluginAPI.call('method', params)
   ```

## 完成检查清单

- [ ] shared/plugin-api 创建并编译通过
- [ ] PluginManager 重写完成
- [ ] 前端 PluginView 组件工作正常
- [ ] password-manager 迁移成功
- [ ] auth-plugin 迁移成功
- [ ] 删除旧的 JSON-RPC 代码
- [ ] 单元测试覆盖率 > 70%
- [ ] 文档更新完成
- [ ] 三个平台测试通过 (macOS/Windows/Linux)

