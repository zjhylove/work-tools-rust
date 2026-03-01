# 动态库插件架构设计

## 设计目标

将 Rust 版本插件架构改造为与 Java 版本一致的同进程动态库架构。

## 核心设计

### 1. 插件形式

**当前**: 独立可执行文件 → **目标**: 动态库

```rust
// 编译为动态库
[lib]
crate-type = ["cdylib"]

// 输出文件
// macOS: libpassword_manager.dylib
// Linux: libpassword_manager.so
// Windows: password_manager.dll
```

### 2. 加载机制

使用 `libloading` 动态加载插件:

```rust
let library = Library::new("plugin.dylib")?;
let create: Symbol<PluginCreateFn> = library.get(b"plugin_create")?;
let plugin = create();
```

### 3. 插件接口

统一的 Plugin trait:

```rust
pub trait Plugin {
    // 元信息
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn version(&self) -> &str;
    fn icon(&self) -> &str;
    
    // UI
    fn get_view(&self) -> String;  // 返回 HTML
    
    // 生命周期
    fn init(&mut self) -> Result<()>;
    fn destroy(&mut self) -> Result<()>;
    
    // 通信
    fn handle_call(&mut self, method: &str, params: Value) -> Result<Value>;
}
```

### 4. UI 渲染

```
插件 get_view() → HTML → 前端 innerHTML → WebView 渲染
```

前端通过 `window.pluginAPI` 与插件通信:

```javascript
window.pluginAPI.call('list_passwords', {})
  .then(result => console.log(result));
```

## 架构对比

| 维度 | Java 版本 | Rust 当前 | Rust 目标 |
|------|----------|----------|----------|
| 插件形式 | Jar + SPI | 可执行文件 | 动态库 |
| 加载方式 | ServiceLoader | Command::spawn | libloading |
| 通信方式 | 直接方法调用 | JSON-RPC IPC | 直接方法调用 |
| UI 渲染 | JavaFX Node | 硬编码组件 | HTML WebView |
| 生命周期 | init/getView/destroy | get_info/get_view | init/getView/destroy |

## 优势

1. **架构一致**: 与 Java 版本相同的开发模式
2. **性能更优**: 无 IPC 开销,直接函数调用
3. **开发简单**: 插件开发者只需实现 trait
4. **类型安全**: 编译期检查接口实现

## 风险和缓解

| 风险 | 缓解措施 |
|------|----------|
| ABI 不稳定 | 使用 cdylib,固定 Rust 版本 |
| 插件崩溃 | catch_unwind 保护 |
| 内存泄漏 | 严格测试 destroy() |

## 实施路径

1. 创建 plugin-api 共享库 (1h)
2. 重写 PluginManager (3h)
3. 前端动态渲染 (2h)
4. 迁移现有插件 (4h)
5. 测试和优化 (2h)

**总计**: 12 小时
