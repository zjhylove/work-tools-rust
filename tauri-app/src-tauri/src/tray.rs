use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{menu::MenuBuilder, menu::MenuItemBuilder, Runtime, Manager};

const TRAY_ID: &str = "worktools-tray";
const DEFAULT_TOOLTIP: &str = "Work Tools";
const HIDE_HINT_TOOLTIP: &str = "应用已最小化到托盘，双击图标可恢复窗口";

#[derive(Deserialize, Serialize)]
struct TrayConfig {
    hide_to_tray_hint_shown: bool,
}

/// 系统托盘入口，由 lib.rs 的 setup 中调用。
/// 不会 panic，所有错误仅记录日志。托盘创建失败不影响应用正常启动。
pub fn start_tray<R: Runtime>(app: &mut tauri::App<R>) {
    let Some(icon) = app.default_window_icon().cloned() else {
        tracing::warn!("未找到默认窗口图标，跳过系统托盘创建");
        return;
    };

    let Some(window) = app.get_webview_window("main") else {
        tracing::warn!("未找到主窗口，跳过系统托盘创建");
        return;
    };

    let app_handle = app.handle().clone();
    let hint_shown = Arc::new(AtomicBool::new(hint_already_shown()));

    // 初始菜单：窗口默认可见，显示"隐藏窗口"
    let menu = match build_menu(app.handle(), "隐藏窗口") {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("创建托盘菜单失败: {}", e);
            return;
        }
    };

    // 构建托盘
    if let Err(e) = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip(DEFAULT_TOOLTIP)
        .on_menu_event(move |handle, event| match event.id().as_ref() {
            "toggle" => {
                if let Err(e) = toggle_window(handle) {
                    tracing::warn!("托盘切换窗口失败: {}", e);
                }
            }
            "quit" => {
                handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event({
            let app_handle = app_handle.clone();
            move |_tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                } = event
                {
                    if let Err(e) = toggle_window(&app_handle) {
                        tracing::warn!("托盘点击切换窗口失败: {}", e);
                    }
                }
            }
        })
        .build(app)
    {
        tracing::warn!("创建系统托盘失败: {}", e);
        return;
    };

    // 拦截窗口关闭 → 隐藏到托盘
    let window_hide = window.clone();
    window.on_window_event({
        let app_handle = app_handle.clone();
        let hint_shown = hint_shown.clone();
        move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();

                if let Err(e) = window_hide.hide() {
                    tracing::warn!("隐藏窗口失败: {}", e);
                    return;
                }

                // 首次隐藏时在托盘上显示 tooltip 提示
                if !hint_shown.load(Ordering::Relaxed) {
                    if let Some(tray_icon) = app_handle.tray_by_id(TRAY_ID) {
                        let _ = tray_icon.set_tooltip(Some(HIDE_HINT_TOOLTIP));
                    }
                    if let Err(e) = mark_hint_shown() {
                        tracing::warn!("保存托盘提示状态失败: {}", e);
                    }
                    hint_shown.store(true, Ordering::Relaxed);

                    // 5 秒后恢复默认 tooltip
                    let handle = app_handle.clone();
                    tauri::async_runtime::spawn(async move {
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                        if let Some(tray_icon) = handle.tray_by_id(TRAY_ID) {
                            let _ = tray_icon.set_tooltip(Some(DEFAULT_TOOLTIP));
                        }
                    });
                }

                // 重建菜单（窗口已隐藏 → "显示窗口"）
                replace_tray_menu(&app_handle, "显示窗口");
            }
        }
    });

    tracing::info!("系统托盘创建成功");
}

/// 切换主窗口可见性并重建托盘菜单
fn toggle_window<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> anyhow::Result<()> {
    let window = app_handle
        .get_webview_window("main")
        .ok_or_else(|| anyhow::anyhow!("找不到主窗口"))?;

    if window.is_visible()? {
        window.hide()?;
        replace_tray_menu(app_handle, "显示窗口");
    } else {
        window.show()?;
        window.set_focus()?;
        replace_tray_menu(app_handle, "隐藏窗口");
    }

    Ok(())
}

/// 构建托盘右键菜单
fn build_menu<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    label: &str,
) -> anyhow::Result<tauri::menu::Menu<R>> {
    let toggle_item = MenuItemBuilder::with_id("toggle", label).build(app_handle)?;
    let quit_item = MenuItemBuilder::with_id("quit", "退出").build(app_handle)?;
    Ok(MenuBuilder::new(app_handle)
        .item(&toggle_item)
        .separator()
        .item(&quit_item)
        .build()?)
}

/// 替换托盘右键菜单
///
/// Tauri 2.x 无法原地修改菜单项文本，每次切换都需要重建并替换整个菜单。
fn replace_tray_menu<R: Runtime>(app_handle: &tauri::AppHandle<R>, label: &str) {
    if let Some(tray_icon) = app_handle.tray_by_id(TRAY_ID) {
        match build_menu(app_handle, label) {
            Ok(menu) => {
                let _ = tray_icon.set_menu(Some(menu));
            }
            Err(e) => {
                tracing::warn!("重建托盘菜单失败: {}", e);
            }
        }
    }
}

// ── 首次隐藏提示状态持久化 ────────────────────────

fn tray_config_path() -> anyhow::Result<std::path::PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("无法找到用户主目录"))?;
    Ok(home.join(".worktools/config/tray-config.json"))
}

fn hint_already_shown() -> bool {
    let path = match tray_config_path() {
        Ok(p) => p,
        Err(_) => return false,
    };
    if !path.exists() {
        return false;
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str::<TrayConfig>(&s).ok())
        .map(|c| c.hide_to_tray_hint_shown)
        .unwrap_or(false)
}

fn mark_hint_shown() -> anyhow::Result<()> {
    let path = tray_config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let config = TrayConfig {
        hide_to_tray_hint_shown: true,
    };
    std::fs::write(path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}
