mod commands;
mod db;
mod preview;
mod status;
mod velometer;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use tauri::Manager;
use tauri_plugin_global_shortcut::ShortcutState;

/// 仓库根目录：src-tauri 的上一级（编译期常量，本机构建本机运行）。
pub fn repo_root() -> &'static Path {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let p = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("src-tauri 应有父目录");
        p.canonicalize().unwrap_or_else(|_| p.to_path_buf())
    })
}

pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            // WebView2 在 webview 内容被点击前不主动取得键盘焦点，导致 F11 等 DOM 快捷键首次无效。
            // 窗口重新获得焦点时把焦点还给 webview，保证无需先点击窗口内容即可按 F11 全屏。
            if let tauri::WindowEvent::Focused(true) = event {
                let _ = window.set_focus();
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
            commands::list_browsers,
            commands::get_browser_config,
            commands::set_browser_config,
            commands::open_url,
            commands::open_url_scheme,
            commands::open_dev_panel,
            commands::open_internal_page,
            commands::activate_internal_page,
            commands::layout_internal_pages,
            commands::focus_internal_page,
            commands::close_internal_page,
            commands::go_back_internal_page,
            commands::get_app_version,
            commands::list_internal_pages,
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
        ])
        // Alt+Space 全局召唤：隐藏时显示并聚焦，可见时隐藏。
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts(["alt+space"])
                .expect("invalid global shortcut alt+space")
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
        )
        .register_uri_scheme_protocol("nav", preview::handler)
        .setup(|app| {
            velometer::spawn(app.handle().clone());
            status::spawn(app.handle().clone());
            // 启动后把焦点交给主 webview，避免首次使用 F11 前必须点击窗口内容。
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(300));
                if let Some(webview) = handle.get_webview("main") {
                    let _ = webview.set_focus();
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
