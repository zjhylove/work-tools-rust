//! # 系统托盘模块
//!
//! 管理系统托盘图标、菜单和窗口隐藏/显示行为。
//! 支持"关闭窗口最小化到托盘"模式。
//!
//! ## 行为逻辑
//! 1. 点击窗口关闭按钮 → 窗口隐藏（最小化到托盘），不退出
//! 2. 双击托盘图标 → 切换窗口显示/隐藏
//! 3. 托盘菜单"退出" → 真正退出应用
//! 4. 首次隐藏时显示提示 tooltip，5 秒后恢复
//!
//! ## Rust 知识点
//! - `AtomicBool`: 无锁的原子布尔类型，用于线程间信号传递
//! - `Ordering::Relaxed`: 最宽松的内存排序，适合简单的标志位
//! - `TrayIconBuilder`: Tauri 的托盘建造者模式

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{menu::MenuBuilder, menu::MenuItemBuilder, Runtime, Manager};

// ── 常量 ──

const TRAY_ID: &str = "worktools-tray";
const MAIN_WINDOW: &str = "main";
const MENU_TOGGLE: &str = "toggle";
const MENU_QUIT: &str = "quit";
const LABEL_HIDE: &str = "隐藏窗口";
const LABEL_SHOW: &str = "显示窗口";
const LABEL_QUIT: &str = "退出";
const DEFAULT_TOOLTIP: &str = "Work Tools";
const HIDE_HINT_TOOLTIP: &str = "应用已最小化到托盘，双击图标可恢复窗口";

/// 是否正在退出应用
///
/// ## 为什么需要这个标志？
/// 点击"退出"按钮时，`handle.exit(0)` 会触发 Tauri 销毁窗口，
/// 进而触发 `CloseRequested` 事件 → 我们的逻辑会阻止关闭（`prevent_close`）。
/// 如果不跳过这个阻止，WebView2 的清理顺序会出错，导致 Windows Error 1412。
///
/// ## Rust 知识点: AtomicBool
/// `AtomicBool` 是线程安全的布尔值，使用 CPU 原子指令保证操作的不可分割性。
/// 相比 `Mutex<bool>` 更轻量，适合标志位场景。
/// `Ordering::Relaxed` 表示我们只需要原子性，不需要内存屏障（这里不涉及其他共享数据）。
static IS_QUITTING: AtomicBool = AtomicBool::new(false);

/// 托盘配置（持久化到 JSON 文件）
#[derive(Deserialize, Serialize)]
struct TrayConfig {
    /// 首次隐藏到托盘的提示是否已经显示过
    hide_to_tray_hint_shown: bool,
}

/// 启动系统托盘
///
/// 不会 panic — 所有错误仅记录日志。托盘创建失败不影响应用正常启动。
///
/// ## Rust 知识点: let-else 语法
/// ```ignore
/// let Some(icon) = app.default_window_icon().cloned() else {
///     tracing::warn!("...");
///     return;
/// };
/// ```
/// `let-else` 在模式匹配失败时执行 else 块（必须发散：return/break/continue/panic）。
pub fn start_tray<R: Runtime>(app: &mut tauri::App<R>) {
    // 获取默认窗口图标（Tauri 在构建时从 tauri.conf.json 配置的图标）
    let Some(icon) = app.default_window_icon().cloned() else {
        tracing::warn!("未找到默认窗口图标，跳过系统托盘创建");
        return;
    };

    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        tracing::warn!("未找到主窗口，跳过系统托盘创建");
        return;
    };

    let app_handle = app.handle().clone();

    // 构建托盘菜单
    let menu = match build_menu(app.handle(), LABEL_HIDE) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("创建托盘菜单失败: {}", e);
            return;
        }
    };

    // ── 创建托盘图标 ──
    // `TrayIconBuilder::with_id()` 使用建造者模式配置托盘
    if let Err(e) = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .tooltip(DEFAULT_TOOLTIP)
        // ── 菜单点击事件 ──
        .on_menu_event(move |handle, event| match event.id().as_ref() {
            MENU_TOGGLE => {
                if let Err(e) = toggle_window(handle) {
                    tracing::warn!("托盘切换窗口失败: {}", e);
                }
            }
            MENU_QUIT => {
                // 设置退出标志，防止 CloseRequested 拦截关闭
                IS_QUITTING.store(true, Ordering::Relaxed);
                // 先 close 再 exit：确保 WebView2 按正确顺序清理
                // 如果直接 exit() 而不先 close()，WebView2 类注销会触发 Error 1412
                if let Some(w) = handle.get_webview_window(MAIN_WINDOW) {
                    let _ = w.close();
                }
                handle.exit(0);
            }
            _ => {}
        })
        // ── 托盘图标点击事件 ──
        .on_tray_icon_event({
            let app_handle = app_handle.clone();
            move |_tray, event| {
                // 只响应鼠标左键弹起（防止双击时触发两次）
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

    // ── 监听窗口关闭事件 ──
    // 拦截窗口关闭，改为隐藏到托盘（非退出时）
    let window_hide = window.clone();
    window.on_window_event({
        let app_handle = app_handle.clone();
        move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 如果正在退出，放行关闭
                if IS_QUITTING.load(Ordering::Relaxed) {
                    return;
                }

                // 阻止窗口关闭，改为隐藏
                api.prevent_close();

                if let Err(e) = window_hide.hide() {
                    tracing::warn!("隐藏窗口失败: {}", e);
                    return;
                }

                // 首次隐藏时显示提示
                if !hint_already_shown() {
                    show_hint_tooltip(&app_handle);
                }

                // 更新托盘菜单：显示 → 隐藏
                replace_tray_menu(&app_handle, LABEL_SHOW);
            }
        }
    });

    tracing::info!("系统托盘创建成功");
}

/// 切换窗口显示/隐藏状态
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

/// 构建托盘菜单
/// `label` 决定"显示窗口"还是"隐藏窗口"
fn build_menu<R: Runtime>(
    app_handle: &tauri::AppHandle<R>,
    label: &str,
) -> anyhow::Result<tauri::menu::Menu<R>> {
    let toggle_item = MenuItemBuilder::with_id(MENU_TOGGLE, label)
        .build(app_handle)?;
    let quit_item = MenuItemBuilder::with_id(MENU_QUIT, LABEL_QUIT)
        .build(app_handle)?;
    Ok(MenuBuilder::new(app_handle)
        .item(&toggle_item)
        .separator() // 分隔线
        .item(&quit_item)
        .build()?)
}

/// 替换托盘菜单
/// Tauri 2.x 不支持原地修改菜单项文本，需要重建整个菜单
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

/// 显示"应用已最小化到托盘"的提示 tooltip
/// 5 秒后自动恢复默认 tooltip
fn show_hint_tooltip<R: Runtime>(app_handle: &tauri::AppHandle<R>) {
    if let Some(tray_icon) = app_handle.tray_by_id(TRAY_ID) {
        let _ = tray_icon.set_tooltip(Some(HIDE_HINT_TOOLTIP));
    }
    if let Err(e) = mark_hint_shown() {
        tracing::warn!("保存托盘提示状态失败: {}", e);
    }
    // 5 秒后恢复默认 tooltip
    let handle = app_handle.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        if let Some(tray_icon) = handle.tray_by_id(TRAY_ID) {
            let _ = tray_icon.set_tooltip(Some(DEFAULT_TOOLTIP));
        }
    });
}

// ── 首次隐藏提示状态持久化 ──

/// 获取托盘配置文件路径
fn tray_config_path() -> anyhow::Result<std::path::PathBuf> {
    Ok(crate::paths::config_dir()?.join("tray-config.json"))
}

/// 检查是否已经显示过提示
///
/// ## Rust 知识点: 链式调用 + Option 组合子
/// `read_to_string().ok().and_then(|s| serde_json::from_str(&s).ok())`:
/// - `.ok()` 将 Result 转为 Option（丢弃错误信息）
/// - `.and_then()` 在 Some 时继续处理
/// - `.map()` 从 Option 中提取值并转换
/// - `.unwrap_or(false)` 提供最终默认值
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

/// 标记提示已显示（持久化到文件）
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
