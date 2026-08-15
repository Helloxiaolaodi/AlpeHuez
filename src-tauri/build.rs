fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "read_json",
                "write_json",
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
                "list_browsers",
                "get_browser_config",
                "set_browser_config",
                "open_url",
                "open_dev_panel",
            ]),
        ),
    )
    .expect("tauri-build failed");
}
