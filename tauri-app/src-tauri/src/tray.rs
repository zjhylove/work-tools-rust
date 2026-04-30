use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{menu::MenuBuilder, menu::MenuItemBuilder, Runtime, Manager};

const TRAY_ID: &str = "worktools-tray";
const MAIN_WINDOW: &str = "main";
const MENU_TOGGLE: &str = "toggle";
const MENU_QUIT: &str = "quit";
const LABEL_HIDE: &str = "隐藏窗口";
const LABEL_SHOW: &str = "显示窗口";
const LABEL_QUIT: &str = "退出";
const DEFAULT_TOOLTIP: &str = "Work Tools";
const HIDE_HINT_TOOLTIP: &str = "应用已最小化到托盘，双击图标可恢复窗口";

// 点击"退出"时 handle.exit(0) 会触发 Tauri 销毁窗口，CloseRequested 随之被调用。
// 必须跳过 prevent_close()，否则窗口关闭被拦截，WebView2 清理顺序出错导致 Error 1412。
static IS_QUITTING: AtomicBool = AtomicBool::new(false);

#[derive(Deserialize, Serialize)]
struct TrayConfig {
    hide_to_tray_hint_shown: bool,
}

/// 不会 panic，所有错误仅记录日志。托盘创建失败不影响应用正常启动。
pub fn start_tray<R: Runtime>(app: &mut tauri::App<R>) {
    let Some(icon) = app.default_window_icon().cloned() else {
        tracing::warn!("未找到默认窗口图标，跳过系统托盘创建");
        return;
    };

    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        tracing::warn!("未找到主窗口，跳过系统托盘创建");
        return;
    };

    let app_handle = app.handle().clone();

    let menu = match build_menu(app.handle(), LABEL_HIDE) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("创建托盘菜单失败: {}", e);
            return;
        }
    };

    if let Err(e) = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip(DEFAULT_TOOLTIP)
        .on_menu_event(move |handle, event| match event.id().as_ref() {
            MENU_TOGGLE => {
                if let Err(e) = toggle_window(handle) {
                    tracing::warn!("托盘切换窗口失败: {}", e);
                }
            }
            MENU_QUIT => {
                IS_QUITTING.store(true, Ordering::Relaxed);
                // window.close() 和 handle.exit(0) 都通过事件队列发送消息，按 FIFO 顺序处理。
                // close 先入队先处理，CloseRequested 因 IS_QUITTING 跳过 prevent_close，
                // 窗口正常关闭、WebView2 完成清理，然后 exit 才被处理。
                // 若只调 exit() 而不先 close()，窗口直接销毁，WebView2 类注销 Error 1412。
                if let Some(w) = handle.get_webview_window(MAIN_WINDOW) {
                    let _ = w.close();
                }
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

    let window_hide = window.clone();
    window.on_window_event({
        let app_handle = app_handle.clone();
        move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if IS_QUITTING.load(Ordering::Relaxed) {
                    return;
                }

                api.prevent_close();

                if let Err(e) = window_hide.hide() {
                    tracing::warn!("隐藏窗口失败: {}", e);
                    return;
                }

                if !hint_already_shown() {
                    show_hint_tooltip(&app_handle);
                }

                replace_tray_menu(&app_handle, LABEL_SHOW);
            }
        }
    });

    tracing::info!("系统托盘创建成功");
}

fn toggle_window<R: Runtime>(app_handle: &tauri::AppHandle<R>) -> anyhow::Result<()> {
    let window = app_handle
        .get_webview_window(MAIN_WINDOW)
        .ok_or_else(|| anyhow::anyhow!("找不到主窗口"))?;

    if window.is_visible()? {
        window.hide()?;
        replace_tray_menu(app_handle, LABEL_SHOW);
    } else {
        window.show()?;
        window.set_focus()?;
        replace_tray_menu(app_handle, LABEL_HIDE);
    }

    Ok(())
}

fn build_menu<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    label: &str,
) -> anyhow::Result<tauri::menu::Menu<R>> {
    let toggle_item = MenuItemBuilder::with_id(MENU_TOGGLE, label).build(app_handle)?;
    let quit_item = MenuItemBuilder::with_id(MENU_QUIT, LABEL_QUIT).build(app_handle)?;
    Ok(MenuBuilder::new(app_handle)
        .item(&toggle_item)
        .separator()
        .item(&quit_item)
        .build()?)
}

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

fn show_hint_tooltip<R: Runtime>(app_handle: &tauri::AppHandle<R>) {
    if let Some(tray_icon) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray_icon.set_tooltip(Some(HIDE_HINT_TOOLTIP));
    }
    if let Err(e) = mark_hint_shown() {
        tracing::warn!("保存托盘提示状态失败: {}", e);
    }
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        if let Some(tray_icon) = handle.tray_by_id(TRAY_ID) {
            let _ = tray_icon.set_tooltip(Some(DEFAULT_TOOLTIP));
        }
    });
}

// ── 首次隐藏提示状态持久化 ────────────────────────

fn tray_config_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(crate::paths::config_dir()?.join("tray-config.json"))
}

fn hint_already_shown() -> bool {
    let Ok(path) = tray_config_path() else {
        return false;
    };
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
