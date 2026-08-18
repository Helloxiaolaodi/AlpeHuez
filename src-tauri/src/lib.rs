mod commands;
mod db;
mod preview;
mod status;
mod velometer;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tauri::Manager;
#[cfg(not(target_os = "android"))]
use tauri_plugin_global_shortcut::ShortcutState;

static ROOT: OnceLock<PathBuf> = OnceLock::new();

/// 设置仓库根目录（Android 上由 setup 指向应用私有 webroot，桌面版无需调用）。
pub fn init_repo_root(path: PathBuf) {
    let _ = ROOT.set(path);
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
                api.prevent_close();
                let app = window.app_handle().clone();
                let win = window.clone();
                std::thread::spawn(move || {
                    commands::auto_backup_on_close(&app);
                    let _ = win.hide();
                });
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
        .register_uri_scheme_protocol("nav", preview::handler)
        .setup(|app| {
            // Android：repo_root 指向应用私有 webroot（编译期 Windows 路径在移动端不存在）。
            #[cfg(target_os = "android")]
            {
                if let Ok(dir) = app.path().app_data_dir() {
                    init_repo_root(dir.join("webroot"));
                }
            }
            velometer::spawn(app.handle().clone());
            status::spawn(app.handle().clone());
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
            Ok(())
        });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
