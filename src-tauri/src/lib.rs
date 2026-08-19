mod commands;
mod db;
mod preview;
mod status;
mod velometer;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tauri::Manager;
#[cfg(not(target_os = "android"))]
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
#[cfg(not(target_os = "android"))]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(not(target_os = "android"))]
use tauri_plugin_global_shortcut::ShortcutState;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

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

/// 显示并聚焦主窗口。Windows 上直接 show() 有时不置顶，需 unminimize + set_focus 兜底。
#[cfg(not(target_os = "android"))]
fn show_main(app: &tauri::AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .on_window_event(|window, event| {
            // WebView2 在 webview 内容被点击前不主动取得键盘焦点，导致 F11 等 DOM 快捷键首次无效。
            // 窗口重新获得焦点时把焦点还给 webview，保证无需先点击窗口内容即可按 F11 全屏。
            if let tauri::WindowEvent::Focused(true) = event {
                let _ = window.set_focus();
            }
            #[cfg(not(target_os = "android"))]
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 关闭按钮 → 隐藏到系统托盘，进程驻留为常驻守护（托盘「退出」才真正退出）。
                // 备份动作移到托盘菜单 quit 分支统一执行。
                api.prevent_close();
                let _ = window.hide();
            }
            // 最小化按钮 → 同样隐藏到系统托盘（驻留后台继续运行）。
            // Resized 在最小化/还原时都会触发，按 is_minimized 状态区分。
            #[cfg(not(target_os = "android"))]
            if let tauri::WindowEvent::Resized(_) = event {
                if window.is_minimized().unwrap_or(false) {
                    let _ = window.hide();
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
            commands::verify_recovery_code,
            commands::reset_password,
            commands::get_download_config,
            commands::set_download_config,
            commands::download_file,
            commands::open_in_fdm,
            commands::save_bg_image,
            commands::get_bg_config,
            commands::set_bg_config,
            commands::save_feedback,
            commands::get_feedback,
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
            commands::get_app_version,
            commands::list_internal_pages,
            commands::set_internal_page_visible,
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
            commands::hf_test_connection,
            commands::save_daily_note,
        ]);

    // Alt+A 全局召唤：隐藏时显示并聚焦，可见时隐藏。Android 无全局快捷键，且该插件在移动端为空壳。
    #[cfg(not(target_os = "android"))]
    let builder = builder.plugin(
        tauri_plugin_global_shortcut::Builder::new()
            .with_shortcuts(["alt+a"])
            .expect("invalid global shortcut alt+a")
            .with_handler(|app, _shortcut, event| {
                if event.state == ShortcutState::Pressed {
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(true) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
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
            // 若窗口启动时不可见（如残留 WebView2 进程干扰初始化），强制 show() 兜底。
            #[cfg(not(target_os = "android"))]
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if let Some(win) = handle.get_webview_window("main") {
                        if !win.is_visible().unwrap_or(false) {
                            let _ = win.show();
                        }
                    }
                    if let Some(webview) = handle.get_webview("main") {
                        let _ = webview.set_focus();
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
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = win.hide();
                        }
                    }
                    "quit" => {
                        let handle = app.clone();
                        std::thread::spawn(move || {
                            // 只写待备份标记，立即硬退出（避免网络 push 与 WebView2 优雅销毁阻塞退出）；
                            // 备份推迟到下次启动。
                            commands::mark_backup_for_launch(&handle);
                            std::process::exit(0);
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
