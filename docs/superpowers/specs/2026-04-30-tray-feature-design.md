# 系统托盘功能设计

## 日期
2026-04-30

## 概述
为 Work Tools 应用添加系统托盘功能。关闭窗口时最小化到托盘，系统通知区域显示图标，右键菜单可显示/隐藏窗口、退出应用。

## 行为规范

| 操作 | 行为 |
|---|---|
| 点击窗口 ✕ | 隐藏窗口到托盘（不退出） |
| 首次 ✕ | 隐藏 + 系统通知"应用已最小化到托盘" |
| 双击托盘图标 | 显示/隐藏窗口 |
| 右键 → 显示窗口/隐藏窗口 | 切换窗口可见性 |
| 右键 → 退出 | 完全退出应用 |

## 架构

新建 `tauri-app/src-tauri/src/tray.rs` 模块，封装全部托盘逻辑。`lib.rs` 中一行调用。

```
lib.rs  setup()  →  tray::start_tray(app)
```

### 模块职责

`tray.rs`:
- 从 `resources/icons/` 加载托盘图标（跨平台）
- 构建右键菜单（动态标签：显示/隐藏窗口）
- 注册事件处理：
  - 菜单"显示/隐藏窗口" → toggle 窗口可见性
  - 菜单"退出" → `app.exit(0)`
  - 双击托盘图标 → toggle 窗口可见性
  - 窗口 `close_requested` → 阻止默认关闭，隐藏窗口，首次通知

### 菜单结构

```
┌─────────────────┐
│ 显示窗口        │  ← 窗口不可见时显示"显示窗口"，可见时显示"隐藏窗口"
├─────────────────┤
│ 退出            │
└─────────────────┘
```

菜单在 toggle 后完全重建（Tauri 2.x tray menu 不支持原地修改文本）。

### 首次提示状态存储

文件: `~/.worktools/config/tray-config.json`
```json
{ "hide_to_tray_hint_shown": true }
```

通过 `directories` 库确定路径，与现有 config 模块一致。

### 托盘图标

复用打包产物中的图标文件，通过 `app.path().resource_dir()` 解析：
- Windows: `icons/icon.ico`
- macOS: `icons/icon.icns`（降级到 png）
- Linux: `icons/32x32.png`

## lib.rs 变更

仅在 `setup()` 中加一行：

```rust
.setup(|app| {
    let manager = plugin_manager.clone();
    tauri::async_runtime::spawn(async move { /* ... */ });
    app.manage(plugin_manager);
    crate::tray::start_tray(app)?; // 新增
    Ok(())
})
```

## 端到端流程

```
应用启动
  → init_logging
  → PluginManager::new
  → tauri::Builder
      → setup:
          PluginManager::init (spawn)
          app.manage(plugin_manager)
          tray::start_tray(app)
              ├─ 加载图标
              ├─ 构建菜单
              ├─ 注册双击事件 → toggle_window
              └─ 注册 close_requested 事件
                  ├─ api.prevent_close()
                  ├─ window.hide()
                  ├─ 检查 hint_shown 标记
                  ├─ 首次: 发送系统通知
                  └─ 标记 hint_shown
  → 应用运行
      ├─ 双击托盘图标 → toggle_window
      ├─ 菜单"显示/隐藏窗口" → toggle_window + rebuild_menu
      └─ 菜单"退出" → app.exit(0)
```

## 错误处理

| 场景 | 策略 |
|---|---|
| 托盘图标文件缺失 | `tracing::warn!`，不阻塞启动 |
| 托盘创建失败（某些 Linux DE） | `tracing::warn!`，应用正常运行，仅无托盘 |
| 通知发送失败 | 静默忽略 |
| 快速连续 toggle | Tauri 事件队列串行处理，天然安全 |

核心原则：托盘是辅助功能，任何错误都不应阻止应用正常启动。

## 涉及文件

| 文件 | 操作 | 说明 |
|---|---|---|
| `tauri-app/src-tauri/src/tray.rs` | 新增 | 托盘模块 |
| `tauri-app/src-tauri/src/lib.rs` | 修改 | setup() 中加 1 行调用 |
| `tauri-app/src-tauri/Cargo.toml` | 已验证 | `tray-icon` feature 已启用 |
