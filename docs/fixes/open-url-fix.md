# 修复密码管理器链接无法打开的问题

## 问题描述
在密码管理器插件中点击跳转链接时,链接无法在浏览器中打开。控制台显示:
```
[Log] [PluginAPI] 打开链接: http://www.baidu.com
[Info] Promoted URL from http://www.baidu.com/ to https
```
但浏览器没有打开链接。

## 根本原因
`open_url` Tauri 命令在 `commands.rs` 中定义了,但没有在 `lib.rs` 的 `invoke_handler` 中注册,导致前端调用时无法找到该命令。

## 解决方案

### 1. 修改 `tauri-app/src-tauri/src/lib.rs`
在 `invoke_handler` 中添加 `open_url` 命令注册:

```rust
.invoke_handler(tauri::generate_handler![
    // ... 其他命令
    commands::read_plugin_asset,
    commands::open_url,  // ← 添加这一行
])
```

### 2. 验证 `open_url` 函数
确认 `commands.rs` 中的函数实现正确:

```rust
/// 打开外部 URL
#[tauri::command]
pub async fn open_url(url: String) -> Result<(), String> {
    // 使用 opener crate 打开 URL (跨平台)
    opener::open(&url)
        .map_err(|e| format!("打开链接失败: {}", e))
}
```

### 3. 确认依赖
确保 `Cargo.toml` 中包含 `opener` 依赖:

```toml
[dependencies]
opener = "0.7"
```

## 测试步骤
1. 启动应用: `npm run tauri dev`
2. 打开密码管理器插件
3. 点击任意 URL 链接
4. 验证链接在默认浏览器中打开

## 技术细节
- **opener crate**: 跨平台库,支持 macOS、Windows 和 Linux
- **Tauri 命令注册**: 所有暴露给前端的命令必须在 `invoke_handler` 中注册
- **异步处理**: `open_url` 是异步函数,使用 `pub async fn`

## 相关文件
- `/Users/zj/Project/Rust/work-tools-rust/tauri-app/src-tauri/src/lib.rs`
- `/Users/zj/Project/Rust/work-tools-rust/tauri-app/src-tauri/src/commands.rs`
- `/Users/zj/Project/Rust/work-tools-rust/tauri-app/src-tauri/Cargo.toml`
- `/Users/zj/Project/Rust/work-tools-rust/tauri-app/src/components/PluginPlaceholder.tsx`

## 后续优化建议
1. 考虑添加 URL 白名单验证,防止打开恶意链接
2. 添加用户确认提示,询问是否打开外部链接
3. 支持自定义浏览器选择
