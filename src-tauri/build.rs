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
                "get_myfiles_password",
                "set_myfiles_password",
                "has_access_password",
                "get_access_email",
                "set_access_email",
                "setup_access",
                "request_password_recovery",
                "open_password_recovery",
                "verify_recovery_code",
                "reset_password",
                "get_download_config",
                "set_download_config",
                "download_file",
                "open_in_fdm",
                "save_bg_image",
                "get_bg_config",
                "set_bg_config",
                "save_bookmarks_export",
                "fetch_portal_links",
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
                "go_forward_internal_page",
                "reload_internal_page",
                "get_app_version",
                "list_internal_pages",
                "set_internal_page_visible",
                "discard_internal_page",
                "mark_first_run",
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
                "hf_backup",
                "backup_history",
                "mark_backup_for_launch",
                "run_pending_backup_at_launch",
                "hf_test_connection",
                "webdav_backup",
                "webdav_test_connection",
                "set_webdav_password",
                "webdav_password_set",
                "backup_set_local_state",
                "restore_backup",
                "save_daily_note",
            ]),
        ),
    )
    .expect("tauri-build failed");

    // 把网页资源暂存到独立目录，供 preview.rs 的 include_dir! 编译期嵌入：
    // 打包版（无仓库目录）运行时 nav:// 与读写命令回退到这些嵌入资源。
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("android") {
        stage_webroot("android-webroot", &android_files());
    } else {
        stage_webroot("webroot", &desktop_files());
        trim_myfiles_data("webroot");
        blank_webroot_content("webroot");
    }
}

fn android_files() -> Vec<&'static str> {
    vec![
        "index.html",
        "links.json",
        "tailwind.min.js",
        "github-profile.jpg",
        "icons",
        "fonts",
        "myfiles/data.json",
        "myfiles/index.html",
        "myfiles/explorer.js",
        "myfiles/explorer.css",
        "myfiles/softwares",
    ]
}

fn desktop_files() -> Vec<&'static str> {
    // 打包版不携带开发者任何个人内容：仅应用外壳 + 空白启动数据。
    // 个人内容（Portal 书签、软件清单、github-profile 头像、favicon 缓存、
    // 个人报告目录 galibierhub/lucuro 等）一律不进包，stage 后由
    // trim_myfiles_data / blank_webroot_content 写为空白启动数据。
    vec![
        "index.html",
        "links.json",
        "tailwind.min.js",
        "fonts",
        "myfiles/data.json",
        "myfiles/index.html",
        "myfiles/explorer.js",
        "myfiles/explorer.css",
        "myfiles/softwares",
        "myfiles/login.html",
        "panel",
    ]
}

fn stage_webroot(out_name: &str, files: &[&str]) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("src-tauri 应有父目录");
    let out = manifest.join(out_name);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(&out).expect("创建暂存目录失败");

    for rel in files {
        let src = root.join(rel);
        let dst = out.join(rel);
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else if src.exists() {
            std::fs::create_dir_all(dst.parent().expect("目标应有父目录"))
                .expect("创建暂存目录失败");
            std::fs::copy(&src, &dst).expect("暂存文件失败");
        } else {
            panic!("暂存缺失文件: {rel}");
        }
        println!("cargo:rerun-if-changed={}", src.display());
    }
}

/// 打包版 My Files 不公开任何文件夹：个人报告目录（targetc/global-oral 等）不进包，
/// softwares 已从 My Files 界面移除（只在左侧栏 Software 入口打开）。
fn trim_myfiles_data(out_name: &str) {
    use serde_json::Value;
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = manifest.parent().expect("src-tauri 应有父目录").join("myfiles/data.json");
    let dst = manifest.join(out_name).join("myfiles/data.json");
    let Ok(text) = std::fs::read_to_string(&src) else { return };
    let Ok(mut v) = serde_json::from_str::<Value>(&text) else { return };
    if let Some(obj) = v.as_object_mut() {
        obj.insert("folders".into(), Value::Array(vec![]));
    }
    let _ = std::fs::write(&dst, serde_json::to_string_pretty(&v).expect("序列化 data.json") + "\n");
}

/// 打包版不携带开发者现有内容：Portal 链接与软件清单置为空白启动数据。
fn blank_webroot_content(out_name: &str) {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out = manifest.join(out_name);

    let links_blank = r#"{
    "version": 1,
    "appName": "AlpeHuez",
    "exportTime": "",
    "appVersion": "",
    "icons": []
}
"#;
    let _ = std::fs::write(out.join("links.json"), links_blank);

    let soft_blank = r#"{
    "categories": [],
    "software": []
}
"#;
    let _ = std::fs::write(out.join("myfiles").join("softwares").join("software-data.json"), soft_blank);
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
