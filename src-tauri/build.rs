use std::path::Path;

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "read_json",
                "write_json",
                "read_software",
                "write_software",
                "create_folder",
                "git_status",
                "git_log",
                "git_push",
                "sys_stats",
                "run_script",
                "verify_password",
                "change_password",
                "save_bg_image",
                "get_bg_config",
                "set_bg_config",
                "save_feedback",
                "get_feedback",
                "get_wechat_qr",
                "get_browser_config",
                "open_url",
                "open_url_scheme",
                "read_clipboard",
                "open_dev_panel",
                "open_internal_page",
                "activate_internal_page",
                "layout_internal_pages",
                "focus_internal_page",
                "close_internal_page",
                "go_back_internal_page",
                "get_app_version",
                "list_internal_pages",
                "set_internal_page_visible",
                "list_workspaces",
                "get_active_workspace",
                "set_active_workspace",
                "create_workspace",
                "update_workspace",
                "delete_workspace",
                "get_workspace_links",
                "save_workspace_links",
                "get_app_config",
                "set_app_config",
            ]),
        ),
    )
    .expect("tauri-build failed");

    // Android：把门户文件暂存到 android-webroot，供 preview.rs 的 include_dir! 编译期嵌入。
    // 仅拷贝移动端真正用到的文件，避开 .git / releases / src-tauri / 大报告等。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        stage_webroot();
    }
}

fn stage_webroot() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("src-tauri 应有父目录");
    let out = manifest.join("android-webroot");
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("创建 android-webroot 失败");

    let stage = |rel: &str| {
        let src = root.join(rel);
        let dst = out.join(rel);
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else if src.exists() {
            std::fs::create_dir_all(dst.parent().expect("目标应有父目录"))
                .expect("创建暂存目录失败");
            std::fs::copy(&src, &dst).expect("暂存文件失败");
        } else {
            panic!("android-webroot 暂存缺失文件: {rel}");
        }
    };

    for f in [
        "index.html",
        "links.json",
        "tailwind.min.js",
        "1109770.jpg",
        "github-profile.jpg",
    ] {
        stage(f);
    }
    stage("icons");
    stage("myfiles/data.json");
    stage("myfiles/index.html");
    stage("myfiles/explorer.js");
    stage("myfiles/explorer.css");
    stage("myfiles/softwares");
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("创建暂存目录失败");
    for entry in std::fs::read_dir(src).expect("读取目录失败") {
        let entry = entry.expect("读取目录项失败");
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to);
        } else {
            std::fs::copy(&from, &to).expect("暂存文件失败");
        }
    }
}
