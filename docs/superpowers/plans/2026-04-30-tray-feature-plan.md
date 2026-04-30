# 系统托盘功能 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 Work Tools 应用添加系统托盘功能 - 关闭窗口时最小化到托盘而非退出

**Architecture:** 新建 `tray.rs` 模块，封装托盘创建、菜单事件、窗口关闭拦截。`lib.rs` 中一行调用。用 `dirs` 库管理首次提示状态文件。

**Tech Stack:** Rust + Tauri 2.x (`tray-icon` feature, 已启用) | `dirs` (已有依赖) | `tracing` (已有依赖)

---

### Task 1: 创建 `tray.rs` 模块

**Files:**
- Create: `tauri-app/src-tauri/src/tray.rs`

- [ ] **Step 1: 写入 tray.rs 完整代码**

```rust
use anyhow::{Context, Result};
use std::path::PathBuf;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, Runtime,
};
use tracing;

const TRAY_ID: &str = "worktools-tray";
const DEFAULT_TOOLTIP: &str = "Work Tools";
const HINT_TOOLTIP: &str = "应用已最小化到托盘，双击图标可恢复窗口";

fn config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".worktools")
        .join("config")
        .join("tray-config.json")
}

fn is_hint_shown() -> bool {
    config_path().exists()
}

fn mark_hint_shown() {
    let path = config_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, r#"{"hide_to_tray_hint_shown":true}"#);
}

fn build_menu(app: &tauri::AppHandle, is_window_visible: bool) -> Result<tauri::menu::Menu<impl Runtime>> {
    let toggle_label = if is_window_visible { "隐藏窗口" } else { "显示窗口" };
    let toggle = MenuItemBuilder::with_id("toggle", toggle_label).build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
    MenuBuilder::new(app)
        .item(&toggle)
        .item(&quit)
        .build()
        .context("构建托盘菜单失败")
}

/// 切换窗口可见性, 并重建菜单
fn toggle_window(app: &tauri::AppHandle, tray: &tauri::tray::TrayIcon) {
    let Some(window) = app.get_webview_window("main") else {
        tracing::warn!("找不到主窗口");
        return;
    };
    let visible = window.is_visible().unwrap_or(true);
    if visible {
        let _ = window.hide();
    } else {
        let _ = window.show();
        let _ = window.set_focus();
    };
    if let Ok(menu) = build_menu(app, !visible) {
        let _ = tray.set_menu(Some(menu));
    }
    tracing::info!("托盘: 窗口已{}", if !visible { "隐藏" } else { "显示" });
}

/// 启动托盘功能。失败时记录警告，不影响应用正常启动。
pub fn start_tray<R: Runtime>(app: &mut tauri::App<R>) {
    let window = match app.get_webview_window("main") {
        Some(w) => w,
        None => {
            tracing::warn!("托盘: 找不到主窗口, 跳过托盘初始化");
            return;
        }
    };

    let icon = app.default_window_icon().cloned();
    let is_visible = window.is_visible().unwrap_or(true);

    let menu = match build_menu(app.handle(), is_visible) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("托盘: 菜单构建失败: {}, 跳过托盘初始化", e);
            return;
        }
    };

    let tray = match TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon.unwrap_or_else(|| {
            tracing::warn!("托盘: 未找到默认图标, 使用空图标");
            tauri::Icon::default() // 注意: 这个 fallback 可能编译不过，见下方说明
        }))
        .menu(&menu)
        .tooltip(DEFAULT_TOOLTIP)
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "toggle" => {
                    if let Some(tray) = app.tray_icon_by_id(TRAY_ID) {
                        toggle_window(app, &tray);
                    }
                }
                "quit" => {
                    tracing::info!("托盘: 用户退出应用");
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event {
                let app = tray.app_handle().clone();
                toggle_window(&app, tray);
            }
        })
        .build(app)
    {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!("托盘: 创建失败: {} (某些 Linux 桌面环境不支持)", e);
            return;
        }
    };

    // 拦截窗口关闭 → 隐藏到托盘
    window.on_window_event({
        let app_handle = app.handle().clone();
        move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let Some(window) = app_handle.get_webview_window("main") else {
                    return;
                };
                let _ = window.hide();

                if !is_hint_shown() {
                    if let Some(t) = app_handle.tray_icon_by_id(TRAY_ID) {
                        let _ = t.set_tooltip(Some(HINT_TOOLTIP));
                    }
                    mark_hint_shown();

                    let app_clone = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        if let Some(t) = app_clone.tray_icon_by_id(TRAY_ID) {
                            let _ = t.set_tooltip(Some(DEFAULT_TOOLTIP));
                        }
                    });
                }

                if let Some(t) = app_handle.tray_icon_by_id(TRAY_ID) {
                    if let Ok(menu) = build_menu(&app_handle, false) {
                        let _ = t.set_menu(Some(menu));
                    }
                }

                tracing::debug!("托盘: 窗口已隐藏到托盘");
            }
        }
    });

    tracing::info!("托盘: 初始化成功");
}
```

**说明**: Icon fallback — 如果 `tauri::Icon::default()` 编译不过，改用 `app.default_window_icon().cloned()` 的 unwrap 并在缺失时 return early。实际运行中打包产物必有图标，此路径不会触发。

- [ ] **Step 2: 注册 tray 模块到 lib.rs**

文件: `tauri-app/src-tauri/src/lib.rs`，在 `pub mod plugin_manager;` 后添加一行：

```rust
mod tray;
```

- [ ] **Step 3: 验证编译通过**

```bash
cargo check -p work-tools 2>&1
```

预期: `Finished` 或仅 warnings，无 errors。

- [ ] **Step 4: 修复编译错误 (如有)**

根据实际 Tauri 2.x API 调整可能的差异：
- `TrayIconBuilder::with_id` vs `TrayIconBuilder::new` — 查项目所用 Tauri 版本的实际 API
- `tauri::Icon::default()` 不存在则改为 `Option<Icon>` 直接传 `None` 或从 png bytes 构造
- `build_menu` 返回类型的 `impl Runtime` 泛型按编译提示调整

- [ ] **Step 5: Commit**

```bash
git add tauri-app/src-tauri/src/tray.rs tauri-app/src-tauri/src/lib.rs
git commit -m "feat: 添加系统托盘功能 — 最小化到托盘、首次提示、双击切换

- 新建 tray.rs 模块，封装全部托盘逻辑
- 关闭窗口时隐藏到托盘而非退出
- 首次关闭时显示提示 (托盘 tooltip 持续 5 秒)
- 双击托盘图标 / 右键菜单可切换窗口可见性
- 托盘创建失败不影响应用正常启动"
```

---

### Task 2: 修改 lib.rs 调用托盘

**Files:**
- Modify: `tauri-app/src-tauri/src/lib.rs`

- [ ] **Step 1: 在 setup() 中调用托盘初始化**

阅读当前 `lib.rs`，在 `app.manage(plugin_manager);` 之后、`Ok(())` 之前添加：

```rust
crate::tray::start_tray(app);
```

修改后的 setup 闭包：

```rust
.setup(|app| {
    let manager = plugin_manager.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = manager.init().await {
            tracing::error!("插件管理器初始化失败: {}", e);
        }
    });

    app.manage(plugin_manager);

    crate::tray::start_tray(app);

    println!("Work Tools 应用启动成功");
    Ok(())
})
```

- [ ] **Step 2: 验证编译**

```bash
cargo check -p work-tools 2>&1
```

- [ ] **Step 3: Commit**

```bash
git add tauri-app/src-tauri/src/lib.rs
git commit -m "feat: lib.rs 接入托盘模块"
```

---

### Task 3: 功能验证

- [ ] **Step 1: 编译运行**

```bash
cd tauri-app && npm run tauri dev
```

- [ ] **Step 2: 手动验证 checklist**

- [ ] 应用启动时托盘图标出现 (Windows 系统通知区域 / macOS 菜单栏)
- [ ] 右键托盘图标 → 菜单包含 "隐藏窗口" 和 "退出"
- [ ] 点击 "隐藏窗口" → 窗口隐藏，菜单变为 "显示窗口"
- [ ] 点击 "显示窗口" → 窗口恢复并获取焦点，菜单变为 "隐藏窗口"
- [ ] 点击窗口 ✕ → 窗口隐藏（不退出），菜单更新
- [ ] 首次 ✕ → 托盘 tooltip 变为提示文字，5 秒后恢复
- [ ] 第二次 ✕ → 仅隐藏，tooltip 不变
- [ ] 双击托盘图标 → 切换窗口可见性
- [ ] 点击 "退出" → 应用完全退出

- [ ] **Step 3: 异常场景验证**

- [ ] 删除 `tray-config.json` → 下次隐藏应再次显示 tooltip 提示
- [ ] 快速连续点击 ✕ 和双击托盘 → 无崩溃，状态一致

---

### 注意事项

1. **`TrayIconBuilder` API 差异**: Tauri 2.x 不同小版本 API 可能不同。编译时若有 API 不匹配，根据编译器提示调整方法名/参数。
2. **Icon fallback**: `tauri::Icon::default()` 可能不存在于所用版本。若编译报错，改为在 `app.default_window_icon()` 为 `None` 时直接 `return` 跳过托盘初始化。
3. **`build_menu` 返回类型**: `impl Runtime` 泛型可避免显式类型标注。若编译器要求具体类型，改为 `tauri::menu::Menu<tauri::Wry>`。
