mod commands;
mod db;
mod preview;
mod status;
mod velometer;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::Manager;
use tauri::Emitter;
#[cfg(not(target_os = "android"))]
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
#[cfg(not(target_os = "android"))]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(not(target_os = "android"))]
use tauri_plugin_global_shortcut::ShortcutState;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

/// 启动时间：启动初期 Windows 可能把上次会话的最小化状态恢复给新实例，
/// 这时 Resized→最小化→hide 会把窗口藏进托盘，表现为"只有托盘图标没有界面"。
/// 前 8 秒内的最小化事件只还原窗口，不隐藏。
static STARTED_AT: OnceLock<Instant> = OnceLock::new();

/// 用户主动隐藏到托盘标记：关闭按钮 / 最小化 / 托盘「隐藏」都会置位，
/// 启动 watchdog 依据它区分"用户想隐藏"与"窗口本该显示却没显示"，避免对抗。
static USER_HIDDEN: AtomicBool = AtomicBool::new(false);

/// 设置仓库根目录（Android 上由 setup 指向应用私有 webroot，桌面版无需调用）。
/// 统一 canonicalize：Windows 上 fs::canonicalize 会补 `\\?\` 前缀，若根目录不规范化，
/// preview::handler 的 `normalized.starts_with(root)` 前缀判断会误判为越权（403）。
pub fn init_repo_root(path: PathBuf) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path);
    let _ = ROOT.set(canonical);
}

/// 仓库根目录：src-tauri 的上一级（编译期常量，本机构建本机运行）。
pub fn repo_root() -> &'static Path {
    ROOT.get_or_init(|| {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri 应有父目录");
        p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
    })
}

/// 取主窗口（普通 Window）。不用 get_webview_window("main")：应用内打开的网页标签
/// 是挂在主窗口下的子 webview（browser-*），会让 window.is_webview_window() 为 false，
/// get_webview_window 于是返回 None，show_main/hide_to_tray 全部静默失效
/// （表现为最小化/关闭后唤不回来，只能重启）。get_window 不依赖该判定，始终可用。
#[cfg(not(target_os = "android"))]
pub(crate) fn main_window(app: &tauri::AppHandle) -> Option<tauri::Window> {
    app.get_window("main")
}

/// 显示并聚焦主窗口。Windows 上直接 show() 有时不置顶，需 unminimize + set_focus 兜底。
#[cfg(not(target_os = "android"))]
fn show_main(app: &tauri::AppHandle) {
    show_main_impl(app);
}

#[cfg(not(target_os = "android"))]
fn show_main_impl(app: &tauri::AppHandle) {
    USER_HIDDEN.store(false, Ordering::Relaxed);
    if let Some(win) = main_window(app) {
        #[cfg(target_os = "windows")]
        force_app_window(&win);
        let was_hidden = !win.is_visible().unwrap_or(true) || win.is_minimized().unwrap_or(false);
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
        if win.is_fullscreen().unwrap_or(false) {
            let _ = win.set_fullscreen(false);
            let _ = app.emit("alpehuez-fullscreen-exit", "tray");
        }
        if was_hidden {
            // 透明无边框窗口从托盘唤出后偶发不重绘（看起来像没有窗口），
            // 仅在刚显示时微调一次尺寸强制合成器重绘，随后立即恢复原尺寸。
            let size = win.outer_size().unwrap_or_default();
            if size.width > 0 && size.height > 0 {
                let _ = win.set_size(tauri::Size::Physical(tauri::PhysicalSize::new(size.width + 1, size.height)));
                let _ = win.set_size(tauri::Size::Physical(size));
            }
        }
    }
}

/// 隐藏到系统托盘并记录"用户主动隐藏"，避免启动 watchdog 把窗口拉回来。
/// 事件闭包拿到的是 &Window，快捷键/托盘拿到的是 WebviewWindow，用 trait 统一。
#[cfg(not(target_os = "android"))]
pub(crate) trait Hideable {
    fn hide_window(&self);
}
#[cfg(not(target_os = "android"))]
impl Hideable for tauri::Window {
    fn hide_window(&self) {
        let _ = self.hide();
    }
}
#[cfg(not(target_os = "android"))]
impl Hideable for tauri::WebviewWindow {
    fn hide_window(&self) {
        let _ = self.hide();
    }
}
#[cfg(not(target_os = "android"))]
pub(crate) fn hide_to_tray(win: &impl Hideable) {
    USER_HIDDEN.store(true, Ordering::Relaxed);
    let _ = win.hide_window();
}

/// 无边框透明窗口失去系统阴影，用 DwmExtendFrameIntoClientArea 找回（1px 玻璃边即可触发阴影/圆角）。
/// 等价于旧 crate window-shadows 的实现，但它停留在 raw-window-handle 0.5，与 Tauri v2（0.6）不兼容。
#[cfg(target_os = "windows")]
fn enable_window_shadow(win: &tauri::Window) {
    use raw_window_handle::HasWindowHandle;
    let Ok(handle) = win.window_handle() else { return; };
    let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else { return; };
    let margins = windows_sys::Win32::UI::Controls::MARGINS {
        cxLeftWidth: 1,
        cxRightWidth: 1,
        cyTopHeight: 1,
        cyBottomHeight: 1,
    };
    unsafe {
        let _ = windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea(h.hwnd.get() as _, &margins);
    }
}

/// Windows æ­£å¸¸åº”ç”¨çª—å£ä½åºï¼Œä¿è¯ Alt+Tab å’Œ Win+å·¦/å³ ç›²åœ†ä¸­èƒ½çœ‹åˆ°å¹¶æ“ä½œä¸»çª—å£ã€‚
#[cfg(target_os = "windows")]
fn force_app_window(win: &tauri::Window) {
    use raw_window_handle::HasWindowHandle;
    use windows_sys::Win32::Foundation::HWND;
    let _ = win.set_skip_taskbar(false);
    let Ok(handle) = win.window_handle() else { return; };
    let raw_window_handle::RawWindowHandle::Win32(h) = handle.as_raw() else { return; };
    use windows_sys::Win32::UI::WindowsAndMessaging as wm;
    unsafe {
        let hwnd = h.hwnd.get() as HWND;
        let style = wm::GetWindowLongPtrW(hwnd, wm::GWL_STYLE) as u32;
        let updated_style = style | wm::WS_THICKFRAME | wm::WS_MAXIMIZEBOX | wm::WS_MINIMIZEBOX;
        if updated_style != style {
            let _ = wm::SetWindowLongPtrW(hwnd, wm::GWL_STYLE, updated_style as isize);
        }

        let ex_style = wm::GetWindowLongPtrW(hwnd, wm::GWL_EXSTYLE) as u32;
        let updated_ex = (ex_style | wm::WS_EX_APPWINDOW) & !wm::WS_EX_TOOLWINDOW;
        if updated_ex != ex_style {
            let _ = wm::SetWindowLongPtrW(hwnd, wm::GWL_EXSTYLE, updated_ex as isize);
        }

        let _ = wm::SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            wm::SWP_NOMOVE | wm::SWP_NOSIZE | wm::SWP_NOZORDER | wm::SWP_NOACTIVATE | wm::SWP_FRAMECHANGED,
        );
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .on_window_event(|window, event| {
            // WebView2 在 webview 内容被点击前不主动取得键盘焦点，导致 F11 等 DOM 快捷键首次无效。
            // 窗口重新获得焦点时把焦点还给 webview，保证无需先点击窗口内容即可按 F11 全屏。
            // 不能在这里再对窗口自身调用 set_focus()：任务栏点击恢复窗口时会造成焦点事件循环，
            // 表现为窗口唤不出来且界面卡顿。
            if let tauri::WindowEvent::Focused(true) = event {
                if window.label() == "main" {
                    let _ = window.unminimize();
                    if let Some(webview) = window.app_handle().get_webview("main") {
                        let _ = webview.set_focus();
                    }
                }
            }
            #[cfg(not(target_os = "android"))]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 关闭按钮 → 隐藏到系统托盘，进程驻留为常驻守护（托盘「退出」才真正退出）。
                // 备份动作移到托盘菜单 quit 分支统一执行。
                api.prevent_close();
                hide_to_tray(window);
            }
            // 最小化/还原时都会触发 Resized。最小化一律交给系统（任务栏图标 / Win+D /
            // Win+Down 的正常最小化/还原）；winMin 按钮隐藏到托盘走 hide_main_window 命令，
            // 不经过这里。旧实现把"最小化"一律 hide 到托盘，导致点击任务栏图标把窗口藏进
            // 托盘，且 get_webview_window("main") 在存在 browser-* 子 webview 时返回 None，
            // 托盘「显示 AlpeHuez」无法唤回，只能重启。这里只处理启动初期的恢复。
            #[cfg(not(target_os = "android"))]
            if let tauri::WindowEvent::Resized(_) = event {
                if window.is_minimized().unwrap_or(false) {
                    let startup = STARTED_AT
                        .get()
                        .map(|t| t.elapsed() < Duration::from_secs(8))
                        .unwrap_or(false);
                    if startup {
                        // 启动初期：Windows 会恢复上次会话的最小化状态，直接隐藏会把窗口
                        // 藏进托盘，表现为"启动后看不到界面"。改为还原显示。
                        let _ = window.unminimize();
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::read_json,
            commands::write_json,
            commands::read_software,
            commands::write_software,
            commands::create_folder,
            commands::git_status,
            commands::git_log,
            commands::git_push,
            commands::sys_stats,
            commands::run_script,
            commands::verify_password,
            commands::change_password,
            commands::get_myfiles_password,
            commands::set_myfiles_password,
            commands::has_access_password,
            commands::get_access_email,
            commands::set_access_email,
            commands::setup_access,
            commands::request_password_recovery,
            commands::open_password_recovery,
            commands::verify_recovery_code,
            commands::reset_password,
            commands::get_download_config,
            commands::set_download_config,
            commands::download_file,
            commands::open_in_fdm,
            commands::save_bg_image,
            commands::get_bg_config,
            commands::set_bg_config,
            commands::save_bookmarks_export,
            commands::fetch_portal_links,
            commands::send_feedback_email,
            commands::get_wechat_qr,
            commands::get_browser_config,
            commands::open_url,
            commands::open_url_scheme,
            commands::read_clipboard,
            commands::open_dev_panel,
            commands::open_internal_page,
            commands::activate_internal_page,
            commands::layout_internal_pages,
            commands::focus_internal_page,
            commands::close_internal_page,
            commands::go_back_internal_page,
            commands::go_forward_internal_page,
            commands::reload_internal_page,
            commands::get_app_version,
            commands::list_internal_pages,
            commands::set_internal_page_visible,
            commands::discard_internal_page,
            commands::mark_first_run,
            commands::list_workspaces,
            commands::get_active_workspace,
            commands::set_active_workspace,
            commands::create_workspace,
            commands::update_workspace,
            commands::delete_workspace,
            commands::get_workspace_links,
            commands::save_workspace_links,
            commands::get_app_config,
            commands::set_app_config,
            commands::hf_backup,
            commands::backup_history,
            commands::hf_test_connection,
            commands::webdav_backup,
            commands::webdav_test_connection,
            commands::set_webdav_password,
            commands::webdav_password_set,
            commands::backup_set_local_state,
            commands::restore_backup,
            commands::save_daily_note,
            commands::list_daily_notes,
            commands::delete_daily_note,
            commands::hide_main_window,
            commands::launch_note_app,
            commands::pick_folder,
            commands::pick_note_app,
            commands::open_in_explorer,
            commands::save_notes_export,
        ]);

    // Alt+A 全局召唤：隐藏时显示并聚焦，可见时隐藏。Android 无全局快捷键，且该插件在移动端为空壳。
    #[cfg(not(target_os = "android"))]
    let builder = builder.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_shortcuts(["alt+a"])
            .expect("invalid global shortcut alt+a")
            .with_handler(|app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    if let Some(window) = main_window(app) {
                        if window.is_visible().unwrap_or(true) {
                            hide_to_tray(&window);
                        } else {
                            show_main(app);
                        }
                    }
                }
            })
            .build(),
    );

    let builder = builder
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .register_uri_scheme_protocol("nav", preview::handler)
        .setup(|app| {
            let _ = STARTED_AT.set(Instant::now());
            // 单实例：第二次启动直接退出，避免双进程争抢全局快捷键与 WebView2 数据目录
            // 导致窗口无法显示（用户曾因此看到"启动无界面，只能 Alt+A 唤出"）。
            #[cfg(not(target_os = "android"))]
            {
                if let Ok(dir) = app.path().app_data_dir() {
                    if let Ok(lock_file) = std::fs::OpenOptions::new()
                        .create(true)
                        .write(true)
                        .open(dir.join("instance.lock"))
                    {
                        if lock_file.try_lock().is_err() {
                            // 已有驻留实例在运行：写入 show_request 标记让其唤出窗口，本实例退出。
                            let _ = std::fs::write(dir.join("show_request"), b"1");
                            std::process::exit(0);
                        }
                        // 保持文件打开以持锁整个进程生命周期
                        let _ = Box::leak(Box::new(lock_file));
                    }
                }
            }
            // 磨砂玻璃（Windows）：Acrylic 让桌面壁纸透出；找回无边框透明窗口的系统阴影。
            #[cfg(target_os = "windows")]
            {
                if let Some(win) = main_window(app.app_handle()) {
                    force_app_window(&win);
                    let _ = window_vibrancy::apply_acrylic(&win, Some((35, 35, 50, 96)));
                    enable_window_shadow(&win);
                }
            }
            // Android：repo_root 指向应用私有 webroot（编译期 Windows 路径在移动端不存在）。
            #[cfg(target_os = "android")]
            {
                if let Ok(dir) = app.path().app_data_dir() {
                    init_repo_root(dir.join("webroot"));
                }
            }
            // 桌面打包版（本机没有仓库目录）：把嵌入的网页资源播种到 app_data_dir/webroot
            // 并作为仓库根。开发模式（仓库在磁盘上）保持编译期路径，支持网页资源实时编辑。
            #[cfg(not(target_os = "android"))]
            {
                let disk_root = Path::new(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .expect("src-tauri 应有父目录");
                if !disk_root.join("index.html").exists() {
                    if let Ok(dir) = app.path().app_data_dir() {
                        let webroot = dir.join("webroot");
                        preview::materialize(&webroot);
                        init_repo_root(webroot);
                    }
                }
            }
            velometer::spawn(app.handle().clone());
            status::spawn(app.handle().clone());
            // 上次退出遗留的自动备份推迟到启动后台线程执行（不阻塞启动）。
            commands::run_pending_backup_at_launch(app.handle());
            // 窗口 visible:true 启动即显示；关闭窗口驻留后台（CloseRequested → hide），Alt+A 呼出。
            // Android 无全局快捷键可唤回，窗口必须始终显示。
            #[cfg(target_os = "android")]
            {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                }
            }
            // 启动后把焦点交给主 webview，避免首次使用 F11 前必须点击窗口内容（仅桌面）。
            // 启动 watchdog：前 12 秒内若用户未主动隐藏但窗口不可见或仍是最小化
            // （WebView2 初始化失败、启动被最小化状态干扰等），强制还原显示并聚焦。
            #[cfg(not(target_os = "android"))]
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    let deadline = Instant::now() + Duration::from_secs(12);
                    loop {
                        let now = Instant::now();
                        if now >= deadline {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(1200));
                        if let Some(win) = main_window(&handle) {
                            let visible = win.is_visible().unwrap_or(false);
                            let minimized = win.is_minimized().unwrap_or(false);
                            if !USER_HIDDEN.load(Ordering::Relaxed) && (!visible || minimized) {
                                let _ = win.unminimize();
                                let _ = win.show();
                                let _ = win.set_focus();
                            }
                        }
                        if let Some(webview) = handle.get_webview("main") {
                            let _ = webview.set_focus();
                        }
                    }
                });
            }
            // 常驻模式：监听第二次启动写下的 show_request 标记，把隐藏中的主窗口唤出。
            #[cfg(not(target_os = "android"))]
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    if let Ok(dir) = handle.path().app_data_dir() {
                        let marker = dir.join("show_request");
                        loop {
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            if marker.exists() {
                                let _ = std::fs::remove_file(&marker);
                                show_main(&handle);
                            }
                        }
                    }
                });
            }
            // 系统托盘（常驻守护）：左键单击显示窗口，右键菜单 显示/隐藏/退出。
            // 退出时才执行自动备份，关闭窗口只隐藏不备份（用户可能只是暂时收起）。
            #[cfg(not(target_os = "android"))]
            {
                // 托盘菜单固定中文（用户偏好；不跟随 app_lang，避免配置库与前端语言不同步时变英文）。
                let show_i = MenuItem::with_id(app, "show", "显示 AlpeHuez", true, None::<&str>)?;
                let hide_i = MenuItem::with_id(app, "hide", "隐藏窗口", true, None::<&str>)?;
                let quit_i = MenuItem::with_id(app, "quit", "退出 AlpeHuez", true, None::<&str>)?;
                let sep = PredefinedMenuItem::separator(app)?;
                let menu = Menu::with_items(app, &[&show_i, &hide_i, &sep, &quit_i])?;

                let mut tray = TrayIconBuilder::new()
                    .menu(&menu)
                    .icon(app.default_window_icon().expect("缺少应用图标").clone())
                    .tooltip("AlpeHuez")
                    .show_menu_on_left_click(false);
                tray = tray.on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => show_main(app),
                    "hide" => {
                        if let Some(win) = main_window(app) {
                            hide_to_tray(&win);
                        }
                    }
                    "quit" => {
                        let handle = app.clone();
                        std::thread::spawn(move || {
                            // 只写待备份标记，立即退出（备份推迟到下次启动执行）。
                            // 用 app.exit 而非 process::exit：优雅销毁 WebView2，
                            // 避免残留 msedgewebview2 子进程锁住 profile 数据目录，
                            // 否则下次启动窗口可能空白/不显示。
                            commands::mark_backup_for_launch(&handle);
                            handle.exit(0);
                        });
                    }
                    _ => {}
                });
                tray = tray.on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main(tray.app_handle());
                    }
                });
                tray.build(app)?;
            }
            Ok(())
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
