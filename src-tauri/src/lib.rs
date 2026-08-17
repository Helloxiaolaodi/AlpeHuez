mod commands;
mod db;
mod preview;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

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
            commands::open_dev_panel,
            commands::open_internal_page,
            commands::activate_internal_page,
            commands::layout_internal_pages,
            commands::focus_internal_page,
            commands::close_internal_page,
            commands::go_back_internal_page,
            commands::get_app_version,
            commands::list_internal_pages,
        ])
        .register_uri_scheme_protocol("nav", preview::handler)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
