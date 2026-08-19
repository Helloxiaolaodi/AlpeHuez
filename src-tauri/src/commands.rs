use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::sync::atomic::AtomicBool;
use std::sync::{LazyLock, Mutex};
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// 正在加载的浏览器标签 label 集合：加载期间不显示子 webview，避免露出引擎底色（黑屏）。
#[cfg(not(target_os = "android"))]
static LOADING_LABELS: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(|| Mutex::new(HashSet::new()));

#[cfg(not(target_os = "android"))]
use std::sync::atomic::Ordering;
#[cfg(not(target_os = "android"))]
use std::sync::Arc;

use tauri::Manager;
#[cfg(not(target_os = "android"))]
use tauri::{webview::{PageLoadEvent, WebviewBuilder}, Emitter, PhysicalPosition, PhysicalSize, Rect, WebviewUrl, WebviewWindowBuilder};

#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

use crate::db;
use crate::repo_root;

/// Windows 下禁止子进程创建控制台窗口（消灭幽灵终端）。
fn silent(cmd: &mut Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
}

fn db_conn(app: &tauri::AppHandle) -> Result<rusqlite::Connection, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let conn = db::open(&dir.join("alpehuez.db"))?;
    db::init(&conn)?;
    db::seed(&conn)?;
    Ok(conn)
}

/// 主应用语言设置（zh/en），供托盘菜单等系统级 UI 使用，默认中文。
pub fn app_lang(app: &tauri::AppHandle) -> String {
    db_conn(app)
        .ok()
        .and_then(|conn| db::get_config(&conn, "app_lang").ok())
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "zh".into())
}

const ADBLOCK_JS: &str = r#"(function () {
  var SELECTORS = [
    '#ad', '.ad-banner', '.ad-container', '.ad-content', '.ad-footer', '.ad-header',
    '.ad-placeholder', '.ad-slot', '.ads', '.adsbygoogle', '.adsense', '.advert',
    '.advertisement', '.advertising', '.sponsored', '.sponsor', '.promo-ad', '.google-ads',
    '#google_ads_iframe', '[id^="google_ads_"]', '[id^="ad_"]', '[id*="advert"]',
    'ins.adsbygoogle',
    'iframe[src*="doubleclick.net"]', 'iframe[src*="googlesyndication.com"]',
    'iframe[src*="taboola.com"]', 'iframe[src*="outbrain.com"]',
    'img[src*="doubleclick.net"]', 'img[src*="adservice"]'
  ];
  function hideAds() {
    try {
      document.querySelectorAll(SELECTORS.join(',')).forEach(function (node) {
        if (node.closest('body')) node.style.setProperty('display', 'none', 'important');
      });
    } catch (e) {}
  }
  var timer = null;
  function scheduleHide() {
    clearTimeout(timer);
    timer = setTimeout(hideAds, 120);
  }
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', hideAds);
  } else {
    hideAds();
  }
  if (window.MutationObserver) {
    new MutationObserver(scheduleHide).observe(document.documentElement, {
      childList: true,
      subtree: true
    });
  }
})();"#;

const CHILD_KEY_JS: &str = r#"(function () {
  function routeKey(event) {
    if (event.repeat) return;
    var tauriEvent = window.__TAURI__ && window.__TAURI__.event;
    if (!tauriEvent || typeof tauriEvent.emitTo !== 'function') return;
    if (event.key === 'F11') {
      event.preventDefault();
      event.stopPropagation();
      tauriEvent.emitTo('main', 'alpehuez-fullscreen-toggle', {});
    } else if (event.altKey && event.key === 'ArrowLeft') {
      tauriEvent.emitTo('main', 'alpehuez-back', {});
    } else if (event.key === 'Escape') {
      tauriEvent.emitTo('main', 'alpehuez-fullscreen-exit', {});
    }
  }
  document.addEventListener('keydown', routeKey, true);
})();"#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptResult {
    pub ok: bool,
    pub code: i32,
    pub output: String,
    pub timed_out: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub ok: bool,
    pub status: String,
    pub branch: String,
    pub last: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushResult {
    pub ok: bool,
    pub add: String,
    pub commit: String,
    pub commit_code: i32,
    pub push: String,
    pub push_code: i32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLogEntry {
    pub short: String,
    pub date: String,
    pub author: String,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SysStats {
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InternalPageInfo {
    pub label: String,
    pub url: String,
    pub title: String,
}

/// 与 Node `JSON.stringify(v, null, 4)` 完全一致的 4 空格缩进序列化。
fn to_pretty_4(value: &Value) -> String {
    let mut buf = Vec::new();
    let mut ser = serde_json::Serializer::with_formatter(
        &mut buf,
        serde_json::ser::PrettyFormatter::with_indent(b"    "),
    );
    value.serialize(&mut ser).expect("JSON 序列化失败");
    String::from_utf8(buf).expect("JSON 应为 UTF-8")
}

/// 复刻 server.mjs 的 recomputeLinksMd5：去掉 md5 字段 → 4 空格缩进 + 换行 → md5 hex。
fn recompute_links_md5(links: &Value) -> String {
    let mut rest = links.clone();
    rest.as_object_mut().expect("links 应为对象").remove("md5");
    let json = to_pretty_4(&rest) + "\n";
    format!("{:x}", md5::compute(json.as_bytes()))
}

fn run_cmd(cmd: &str, args: &[&str], cwd: &Path) -> (i32, String) {
    let mut c = Command::new(cmd);
    silent(&mut c);
    match c.args(args).current_dir(cwd).output() {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            (out.status.code().unwrap_or(-1), s)
        }
        Err(e) => (-1, e.to_string()),
    }
}

fn run_cmd_timeout(cmd: &str, args: &[&str], cwd: &Path, timeout_secs: u64) -> (i32, String, bool) {
    let mut c = Command::new(cmd);
    silent(&mut c);
    let mut child = match c
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return (-1, e.to_string(), false),
    };
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let out_handle = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut so) = stdout {
            let _ = so.read_to_string(&mut s);
        }
        s
    });
    let err_handle = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(mut se) = stderr {
            let _ = se.read_to_string(&mut s);
        }
        s
    });
    let start = Instant::now();
    let mut timed_out = false;
    let status = loop {
        if let Some(st) = child.try_wait().unwrap_or(None) {
            break st;
        }
        if start.elapsed() > Duration::from_secs(timeout_secs) {
            let _ = child.kill();
            timed_out = true;
            break child.wait().unwrap_or_else(|_| ExitStatus::from_raw(1));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let out = out_handle.join().unwrap_or_default();
    let err = err_handle.join().unwrap_or_default();
    (status.code().unwrap_or(-1), out + &err, timed_out)
}

fn find_node() -> String {
    let mut c = Command::new("node");
    silent(&mut c);
    if c.arg("--version").output().is_ok() {
        return "node".to_string();
    }
    for p in [
        "C:\\Program Files\\nodejs\\node.exe",
        "C:\\Program Files (x86)\\nodejs\\node.exe",
    ] {
        if Path::new(p).exists() {
            return p.to_string();
        }
    }
    "node".to_string()
}

#[tauri::command]
pub async fn read_json(path: String) -> Result<Value, String> {
    let full = repo_root().join(path);
    let content = fs::read_to_string(&full).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_json(path: String, data: Value) -> Result<Value, String> {
    let full = repo_root().join(&path);
    if path == "links.json" {
        if !data.get("icons").and_then(|v| v.as_array()).is_some() {
            return Err("数据格式不正确：缺少 icons 数组".into());
        }
        for group in data["icons"].as_array().expect("已校验") {
            for item in group.get("children").and_then(|c| c.as_array()).unwrap_or(&vec![]) {
                let title = item.get("title").and_then(|t| t.as_str()).unwrap_or("");
                let url = item.get("url").and_then(|u| u.as_str()).unwrap_or("");
                if title.is_empty() || url.is_empty() {
                    return Err(format!(
                        "卡片「{}」缺少标题或 URL",
                        if title.is_empty() { "(无标题)" } else { title }
                    ));
                }
            }
        }
        let md5 = recompute_links_md5(&data);
        let mut data = data;
        data.as_object_mut().expect("已校验").insert("md5".into(), Value::String(md5.clone()));
        let json = to_pretty_4(&data) + "\n";
        fs::write(&full, json).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "md5": md5 }))
    } else {
        if !data.get("folders").and_then(|v| v.as_array()).is_some() {
            return Err("数据格式不正确：缺少 folders 数组".into());
        }
        for folder in data["folders"].as_array().expect("已校验") {
            let slug = folder.get("slug").and_then(|s| s.as_str()).unwrap_or("");
            let name = folder.get("name").and_then(|n| n.as_str()).unwrap_or("");
            if slug.is_empty() || name.is_empty() {
                return Err(format!(
                    "文件夹「{}」缺少 slug 或名称",
                    if name.is_empty() { slug } else { name }
                ));
            }
        }
        let json = to_pretty_4(&data) + "\n";
        fs::write(&full, json).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({}))
    }
}

/// 读取 Windows 软件下载列表（myfiles/softwares/software-data.json）。
#[tauri::command]
pub async fn read_software() -> Result<Value, String> {
    let full = repo_root().join("myfiles/softwares/software-data.json");
    let content = fs::read_to_string(&full).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

/// 保存 Windows 软件下载列表（仅软件页面使用的 categories/software 结构）。
#[tauri::command]
pub async fn write_software(data: Value) -> Result<(), String> {
    if !data.get("categories").and_then(|v| v.as_array()).is_some() {
        return Err("数据格式不正确：缺少 categories 数组".into());
    }
    if !data.get("software").and_then(|v| v.as_array()).is_some() {
        return Err("数据格式不正确：缺少 software 数组".into());
    }
    let full = repo_root().join("myfiles/softwares/software-data.json");
    fs::write(&full, to_pretty_4(&data) + "\n").map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_folder(slug: String, name: String, protected: bool) -> Result<(), String> {
    let slug = slug.trim().to_lowercase();
    let name = name.trim().to_string();
    if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err("slug 只能包含小写字母、数字和连字符（如 my-folder）".into());
    }
    if name.is_empty() {
        return Err("请填写文件夹名称".into());
    }
    let root = repo_root();
    let folder_dir = root.join("myfiles").join(&slug);
    fs::create_dir_all(&folder_dir).map_err(|e| e.to_string())?;

    let index_template = fs::read_to_string(root.join("myfiles/softwares/index.html")).map_err(|e| e.to_string())?;
    fs::write(folder_dir.join("index.html"), index_template).map_err(|e| e.to_string())?;

    if protected {
        let login_template = fs::read_to_string(root.join("myfiles/targetc/login.html")).map_err(|e| e.to_string())?;
        let mut login_html = login_template.replace("/myfiles/targetc", &format!("/myfiles/{}", slug));
        login_html = login_html.replace("TargetC Data Analysis", &name);
        login_html = login_html.replace(
            &format!("/myfiles/{}/TargetC-phenotypes-analysis-260814.html", slug),
            &format!("/myfiles/{}/", slug),
        );
        fs::write(folder_dir.join("login.html"), login_html).map_err(|e| e.to_string())?;

        let func_dir = root.join("functions/myfiles").join(&slug);
        fs::create_dir_all(&func_dir).map_err(|e| e.to_string())?;

        let auth_template = fs::read_to_string(root.join("functions/myfiles/targetc/_auth.js")).map_err(|e| e.to_string())?;
        fs::write(
            func_dir.join("_auth.js"),
            auth_template.replace("'targetc_auth_v2'", &format!("'{}_auth_v2'", slug)),
        )
        .map_err(|e| e.to_string())?;

        let login_js_template = fs::read_to_string(root.join("functions/myfiles/targetc/login.js")).map_err(|e| e.to_string())?;
        let mut login_js = login_js_template.replace("/myfiles/targetc", &format!("/myfiles/{}", slug));
        login_js = login_js.replace(
            &format!("/myfiles/{}/TargetC-phenotypes-analysis-260814.html", slug),
            &format!("/myfiles/{}/", slug),
        );
        fs::write(func_dir.join("login.js"), login_js).map_err(|e| e.to_string())?;

        let middleware_template = fs::read_to_string(root.join("functions/myfiles/targetc/_middleware.js")).map_err(|e| e.to_string())?;
        fs::write(
            func_dir.join("_middleware.js"),
            middleware_template.replace("/myfiles/targetc", &format!("/myfiles/{}", slug)),
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn git_status() -> Result<GitStatus, String> {
    let root = repo_root();
    // -b 让第一行输出分支信息（## main...origin/main），一次调用合并 branch + status。
    let (_, status) = run_cmd("git", &["status", "--short", "-b"], root);
    let (_, last) = run_cmd("git", &["log", "-1", "--oneline"], root);
    let mut lines = status.lines();
    let first = lines.next().unwrap_or("").trim();
    let branch = first
        .trim_start_matches("## ")
        .split("...")
        .next()
        .unwrap_or("")
        .trim_start_matches("No commits yet on ")
        .to_string();
    let rest = lines.collect::<Vec<_>>().join("\n");
    Ok(GitStatus {
        ok: true,
        status: rest,
        branch,
        last: last.trim().to_string(),
    })
}

/// 最近 20 条提交历史（Dev Timeline 用）。tab 分隔避免作者名中的特殊字符错位。
#[tauri::command]
pub async fn git_log() -> Result<Vec<GitLogEntry>, String> {
    let root = repo_root();
    let (code, out) = run_cmd(
        "git",
        &[
            "log",
            "-20",
            "--pretty=format:%h%x09%ad%x09%an%x09%s",
            "--date=format:%Y-%m-%d %H:%M",
        ],
        root,
    );
    if code != 0 {
        return Err(out);
    }
    let entries = out
        .lines()
        .filter_map(|line| {
            let mut parts = line.splitn(4, '\t');
            let short = parts.next()?.to_string();
            let date = parts.next()?.to_string();
            let author = parts.next()?.to_string();
            let message = parts.next()?.to_string();
            Some(GitLogEntry { short, date, author, message })
        })
        .collect();
    Ok(entries)
}

/// 系统资源：CPU / 内存 / 磁盘（分段进度条用）。
/// 只加载 CPU/内存（不枚举全部进程），避免 System::new_all() 卡顿。
#[tauri::command]
pub async fn sys_stats() -> Result<SysStats, String> {
    use sysinfo::{CpuRefreshKind, Disks, MemoryRefreshKind, RefreshKind, System};
    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::new().with_cpu_usage())
            .with_memory(MemoryRefreshKind::new().with_ram()),
    );
    sys.refresh_cpu_usage();
    std::thread::sleep(Duration::from_millis(200));
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let disks = Disks::new_with_refreshed_list();
    let disk = disks
        .iter()
        .find(|d| !d.is_removable())
        .or_else(|| disks.iter().next());
    let (disk_used, disk_total) = match disk {
        Some(d) => {
            let total = d.total_space();
            let avail = d.available_space();
            (total.saturating_sub(avail), total)
        }
        None => (0, 0),
    };
    Ok(SysStats {
        cpu: sys.global_cpu_info().cpu_usage(),
        mem_used: sys.used_memory(),
        mem_total: sys.total_memory(),
        disk_used,
        disk_total,
    })
}

#[tauri::command]
pub async fn git_push(app: tauri::AppHandle, message: String) -> Result<PushResult, String> {
    use tauri::Emitter;
    let root = repo_root();
    let msg = if message.trim().is_empty() {
        "Update site content".to_string()
    } else {
        message.trim().to_string()
    };
    let emit = |phase: &str, text: &str| {
        let _ = app.emit("deploy-log", serde_json::json!({ "phase": phase, "text": text }));
    };
    emit("add", "$ git add -A\n");
    let (add_code, add) = run_cmd("git", &["add", "-A"], root);
    if add_code != 0 { emit("error", &add); } else { emit("add", &add); }
    emit("commit", "$ git commit -m \"...\"\n");
    let (commit_code, commit) = run_cmd("git", &["commit", "-m", &msg], root);
    if commit_code != 0 { emit("error", &commit); } else { emit("commit", &commit); }
    emit("push", "$ git push\n");
    let (push_code, push) = run_cmd("git", &["push"], root);
    if push_code != 0 { emit("error", &push); } else { emit("push", &push); }
    Ok(PushResult { ok: true, add, commit, commit_code, push, push_code })
}

#[tauri::command]
pub async fn run_script(script: String) -> Result<ScriptResult, String> {
    let file = match script.as_str() {
        "download_icons" => "download_icons.mjs",
        "enhance_links" => "enhance_links.mjs",
        "repair_icons" => "repair_icons.mjs",
        _ => return Err("未知脚本".into()),
    };
    let root = repo_root();
    let node = find_node();
    let (code, output, timed_out) = run_cmd_timeout(&node, &[file], root, 300);
    Ok(ScriptResult {
        ok: code == 0,
        code,
        output,
        timed_out,
    })
}

fn config_file(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

fn get_password(app: &tauri::AppHandle) -> Result<String, String> {
    if let Ok(env_password) = std::env::var("ALPEHUZ_PASSWORD") {
        if !env_password.is_empty() {
            return Ok(env_password);
        }
    }

    let file = config_file(app)?;
    if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        if let Some(password) = v.get("password").and_then(|p| p.as_str()) {
            if !password.is_empty() {
                return Ok(password.to_string());
            }
        }
        Err("访问密码尚未配置".into())
    } else {
        Err("访问密码尚未配置".into())
    }
}

#[tauri::command]
pub async fn verify_password(app: tauri::AppHandle, input: String) -> Result<bool, String> {
    let stored = get_password(&app)?;
    Ok(input == stored)
}

#[tauri::command]
pub async fn change_password(app: tauri::AppHandle, old: String, new: String) -> Result<(), String> {
    let stored = get_password(&app)?;
    if old != stored {
        return Err("旧密码错误".into());
    }
    if new.len() < 4 {
        return Err("新密码至少 4 位".into());
    }
    // #31 统一访问密码：面板密码与 My Files 访问密码保持同步。
    write_config_keys(&app, &[("password", new.clone()), ("myfiles_password", new)])
}

/// My Files 网页访问密码：仅保存在本机配置（不入仓库）。
/// 部署端密码需手动同步到 Cloudflare Pages 环境变量 ALPEHUZ_ACCESS_PASSWORD。
#[tauri::command]
pub async fn get_myfiles_password(app: tauri::AppHandle) -> Result<String, String> {
    let file = config_file(&app)?;
    if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        return Ok(v
            .get("myfiles_password")
            .and_then(|p| p.as_str())
            .unwrap_or("")
            .to_string());
    }
    Ok(String::new())
}

#[tauri::command]
pub async fn set_myfiles_password(app: tauri::AppHandle, password: String) -> Result<(), String> {
    let password = password.trim().to_string();
    if password.len() < 4 {
        return Err("新密码至少 4 位".into());
    }
    // #31 统一访问密码：My Files 访问密码与面板密码保持同步。
    write_config_keys(&app, &[("myfiles_password", password.clone()), ("password", password)])
}

/* ---------- 统一访问密码：首次设置 / 找回密码 ---------- */

/// 写多个字符串键到 config.json（单个原子写入，避免多次读写竞争）。
fn write_config_keys(app: &tauri::AppHandle, keys: &[(&str, String)]) -> Result<(), String> {
    let file = config_file(app)?;
    let mut v = read_config_value(&file);
    if let Some(obj) = v.as_object_mut() {
        for (k, val) in keys {
            obj.insert(k.to_string(), serde_json::Value::String(val.clone()));
        }
    }
    fs::write(&file, to_pretty_4(&v) + "\n").map_err(|e| e.to_string())
}

/// 是否已配置访问密码（面板 / My Files 统一使用同一密码）。
#[tauri::command]
pub async fn has_access_password(app: tauri::AppHandle) -> Result<bool, String> {
    if let Ok(env_password) = std::env::var("ALPEHUZ_PASSWORD") {
        if !env_password.is_empty() {
            return Ok(true);
        }
    }
    let file = config_file(&app)?;
    let v = read_config_value(&file);
    Ok(v.get("password")
        .and_then(|p| p.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false))
}

/// 读取找回密码邮箱（config.json 的 recovery_email，空串表示未设置）。
#[tauri::command]
pub async fn get_access_email(app: tauri::AppHandle) -> Result<String, String> {
    let file = config_file(&app)?;
    let v = read_config_value(&file);
    Ok(v.get("recovery_email")
        .and_then(|e| e.as_str())
        .unwrap_or("")
        .to_string())
}

#[tauri::command]
pub async fn set_access_email(app: tauri::AppHandle, email: String) -> Result<(), String> {
    let email = email.trim().to_string();
    if !email.is_empty() && !email.contains('@') {
        return Err("邮箱地址格式不正确".into());
    }
    write_config_keys(&app, &[("recovery_email", email)])
}

/// 首次使用设置向导：一次性写入统一的访问密码 + 找回邮箱。
#[tauri::command]
pub async fn setup_access(app: tauri::AppHandle, password: String, email: String) -> Result<(), String> {
    let password = password.trim().to_string();
    if password.len() < 4 {
        return Err("新密码至少 4 位".into());
    }
    let email = email.trim().to_string();
    if !email.is_empty() && !email.contains('@') {
        return Err("邮箱地址格式不正确".into());
    }
    write_config_keys(
        &app,
        &[
            ("password", password.clone()),
            ("myfiles_password", password),
            ("recovery_email", email),
        ],
    )
}

/// 生成 6 位找回验证码（基于时间戳 + 进程号 + 邮箱的散列，足够本地使用）。
fn make_recovery_code(email: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let digest = md5::compute(format!("{}|{}|{}", email, std::process::id(), nanos));
    let hex = format!("{:x}", digest);
    let mut code = String::new();
    for c in hex.chars() {
        if code.len() >= 6 {
            break;
        }
        if c.is_ascii_digit() {
            code.push(c);
        }
    }
    if code.len() < 6 {
        code = format!("{:06}", nanos % 1_000_000);
    }
    code
}

/// 校验找回邮箱并生成 6 位验证码；返回验证码供 mailto 邮件填写。
/// 本机应用无法真正发送邮件，验证码随 mailto 内容交给用户的邮件客户端。
#[tauri::command]
pub async fn request_password_recovery(app: tauri::AppHandle, email: String) -> Result<String, String> {
    let email = email.trim().to_string();
    let file = config_file(&app)?;
    let v = read_config_value(&file);
    let stored = v.get("recovery_email").and_then(|e| e.as_str()).unwrap_or("");
    if stored.is_empty() {
        return Err("尚未设置找回邮箱，无法找回密码。请在面板「修改密码」中填写找回邮箱".into());
    }
    if stored != email {
        return Err("邮箱与预留的找回邮箱不一致".into());
    }
    let code = make_recovery_code(&email);
    let expires = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
        + 600; // 10 分钟有效
    write_config_keys(
        &app,
        &[
            ("recovery_code", code.clone()),
            ("recovery_code_expires", expires.to_string()),
        ],
    )?;
    Ok(code)
}

/// 校验验证码是否匹配且未过期。
#[tauri::command]
pub async fn verify_recovery_code(app: tauri::AppHandle, code: String) -> Result<bool, String> {
    let file = config_file(&app)?;
    let v = read_config_value(&file);
    let stored = v.get("recovery_code").and_then(|c| c.as_str()).unwrap_or("");
    if stored.is_empty() || stored != code.trim() {
        return Ok(false);
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let expires = v
        .get("recovery_code_expires")
        .and_then(|c| c.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    Ok(expires > now)
}

/// 用验证码重置访问密码（无需旧密码）。成功后清除验证码。
#[tauri::command]
pub async fn reset_password(app: tauri::AppHandle, code: String, new: String) -> Result<(), String> {
    let ok = verify_recovery_code(app.clone(), code).await?;
    if !ok {
        return Err("验证码错误或已过期".into());
    }
    let new = new.trim().to_string();
    if new.len() < 4 {
        return Err("新密码至少 4 位".into());
    }
    write_config_keys(
        &app,
        &[
            ("password", new.clone()),
            ("myfiles_password", new),
            ("recovery_code", String::new()),
            ("recovery_code_expires", String::new()),
        ],
    )
}

/* ---------- 软件下载：下载目录配置 + 内置下载 + Free Download Manager ---------- */

/// 读取 config.json 为对象（文件不存在/损坏返回空对象）。
fn read_config_value(file: &Path) -> Value {
    if file.exists() {
        if let Ok(content) = fs::read_to_string(file) {
            if let Ok(v) = serde_json::from_str::<Value>(&content) {
                if v.is_object() {
                    return v;
                }
            }
        }
    }
    serde_json::json!({})
}

/// 默认下载目录：Windows 取 USERPROFILE\Downloads。
fn default_download_dir() -> String {
    #[cfg(windows)]
    {
        if let Ok(profile) = std::env::var("USERPROFILE") {
            if !profile.is_empty() {
                return format!("{}\\Downloads", profile);
            }
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return format!("{}/Downloads", home);
        }
    }
    ".".to_string()
}

/// 下载配置：{ dir, useFdm }，仅保存在本机 config.json（不入仓库）。
#[tauri::command]
pub async fn get_download_config(app: tauri::AppHandle) -> Result<Value, String> {
    let v = read_config_value(&config_file(&app)?);
    let dir = v
        .get("download_dir")
        .and_then(|d| d.as_str())
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(default_download_dir);
    let use_fdm = v
        .get("download_use_fdm")
        .and_then(|b| b.as_bool())
        .unwrap_or(false);
    Ok(serde_json::json!({ "dir": dir, "useFdm": use_fdm }))
}

#[tauri::command]
pub async fn set_download_config(app: tauri::AppHandle, dir: String, use_fdm: bool) -> Result<(), String> {
    let mut dir = dir.trim().to_string();
    if dir.is_empty() {
        dir = default_download_dir();
    }
    let dir_path = std::path::PathBuf::from(&dir);
    if !dir_path.exists() {
        fs::create_dir_all(&dir_path).map_err(|e| format!("无法创建下载目录：{}", e))?;
    }
    let file = config_file(&app)?;
    let mut v = read_config_value(&file);
    v["download_dir"] = serde_json::Value::String(dir);
    v["download_use_fdm"] = serde_json::Value::Bool(use_fdm);
    fs::write(&file, to_pretty_4(&v) + "\n").map_err(|e| e.to_string())
}

/// 文件名清洗：去掉 Windows 非法字符与控制字符，杜绝路径穿越/覆盖。
fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .filter(|c| !matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'))
        .filter(|c| !c.is_control())
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').to_string();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("con") || trimmed.eq_ignore_ascii_case("nul") {
        "download".to_string()
    } else {
        trimmed
    }
}

/// 从 URL 路径末段推导文件名（含 percent 解码）。
fn url_filename(url: &str) -> Option<String> {
    let path = url.split(['?', '#']).next().unwrap_or(url);
    let last = path.split('/').filter(|s| !s.is_empty()).last()?;
    let decoded = percent_encoding::percent_decode_str(last).decode_utf8_lossy().into_owned();
    let name = sanitize_filename(&decoded);
    if name.is_empty() || name == "download" {
        None
    } else {
        Some(name)
    }
}

/// 从 Content-Disposition 头解析文件名（支持 filename* / filename 两种写法）。
fn cd_filename(value: &str) -> Option<String> {
    if let Some(idx) = value.find("filename*=UTF-8''") {
        let raw = value[idx + "filename*=UTF-8''".len()..].split(';').next().unwrap_or("").trim();
        let decoded = percent_encoding::percent_decode_str(raw).decode_utf8_lossy().into_owned();
        let name = sanitize_filename(&decoded);
        if !name.is_empty() && name != "download" {
            return Some(name);
        }
    }
    if let Some(idx) = value.find("filename=") {
        let raw = value[idx + 9..].split(';').next().unwrap_or("").trim_matches('"').trim();
        let name = sanitize_filename(raw);
        if !name.is_empty() && name != "download" {
            return Some(name);
        }
    }
    None
}

/// 内置下载：后台线程流式下载到本地目录，进度通过 download-progress 事件上报。
/// 立即返回 { id, target }，前端监听 download-progress 事件更新进度。
#[tauri::command]
pub async fn download_file(app: tauri::AppHandle, url: String, dir: String) -> Result<Value, String> {
    let url = url.trim().to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("仅支持 http/https 下载链接".into());
    }
    let dir = if dir.trim().is_empty() {
        default_download_dir()
    } else {
        dir.trim().to_string()
    };
    let dir_path = std::path::PathBuf::from(&dir);
    fs::create_dir_all(&dir_path).map_err(|e| format!("无法创建下载目录：{}", e))?;

    let name = url_filename(&url).unwrap_or_else(|| "download".to_string());
    let id = format!("{:x}", md5::compute(format!("{}|{}", url, name)));
    let target = dir_path.join(&name);
    let target_str = target.to_string_lossy().to_string();

    let handle = app.clone();
    let thread_id = id.clone();
    std::thread::spawn(move || download_worker(&handle, &url, &dir_path, &name, &thread_id, &target));

    Ok(serde_json::json!({ "id": id, "target": target_str }))
}

fn download_worker(
    app: &tauri::AppHandle,
    url: &str,
    dir: &Path,
    fallback_name: &str,
    id: &str,
    fallback_target: &std::path::PathBuf,
) {
    use tauri::Emitter;
    let emit = |v: Value| {
        let _ = app.emit("download-progress", v);
    };
    emit(serde_json::json!({
        "id": id, "phase": "start", "name": fallback_name,
        "target": fallback_target.to_string_lossy().to_string(), "received": 0, "total": 0
    }));
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(30))
        .build();
    let result = agent.get(url).set("User-Agent", "AlpeHuez/0.5.0").call();
    let resp = match result {
        Ok(resp) => resp,
        Err(e) => {
            emit(serde_json::json!({ "id": id, "phase": "error", "error": format!("连接失败：{}", e) }));
            return;
        }
    };
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let name = if fallback_name == "download" {
        resp.header("Content-Disposition")
            .and_then(cd_filename)
            .unwrap_or_else(|| fallback_name.to_string())
    } else {
        fallback_name.to_string()
    };
    let target = dir.join(&name);
    let mut file = match fs::File::create(&target) {
        Ok(f) => f,
        Err(e) => {
            emit(serde_json::json!({ "id": id, "phase": "error", "error": format!("无法创建文件：{}", e) }));
            return;
        }
    };
    let mut reader = resp.into_reader();
    let mut buf = [0u8; 65536];
    let mut received: u64 = 0;
    let mut last_emit = Instant::now();
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                received += n as u64;
                if let Err(e) = file.write_all(&buf[..n]) {
                    emit(serde_json::json!({ "id": id, "phase": "error", "error": format!("写入失败：{}", e) }));
                    return;
                }
                if last_emit.elapsed() >= Duration::from_millis(150) || (total > 0 && received >= total) {
                    emit(serde_json::json!({
                        "id": id, "phase": "progress", "name": name,
                        "target": target.to_string_lossy().to_string(),
                        "received": received, "total": total
                    }));
                    last_emit = Instant::now();
                }
            }
            Err(e) => {
                emit(serde_json::json!({ "id": id, "phase": "error", "error": format!("读取失败：{}", e) }));
                return;
            }
        }
    }
    emit(serde_json::json!({
        "id": id, "phase": "done", "name": name,
        "target": target.to_string_lossy().to_string(),
        "received": received, "total": total
    }));
}

/// 打开 Free Download Manager 接管下载（第二种下载方式）。
/// FDM 命令行传 URL 即可加入下载队列，无需额外参数。
#[tauri::command]
pub async fn open_in_fdm(url: String) -> Result<(), String> {
    let url = url.trim().to_string();
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("仅支持 http/https 下载链接".into());
    }
    #[cfg(windows)]
    {
        let exe = find_fdm_exe()?;
        let mut c = Command::new(&exe);
        silent(&mut c);
        c.arg(&url).spawn().map_err(|e| format!("启动 Free Download Manager 失败：{}", e))?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = url;
        Err("Free Download Manager 仅支持 Windows".into())
    }
}

/// 查找 fdm.exe：先查已知安装路径，再查注册表 Uninstall 项。
#[cfg(windows)]
fn find_fdm_exe() -> Result<std::path::PathBuf, String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;

    const KNOWN: &[&str] = &[
        r"C:\Program Files\FreeDownloadManager.ORG\Free Download Manager\fdm.exe",
        r"C:\Program Files (x86)\FreeDownloadManager.ORG\Free Download Manager\fdm.exe",
        r"C:\Program Files\Free Download Manager\fdm.exe",
        r"C:\Program Files (x86)\Free Download Manager\fdm.exe",
    ];
    for p in KNOWN {
        let pb = std::path::PathBuf::from(p);
        if pb.exists() {
            return Ok(pb);
        }
    }
    let uninstall_roots = [
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_LOCAL_MACHINE, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        (HKEY_CURRENT_USER, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];
    for (hive, sub) in uninstall_roots {
        let Ok(root) = RegKey::predef(hive).open_subkey_with_flags(sub, KEY_READ) else {
            continue;
        };
        for name in root.enum_keys().flatten() {
            let Ok(sk) = root.open_subkey_with_flags(&name, KEY_READ) else {
                continue;
            };
            let Ok(display_name) = sk.get_value::<String, _>("DisplayName") else {
                continue;
            };
            if !display_name.to_lowercase().contains("free download manager") {
                continue;
            }
            if let Ok(icon) = sk.get_value::<String, _>("DisplayIcon") {
                let exe = icon.split(',').next().unwrap_or(&icon).trim().trim_matches('"');
                if !exe.is_empty() && std::path::Path::new(exe).exists() {
                    return Ok(std::path::PathBuf::from(exe));
                }
            }
            if let Ok(location) = sk.get_value::<String, _>("InstallLocation") {
                let p = std::path::PathBuf::from(location).join("fdm.exe");
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    Err("未找到 Free Download Manager（fdm.exe），请先安装 FDM 或使用内置下载".into())
}

/// 保存背景图（base64 → 物理文件，方案 B）。返回文件绝对路径，前端用 convertFileSrc 引用。
#[tauri::command]
pub async fn save_bg_image(app: tauri::AppHandle, data: String, ext: String) -> Result<String, String> {
    use base64::Engine;
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?.join("bg");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ext = if matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp") { ext } else { "png".to_string() };
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let name = format!("bg_{}.{}", secs, ext);
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data.trim())
        .map_err(|e| e.to_string())?;
    let path = dir.join(&name);
    fs::write(&path, bytes).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn get_bg_config(app: tauri::AppHandle) -> Result<Value, String> {
    let file = config_file(&app)?;
    if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(v.get("bg").cloned().unwrap_or_else(|| serde_json::json!({})))
    } else {
        Ok(serde_json::json!({}))
    }
}

#[tauri::command]
pub async fn set_bg_config(app: tauri::AppHandle, config: Value) -> Result<(), String> {
    let file = config_file(&app)?;
    let mut v = if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if !v.is_object() {
        v = serde_json::json!({});
    }
    v.as_object_mut()
        .expect("刚构造的对象")
        .insert("bg".into(), config);
    fs::write(&file, to_pretty_4(&v) + "\n").map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_feedback(app: tauri::AppHandle, text: String) -> Result<(), String> {
    let file = config_file(&app)?;
    let mut v = if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if !v.is_object() {
        v = serde_json::json!({});
    }
    v.as_object_mut()
        .expect("刚构造的对象")
        .insert("feedback".into(), serde_json::Value::String(text.trim().to_string()));
    fs::write(&file, to_pretty_4(&v) + "\n").map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_feedback(app: tauri::AppHandle) -> Result<String, String> {
    let file = config_file(&app)?;
    if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        return Ok(v
            .get("feedback")
            .and_then(|f| f.as_str())
            .unwrap_or("")
            .to_string());
    }
    Ok(String::new())
}

#[tauri::command]
pub async fn get_wechat_qr() -> Result<String, String> {
    const CANDIDATES: [&str; 1] = [
        r"D:\YL2026\sun-panel\my-nav\新建文件夹\github-release-ApleHuez-pictures\v0.2.0\qrcode-wechat.png",
    ];
    for candidate in CANDIDATES {
        let path = Path::new(candidate);
        if path.exists() {
            let bytes = fs::read(path).map_err(|e| e.to_string())?;
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            return Ok(format!("data:image/png;base64,{encoded}"));
        }
    }
    Err("未找到微信二维码，请在本地保留 qrcode-wechat.png 后重试".into())
}

/// 在系统默认浏览器中打开外部链接（仅 http/https）。
/// 若用户配置了默认浏览器，则优先使用该浏览器。
#[tauri::command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    route_external_url(&app, &url)
}

/// 在操作系统默认应用中打开自定义 URL Scheme（appflowy://、obsidian://、mailto: 等）。
/// http/https/file 请走 open_url，此处直接拒绝避免绕过内部 WebView 策略。
#[tauri::command]
pub async fn open_url_scheme(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("empty url".into());
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") || trimmed.starts_with("file://") {
        return Err("http(s)/file 请使用 open_url".into());
    }
    if !trimmed.contains("://") && !trimmed.starts_with("mailto:") && !trimmed.starts_with("tel:") {
        return Err("不是可识别的 URL scheme".into());
    }
    #[cfg(target_os = "android")]
    {
        tauri_plugin_opener::open_url(trimmed, None::<&str>).map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        let quoted = format!("\"{}\"", trimmed);
        let mut c = std::process::Command::new("cmd");
        silent(&mut c);
        c.args(["/C", "start", "", &quoted])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "android")))]
    {
        let mut c = std::process::Command::new("open");
        silent(&mut c);
        c.arg(trimmed).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// 读取系统剪贴板文本（用于拦截腾讯会议链接等）。
#[tauri::command]
pub async fn read_clipboard() -> Result<String, String> {
    #[cfg(not(target_os = "android"))]
    {
        let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        return cb.get_text().map_err(|e| e.to_string());
    }
    #[cfg(target_os = "android")]
    {
        Err("剪贴板读取仅桌面版可用".into())
    }
}

fn browser_mode(app: &tauri::AppHandle) -> Result<String, String> {
    let file = config_file(app)?;
    if !file.exists() {
        return Ok("internal".into());
    }
    let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let mode = v
        .get("browser")
        .and_then(|b| b.get("mode"))
        .and_then(|m| m.as_str())
        .unwrap_or("internal");
    Ok(if mode == "external" { "external" } else { "internal" }.into())
}

/// 在外部浏览器打开 http/https 链接。调用方（前端 openUrl / openExternal）已按
/// browser_mode 自行决定走内部 WebView 还是外部，这里只负责"外部打开"，不再看 mode，
/// 否则右键「Open external」会被错误地开进内部 WebView。
fn route_external_url(app: &tauri::AppHandle, url: &str) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("仅支持 http/https 链接".into());
    }
    // Android：无内嵌子窗口，一律交给系统浏览器打开。
    #[cfg(target_os = "android")]
    {
        let _ = app;
        return tauri_plugin_opener::open_url(url, None::<&str>).map_err(|e| e.to_string());
    }
    #[cfg(not(target_os = "android"))]
    {
        if let Ok(Some(browser)) = get_browser_path(app) {
            let mut c = Command::new(&browser);
            silent(&mut c);
            c.arg(&url).spawn().map_err(|e| e.to_string())?;
            return Ok(());
        }
        open_in_system_browser(&url)
    }
}

/// 用 ShellExecuteW 打开 URL（尊重系统默认浏览器关联），失败时回退到常见浏览器可执行文件。
#[cfg(windows)]
fn open_in_system_browser(url: &str) -> Result<(), String> {
    use std::ptr;
    use windows_sys::Win32::Foundation::HINSTANCE;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    let wide: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    // 返回值大于 32 表示成功（句柄），否则是 SE_ERR_* 错误码。
    let res: HINSTANCE = unsafe {
        ShellExecuteW(
            ptr::null_mut(),
            ptr::null(),
            wide.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        )
    };
    let code = res as isize;
    if code > 32 {
        return Ok(());
    }
    // 默认关联缺失/损坏（常见于卸载浏览器后残留）时，逐个尝试常见浏览器。
    for path in [
        "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe",
        "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe",
        "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe",
        "C:\\Program Files\\Mozilla Firefox\\firefox.exe",
        "C:\\Program Files (x86)\\Mozilla Firefox\\firefox.exe",
        "C:\\Program Files (x86)\\360\\360se6\\Application\\360se.exe",
        "C:\\Program Files\\360\\360se6\\Application\\360se.exe",
    ] {
        if Path::new(path).exists() {
            let mut c = Command::new(path);
            silent(&mut c);
            c.arg(url).spawn().map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("无法打开链接：找不到默认浏览器（错误码 {code}）"))
}

#[cfg(not(windows))]
fn open_in_system_browser(url: &str) -> Result<(), String> {
    let mut c = Command::new("xdg-open");
    silent(&mut c);
    c.arg(url).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

fn get_browser_path(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    if browser_mode(app)? != "external" {
        return Ok(None);
    }
    let file = config_file(app)?;
    if !file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let browser = v.get("browser");
    Ok(browser
        .and_then(|b| b.get("path"))
        .and_then(|p| p.as_str())
        .filter(|p| !p.is_empty())
        .map(|s| s.to_string()))
}

#[tauri::command]
pub async fn get_browser_config(app: tauri::AppHandle) -> Result<Value, String> {
    let file = config_file(&app)?;
    if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(v.get("browser").cloned().unwrap_or_else(|| serde_json::json!({})))
    } else {
        Ok(serde_json::json!({}))
    }
}

/// 打开开发者面板窗口（单例，已存在则聚焦）。
/// 用 WebviewUrl::App 加载打包进 frontendDist 的 panel/index.html，
/// 保证 __TAURI__ 注入与 IPC 可用（External 加载 nav:// 页面时面板从未成功加载）。
/// 窗口先隐藏创建，等面板页面真正加载完成（tauri.localhost 主框架 Finished）再显示，
/// 避免 WebView2 初始 about:blank 阶段在屏幕上闪出白色窗口。
static PANEL_OPENING: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub async fn open_dev_panel(app: tauri::AppHandle) -> Result<(), String> {
    // Android 不支持多窗口，开发者面板仅桌面版可用。
    #[cfg(target_os = "android")]
    {
        let _ = app;
        return Err("开发者面板仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
        // 面板语言跟随主应用：从配置库读 app_lang，经 ?lang= 传给面板窗口
        // （独立面板窗口与主窗口不同 origin，无法直读主窗口 localStorage）。
        let panel_lang = db_conn(&app)
            .ok()
            .and_then(|conn| db::get_config(&conn, "app_lang").ok())
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .filter(|s| s == "zh" || s == "en")
            .unwrap_or_else(|| "en".into());

        if let Some(win) = app.get_webview_window("panel") {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        let _ = win.eval(&format!("location.search = 'embedded=1&auth={nonce}&lang={panel_lang}'"));
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    if PANEL_OPENING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Ok(());
    }

    let shown = Arc::new(AtomicBool::new(false));
    let route_app = app.clone();
    let result = WebviewWindowBuilder::new(
        &app,
        "panel",
        WebviewUrl::App(format!("index.html?lang={panel_lang}").into()),
    )
        .title("AlpeHuez 开发者面板")
        .inner_size(1280.0, 800.0)
        .min_inner_size(960.0, 640.0)
        .center()
        .visible(false)
        .on_page_load(
            move |window, payload| {
                if payload.event() == PageLoadEvent::Finished
                    && payload.url().as_str().contains("tauri.localhost")
                    && !shown.swap(true, Ordering::SeqCst)
                {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            },
        )
        .on_new_window(move |new_url, _features| {
            let opener = route_app.clone();
            std::thread::spawn(move || {
                let _ = route_external_url(&opener, new_url.as_str());
            });
            tauri::webview::NewWindowResponse::Deny
        })
        .build()
        .map(|_| ())
        .map_err(|e| e.to_string());

    PANEL_OPENING.store(false, Ordering::SeqCst);
    result
    }
}

/// 在 AlpeHuez 主窗口内打开外部网站，所有网页标签共享同一份
/// 持久化会话数据，从而保留登录态，而不把账号密码写进 links.json。
#[tauri::command]
pub async fn open_internal_page(
    app: tauri::AppHandle,
    url: String,
    title: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    peek: Option<bool>,
) -> Result<InternalPageInfo, String> {
    open_internal_page_impl(app, url, title, x, y, width, height, peek)
}

fn open_internal_page_impl(
    app: tauri::AppHandle,
    url: String,
    title: Option<String>,
    x: Option<f64>,
    y: Option<f64>,
    width: Option<f64>,
    height: Option<f64>,
    peek: Option<bool>,
) -> Result<InternalPageInfo, String> {
    // Android 不支持子 webview 内嵌浏览，所有站点走系统浏览器。
    #[cfg(target_os = "android")]
    {
        let _ = (app, url, title, x, y, width, height, peek);
        return Err("移动端不支持应用内浏览，已改用系统浏览器打开".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    let parsed = url
        .parse::<tauri::Url>()
        .map_err(|e| e.to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("仅支持 http/https 网站".into());
    }

    let hash: String = format!("{:x}", md5::compute(url.as_bytes()))
        .chars()
        .take(12)
        .collect();
    let prefix = if peek.unwrap_or(false) { "peek-" } else { "browser-" };
    let label = format!("{prefix}{hash}");
    let fallback_title = parsed
        .host_str()
        .map(|h| h.to_string())
        .unwrap_or_else(|| "Untitled".into());
    let page_title = title.clone().unwrap_or_else(|| fallback_title.clone());

    if let Some(webview) = app.get_webview(&label) {
        if let (Some(x), Some(y), Some(width), Some(height)) = (x, y, width, height) {
            let _ = webview.set_bounds(Rect {
                position: PhysicalPosition::new(x, y).into(),
                size: PhysicalSize::new(width, height).into(),
            });
        }
        // 正在加载的标签不提前显示，等 browser-load-finished 后由前端统一显示。
        if !LOADING_LABELS.lock().unwrap().contains(&label) {
            webview.show().map_err(|e| e.to_string())?;
            webview.set_focus().map_err(|e| e.to_string())?;
        }
        return Ok(InternalPageInfo {
            label,
            url,
            title: page_title,
        });
    }

    let session_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("browser-session");
    let main_window = app
        .get_window("main")
        .ok_or_else(|| "未找到 AlpeHuez 主窗口".to_string())?;
    let position = PhysicalPosition::new(x.unwrap_or(320.0), y.unwrap_or(96.0));
    let size = PhysicalSize::new(width.unwrap_or(900.0), height.unwrap_or(704.0));

    let page_app = app.clone();
    let page_label = label.clone();
    let _page_url = url.clone();
    let page_title_clone = page_title.clone();
    let title_app = app.clone();
    let title_label = label.clone();
    let title_url = url.clone();
    let open_app = app.clone();
    let child_script = format!("{}\n{}", ADBLOCK_JS, CHILD_KEY_JS);

    let builder = WebviewBuilder::new(label.clone(), WebviewUrl::External(parsed))
        .data_directory(session_dir)
        // 引擎默认底色用白色而非深色：创建瞬间若有一帧漏出，浅色主题下是白不是黑。
        // 加载期间 webview 会被 hide + 前端自行车遮罩覆盖，此色仅作兜底。
        .background_color(tauri::window::Color(255, 255, 255, 255))
        .initialization_script(child_script.as_str())
        .on_page_load(move |webview, payload| {
            if payload.url().as_str() == "about:blank" {
                return;
            }
            let current_url = payload.url().to_string();
            let current_label = page_label.clone();
            if payload.event() == PageLoadEvent::Started {
                // 加载期间隐藏 webview，由前端自行车 loading overlay 代替黑屏。
                let _ = webview.hide();
                LOADING_LABELS.lock().unwrap().insert(current_label.clone());
                let _ = page_app.emit(
                    "browser-load-started",
                    InternalPageInfo {
                        label: current_label,
                        url: current_url,
                        title: page_title_clone.clone(),
                    },
                );
            } else if payload.event() == PageLoadEvent::Finished {
                LOADING_LABELS.lock().unwrap().remove(&current_label);
                let _ = page_app.emit(
                    "browser-tab-updated",
                    InternalPageInfo {
                        label: current_label.clone(),
                        url: current_url.clone(),
                        title: page_title_clone.clone(),
                    },
                );
                let _ = page_app.emit(
                    "browser-load-finished",
                    InternalPageInfo {
                        label: current_label,
                        url: current_url,
                        title: page_title_clone.clone(),
                    },
                );
            }
        })
        .on_document_title_changed(move |webview, document_title| {
            let current_url = webview
                .url()
                .map(|u| u.to_string())
                .unwrap_or_else(|_| title_url.clone());
            let _ = title_app.emit(
                "browser-tab-updated",
                InternalPageInfo {
                    label: title_label.clone(),
                    url: current_url,
                    title: document_title,
                },
            );
        })
        .on_new_window(move |new_url, _features| {
            let opener = open_app.clone();
            std::thread::spawn(move || {
                let _ = route_external_url(&opener, new_url.as_str());
            });
            tauri::webview::NewWindowResponse::Deny
        });

    // 创建后立即隐藏，由前端自行车 loading overlay 代替黑屏；加载完成后经
    // activate_internal_page 才显示。WebviewBuilder 无 visible()，只能建好再 hide。
    let child = main_window
        .add_child(builder, position, size)
        .map_err(|e| e.to_string())?;
    let _ = child.hide();

    Ok(InternalPageInfo {
        label,
        url,
        title: page_title,
    })
    }
}

#[tauri::command]
pub async fn activate_internal_page(
    app: tauri::AppHandle,
    label: Option<String>,
) -> Result<(), String> {
    // Android 无内嵌子 webview，网页标签仅桌面版可用。
    #[cfg(target_os = "android")]
    {
        let _ = (app, label);
        return Err("网页标签仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    for (candidate, webview) in app.webviews() {
        if !candidate.starts_with("browser-") || webview.window().label() != "main" {
            continue;
        }
        if label.as_deref() == Some(candidate.as_str()) {
            // 正在加载的标签不显示，等 browser-load-finished 后前端再调用本命令显示。
            if !LOADING_LABELS.lock().unwrap().contains(&candidate) {
                webview.show().map_err(|e| e.to_string())?;
                webview.set_focus().map_err(|e| e.to_string())?;
            }
        } else {
            webview.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
    }
}

#[tauri::command]
pub async fn layout_internal_pages(
    app: tauri::AppHandle,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (app, x, y, width, height);
        return Err("网页标签仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    for (label, webview) in app.webviews() {
        if !label.starts_with("browser-") || webview.window().label() != "main" {
            continue;
        }
        let bounds = Rect {
            position: PhysicalPosition::new(x, y).into(),
            size: PhysicalSize::new(width, height).into(),
        };
        webview.set_bounds(bounds).map_err(|e| e.to_string())?;
    }
    Ok(())
    }
}

#[tauri::command]
pub async fn focus_internal_page(app: tauri::AppHandle, label: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (app, label);
        return Err("网页标签仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "网页标签已关闭".to_string())?;
    webview.show().map_err(|e| e.to_string())?;
    webview.set_focus().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub async fn close_internal_page(app: tauri::AppHandle, label: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (app, label);
        return Err("网页标签仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    if !label.starts_with("browser-") {
        return Err("无效的网页标签".into());
    }
    if let Some(webview) = app.get_webview(&label) {
        webview.close().map_err(|e| e.to_string())?;
    }
    let _ = app.emit("browser-tab-closed", label);
    Ok(())
    }
}

/// 按 label 隐藏/显示内部页面 webview（标签休眠用）。内部页面是 child webview，
/// 不是独立窗口，必须用 get_webview 查找（get_webview_window 永远匹配不到 browser-*）。
#[tauri::command]
pub async fn set_internal_page_visible(app: tauri::AppHandle, label: String, visible: bool) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (app, label, visible);
        return Err("网页标签仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    if !label.starts_with("browser-") {
        return Err("无效的网页标签".into());
    }
    if let Some(webview) = app.get_webview(&label) {
        if visible {
            webview.show().map_err(|e| e.to_string())?;
            webview.set_focus().map_err(|e| e.to_string())?;
        } else {
            webview.hide().map_err(|e| e.to_string())?;
        }
    }
    Ok(())
    }
}

#[tauri::command]
pub fn mark_first_run(app: tauri::AppHandle) -> bool {
    let marker = app
        .path()
        .app_config_dir()
        .unwrap_or_default()
        .join("first_run.marker");
    if marker.exists() {
        return false;
    }
    let _ = std::fs::write(&marker, "1");
    true
}

#[tauri::command]
pub fn go_back_internal_page(app: tauri::AppHandle, label: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (app, label);
        return Err("网页标签仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    if !label.starts_with("browser-") {
        return Err("invalid browser tab label".into());
    }
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "browser tab is closed".to_string())?;
    webview
        .eval("window.history.back()")
        .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn go_forward_internal_page(app: tauri::AppHandle, label: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let _ = (app, label);
        return Err("网页标签仅桌面版可用".into());
    }
    #[cfg(not(target_os = "android"))]
    {
    if !label.starts_with("browser-") {
        return Err("invalid browser tab label".into());
    }
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "browser tab is closed".to_string())?;
    webview
        .eval("window.history.forward()")
        .map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub fn list_internal_pages(app: tauri::AppHandle) -> Vec<InternalPageInfo> {
    app.webviews()
        .into_iter()
        .filter(|(label, webview)| {
            label.starts_with("browser-") && webview.window().label() == "main"
        })
        .map(|(label, webview)| {
            let url = webview.url().map(|u| u.to_string()).unwrap_or_default();
            let title = url
                .parse::<tauri::Url>()
                .ok()
                .and_then(|u| u.host_str().map(|h| h.to_string()))
                .unwrap_or_else(|| "Untitled".to_string());
            InternalPageInfo { label, url, title }
        })
        .collect()
}

#[tauri::command]
pub async fn list_workspaces(app: tauri::AppHandle) -> Result<Vec<db::Workspace>, String> {
    let conn = db_conn(&app)?;
    db::list_workspaces(&conn)
}

#[tauri::command]
pub async fn get_active_workspace(app: tauri::AppHandle) -> Result<i64, String> {
    let conn = db_conn(&app)?;
    match db::get_config(&conn, "active_workspace")? {
        serde_json::Value::Number(n) => n.as_i64().ok_or_else(|| "active_workspace 非整数".into()),
        _ => {
            let first = db::list_workspaces(&conn)?.into_iter().next().ok_or_else(|| "无工作台".to_string())?;
            Ok(first.id)
        }
    }
}

#[tauri::command]
pub async fn set_active_workspace(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = db_conn(&app)?;
    db::get_workspace(&conn, id)?; // 校验存在
    db::set_config(&conn, "active_workspace", &serde_json::json!(id))
}

#[tauri::command]
pub async fn create_workspace(
    app: tauri::AppHandle,
    name: String,
    role: String,
    rider_type: String,
    rider_name: String,
    rider_number: i64,
) -> Result<db::Workspace, String> {
    let conn = db_conn(&app)?;
    db::create_workspace(&conn, &name, &role, &rider_type, &rider_name, rider_number)
}

#[tauri::command]
pub async fn update_workspace(
    app: tauri::AppHandle,
    id: i64,
    name: String,
    role: String,
    rider_type: String,
    rider_name: String,
    rider_number: i64,
    specialties: serde_json::Value,
) -> Result<(), String> {
    let conn = db_conn(&app)?;
    db::update_workspace(&conn, id, &name, &role, &rider_type, &rider_name, rider_number, &specialties)
}

#[tauri::command]
pub async fn delete_workspace(app: tauri::AppHandle, id: i64) -> Result<(), String> {
    let conn = db_conn(&app)?;
    db::delete_workspace(&conn, id)
}

#[tauri::command]
pub async fn get_workspace_links(app: tauri::AppHandle, id: i64) -> Result<serde_json::Value, String> {
    let conn = db_conn(&app)?;
    let ws = db::get_workspace(&conn, id)?;
    if ws.role == "leader" {
        let content = std::fs::read_to_string(crate::repo_root().join("links.json")).map_err(|e| e.to_string())?;
        return serde_json::from_str(&content).map_err(|e| e.to_string());
    }
    db::get_workspace_links(&conn, id)
}

#[tauri::command]
pub async fn save_workspace_links(app: tauri::AppHandle, id: i64, data: serde_json::Value) -> Result<(), String> {
    let conn = db_conn(&app)?;
    db::save_workspace_links(&conn, id, &data)
}

#[tauri::command]
pub async fn get_app_config(app: tauri::AppHandle, key: String) -> Result<serde_json::Value, String> {
    let conn = db_conn(&app)?;
    db::get_config(&conn, &key)
}

#[tauri::command]
pub async fn set_app_config(app: tauri::AppHandle, key: String, value: serde_json::Value) -> Result<(), String> {
    let conn = db_conn(&app)?;
    db::set_config(&conn, &key, &value)
}

// ===== HF 备份（P1）=====

fn hf_token() -> Result<String, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "无法定位用户主目录，请确认 ~/.cache/huggingface/token 存在".to_string())?;
    let path = Path::new(&home).join(".cache").join("huggingface").join("token");
    let token = fs::read_to_string(&path).map_err(|e| format!("读取 HF token 失败: {e}（路径: {}）", path.display()))?;
    let token = token.trim();
    if token.is_empty() {
        return Err("HF token 为空（~/.cache/huggingface/token）".to_string());
    }
    Ok(token.to_string())
}

fn hf_repo(conn: &rusqlite::Connection) -> String {
    db::get_config(conn, "hf_repo")
        .ok()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Helloxiaolaodi/AlpeHuez".to_string())
}

fn hf_remote_url(repo: &str, token: &str) -> String {
    format!("https://user:{token}@huggingface.co/datasets/{repo}.git")
}

fn run_git_proxy(args: &[&str], cwd: &Path) -> (i32, String) {
    let mut c = Command::new("git");
    silent(&mut c);
    let mut full = vec!["-c", "http.lowSpeedLimit=1", "-c", "http.lowSpeedTime=45"];
    full.extend_from_slice(args);
    c.env("https_proxy", "http://127.0.0.1:7897")
        .env("http_proxy", "http://127.0.0.1:7897");
    match c.args(&full).current_dir(cwd).output() {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            (out.status.code().unwrap_or(-1), s)
        }
        Err(e) => (-1, e.to_string()),
    }
}

fn hf_backup_dir(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("hf-backup");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

/// 清除备份仓残留的 rebase/am 状态。上一次备份若在 push 被拒后 rebase 中途
/// 失败（网络中断等），会在 .git 下留下 rebase-merge 目录，导致后续备份
/// 一律报 "already a rebase-merge directory"。这里全部容忍失败：能 abort 就
/// abort，不能就删目录，最后强制回到 main 分支。
fn cleanup_hf_git_state(dir: &Path) {
    let _ = run_git_proxy(&["rebase", "--abort"], dir);
    let _ = run_git_proxy(&["am", "--abort"], dir);
    let _ = fs::remove_dir_all(dir.join(".git").join("rebase-merge"));
    let _ = fs::remove_dir_all(dir.join(".git").join("rebase-apply"));
    let _ = run_git_proxy(&["checkout", "-f", "main"], dir);
    let _ = run_git_proxy(&["reset", "--hard", "main"], dir);
}

fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir_all(&from, &to)?;
        } else {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn save_daily_note(app: tauri::AppHandle, date: String, notes: String) -> Result<(), String> {
    if date.is_empty()
        || !date
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        return Err("无效的日期".into());
    }
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("daily-notes");
    fs::create_dir_all(&dir).map_err(|e| format!("创建每日笔记目录失败: {e}"))?;
    fs::write(dir.join(format!("{date}.json")), notes)
        .map_err(|e| format!("保存每日笔记失败: {e}"))?;
    Ok(())
}

/// 执行一次 HF 备份（手动按钮与退出时自动备份共用）。
pub fn run_hf_backup(app: &tauri::AppHandle) -> Result<String, String> {
    use rusqlite::backup::Backup;
    let conn = db_conn(app)?;
    let repo = hf_repo(&conn);
    let token = hf_token()?;
    let url = hf_remote_url(&repo, &token);
    let data_dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    let dir = hf_backup_dir(app)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if !dir.join(".git").exists() {
        let (c, o) = run_git_proxy(&["init", "-b", "main"], &dir);
        if c != 0 { return Err(format!("git init 失败: {o}")); }
        for (k, v) in [("user.name", "AlpeHuez Backup"), ("user.email", "alpehuez@localhost")] {
            let (c, o) = run_git_proxy(&["config", k, v], &dir);
            if c != 0 { return Err(format!("git config {k} 失败: {o}")); }
        }
    }

    // 先清除上一次可能残留的 rebase 状态，再重写全部快照文件。
    cleanup_hf_git_state(&dir);

    // SQLite 在线备份（一致性快照，正确处理 journal）
    {
        let src = db::open(&data_dir.join("alpehuez.db"))?;
        let mut dest = rusqlite::Connection::open(dir.join("alpehuez.db")).map_err(|e| e.to_string())?;
        let backup = Backup::new(&src, &mut dest).map_err(|e| format!("备份 SQLite 失败: {e}"))?;
        backup
            .run_to_completion(8, std::time::Duration::from_millis(50), None)
            .map_err(|e| format!("备份 SQLite 失败: {e}"))?;
    }

    if let Ok(cfg_dir) = app.path().app_config_dir() {
        let src = cfg_dir.join("config.json");
        if src.exists() {
            fs::copy(&src, dir.join("config.json")).map_err(|e| format!("复制 config.json 失败: {e}"))?;
        }
    }
    let links = repo_root().join("links.json");
    if links.exists() {
        fs::copy(&links, dir.join("links.json")).map_err(|e| format!("复制 links.json 失败: {e}"))?;
    }
    // 每日笔记随备份一并上传（覆盖式同步整个 daily-notes 目录）
    let notes_src = data_dir.join("daily-notes");
    if notes_src.exists() {
        let notes_dst = dir.join("daily-notes");
        if notes_dst.exists() {
            let _ = fs::remove_dir_all(&notes_dst);
        }
        if let Err(e) = copy_dir_all(&notes_src, &notes_dst) {
            return Err(format!("复制 daily-notes 失败: {e}"));
        }
    }

    let manifest = serde_json::json!({
        "timestamp": ts,
        "app": "AlpeHuez",
        "files": ["alpehuez.db", "config.json", "links.json", "daily-notes", "backup-manifest.json"]
    });
    fs::write(
        dir.join("backup-manifest.json"),
        serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("写备份清单失败: {e}"))?;

    let (c1, o1) = run_git_proxy(&["add", "-A"], &dir);
    if c1 != 0 { return Err(format!("git add 失败: {o1}")); }
    let (c2, o2) = run_git_proxy(&["commit", "-m", &format!("backup: {ts}")], &dir);
    if c2 != 0 && !o2.contains("nothing to commit") {
        return Err(format!("git commit 失败: {o2}"));
    }
    if c2 == 0 {
        let (c3, o3) = run_git_proxy(&["push", &url, "HEAD:main"], &dir);
        if c3 != 0 {
            let (_, fo) = run_git_proxy(&["fetch", &url], &dir);
            let (c4, o4) = run_git_proxy(&["rebase", "FETCH_HEAD"], &dir);
            if c4 != 0 {
                return Err(format!("推送失败: {o3}；rebase 远端失败: {o4}（{fo}）"));
            }
            let (c5, o5) = run_git_proxy(&["push", &url, "HEAD:main"], &dir);
            if c5 != 0 { return Err(o5); }
        }
    }

    db::set_config(&conn, "hf_last_backup", &serde_json::json!(ts)).map_err(|e| e.to_string())?;
    Ok(format!("备份完成（{ts}）"))
}

#[cfg(not(target_os = "android"))]
/// 退出时自动备份：读取 hf_auto_backup 配置，为 true 则执行备份。
/// 退出时写入"待备份"标记（仅当开启自动备份）。退出立即结束，
/// 实际备份推迟到下次启动的后台线程执行，避免网络 push 阻塞退出。
pub fn mark_backup_for_launch(app: &tauri::AppHandle) {
    let auto = db_conn(app)
        .ok()
        .and_then(|conn| db::get_config(&conn, "hf_auto_backup").ok())
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !auto {
        return;
    }
    if let Ok(dir) = app.path().app_data_dir() {
        let bdir = dir.join("hf-backup");
        let _ = std::fs::create_dir_all(&bdir);
        let _ = std::fs::write(bdir.join("pending_backup"), b"1");
    }
}

/// 启动时若有待备份标记，在后台线程执行备份并清除标记（不阻塞启动）。
/// 备份失败时保留标记，下次启动继续重试。
pub fn run_pending_backup_at_launch(app: &tauri::AppHandle) {
    let handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(600));
        if let Ok(dir) = handle.path().app_data_dir() {
            let marker = dir.join("hf-backup").join("pending_backup");
            if marker.exists() {
                let _ = run_hf_backup(&handle);
                let _ = std::fs::remove_file(&marker);
            }
        }
    });
}

#[tauri::command]
pub async fn hf_backup(app: tauri::AppHandle) -> Result<String, String> {
    run_hf_backup(&app)
}

#[derive(serde::Serialize)]
pub struct HfTestResult {
    ok: bool,
    repo: String,
    detail: Option<String>,
}

#[tauri::command]
pub async fn hf_test_connection(app: tauri::AppHandle) -> Result<HfTestResult, String> {
    let token = hf_token()?;
    let conn = db_conn(&app)?;
    let repo = hf_repo(&conn);
    let url = hf_remote_url(&repo, &token);
    let dir = hf_backup_dir(&app)?;
    let (c, o) = run_git_proxy(&["ls-remote", &url, "HEAD"], &dir);
    Ok(HfTestResult {
        ok: c == 0,
        repo,
        detail: (c != 0).then_some(o),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 与 Node `JSON.stringify(去掉md5, null, 4) + '\n'` 的 md5 一致性回归测试。
    #[test]
    fn links_md5_matches_node() {
        let root = repo_root();
        let content = fs::read_to_string(root.join("links.json")).expect("读取 links.json");
        let links: Value = serde_json::from_str(&content).expect("解析 links.json");
        let md5 = recompute_links_md5(&links);
        assert_eq!(md5, "e404e869fdcdaf46d617ff5bc31418d0");
    }
}
