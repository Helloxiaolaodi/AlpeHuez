use base64::Engine;
use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInfo {
    pub name: String,
    pub path: String,
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

fn set_password(app: &tauri::AppHandle, new: &str) -> Result<(), String> {
    let file = config_file(app)?;
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
        .insert("password".into(), serde_json::Value::String(new.to_string()));
    fs::write(&file, to_pretty_4(&v) + "\n").map_err(|e| e.to_string())
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
    set_password(&app, &new)
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
        if browser_mode(app)? == "internal" {
            return open_internal_page_impl(
                app.clone(),
                url.to_string(),
                None,
                None,
                None,
                None,
                None,
                None,
            )
            .map(|_| ());
        }
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

/// 扫描注册表列出已安装浏览器（HKCU 优先，按路径去重）。
#[tauri::command]
pub async fn list_browsers() -> Result<Vec<BrowserInfo>, String> {
    #[cfg(windows)]
    {
        use std::collections::HashSet;
        use winreg::enums::*;
        use winreg::RegKey;
        let mut browsers: Vec<BrowserInfo> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for (hive, sub) in [
            (HKEY_CURRENT_USER, "Software\\Clients\\StartMenuInternet"),
            (HKEY_LOCAL_MACHINE, "Software\\Clients\\StartMenuInternet"),
        ] {
            let Ok(key) = RegKey::predef(hive).open_subkey(sub) else { continue };
            for name in key.enum_keys().flatten() {
                let Ok(cmd_key) = key.open_subkey(format!("{}\\shell\\open\\command", name)) else { continue };
                let Ok(path) = cmd_key.get_value::<String, _>("") else { continue };
                let path = path.trim_matches('"').to_string();
                if !path.is_empty() && seen.insert(path.clone()) {
                    browsers.push(BrowserInfo { name, path });
                }
            }
        }
        Ok(browsers)
    }
    #[cfg(not(windows))]
    {
        Ok(Vec::new())
    }
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

#[tauri::command]
pub async fn set_browser_config(app: tauri::AppHandle, config: Value) -> Result<(), String> {
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
        .insert("browser".into(), config);
    fs::write(&file, to_pretty_4(&v) + "\n").map_err(|e| e.to_string())
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
        if let Some(win) = app.get_webview_window("panel") {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis();
        let _ = win.eval(&format!("location.search = 'embedded=1&auth={nonce}'"));
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
    let result = WebviewWindowBuilder::new(&app, "panel", WebviewUrl::App("index.html".into()))
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
        webview.show().map_err(|e| e.to_string())?;
        webview.set_focus().map_err(|e| e.to_string())?;
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
        .background_color(tauri::window::Color(11, 17, 32, 255))
        .initialization_script(child_script.as_str())
        .on_page_load(move |_webview, payload| {
            if payload.url().as_str() == "about:blank" {
                return;
            }
            let current_url = payload.url().to_string();
            let current_label = page_label.clone();
            if payload.event() == PageLoadEvent::Started {
                let _ = page_app.emit(
                    "browser-load-started",
                    InternalPageInfo {
                        label: current_label,
                        url: current_url,
                        title: page_title_clone.clone(),
                    },
                );
            } else if payload.event() == PageLoadEvent::Finished {
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

    main_window
        .add_child(builder, position, size)
        .map_err(|e| e.to_string())?;

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
