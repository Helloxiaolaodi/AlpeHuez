use serde::Serialize;
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tauri::{webview::PageLoadEvent, Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

use crate::repo_root;

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
    match Command::new(cmd).args(args).current_dir(cwd).output() {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).to_string();
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            (out.status.code().unwrap_or(-1), s)
        }
        Err(e) => (-1, e.to_string()),
    }
}

fn run_cmd_timeout(cmd: &str, args: &[&str], cwd: &Path, timeout_secs: u64) -> (i32, String, bool) {
    let mut child = match Command::new(cmd)
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
    if Command::new("node").arg("--version").output().is_ok() {
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
pub async fn git_push(message: String) -> Result<PushResult, String> {
    let root = repo_root();
    let msg = if message.trim().is_empty() {
        "Update site content".to_string()
    } else {
        message.trim().to_string()
    };
    let (_, add) = run_cmd("git", &["add", "-A"], root);
    let (commit_code, commit) = run_cmd("git", &["commit", "-m", &msg], root);
    let (push_code, push) = run_cmd("git", &["push"], root);
    Ok(PushResult {
        ok: true,
        add,
        commit,
        commit_code,
        push,
        push_code,
    })
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

const DEFAULT_PASSWORD: &str = "Heyman2026/";

fn config_file(app: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("config.json"))
}

fn get_password(app: &tauri::AppHandle) -> Result<String, String> {
    let file = config_file(app)?;
    if file.exists() {
        let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
        let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        Ok(v.get("password")
            .and_then(|p| p.as_str())
            .unwrap_or(DEFAULT_PASSWORD)
            .to_string())
    } else {
        let v = serde_json::json!({ "password": DEFAULT_PASSWORD });
        fs::write(&file, to_pretty_4(&v) + "\n").map_err(|e| e.to_string())?;
        Ok(DEFAULT_PASSWORD.to_string())
    }
}

fn set_password(app: &tauri::AppHandle, new: &str) -> Result<(), String> {
    let file = config_file(app)?;
    let v = serde_json::json!({ "password": new });
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

/// 在系统默认浏览器中打开外部链接（仅 http/https）。
/// 若用户配置了默认浏览器，则优先使用该浏览器。
#[tauri::command]
pub async fn open_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("仅支持 http/https 链接".into());
    }
    if let Ok(Some(browser)) = get_browser_path(&app) {
        Command::new(&browser)
            .arg(&url)
            .spawn()
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    open_in_system_browser(&url)
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
            Command::new(path)
                .arg(url)
                .spawn()
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("无法打开链接：找不到默认浏览器（错误码 {code}）"))
}

#[cfg(not(windows))]
fn open_in_system_browser(url: &str) -> Result<(), String> {
    Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn get_browser_path(app: &tauri::AppHandle) -> Result<Option<String>, String> {
    let file = config_file(app)?;
    if !file.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    Ok(v.get("browser")
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
#[tauri::command]
pub async fn open_dev_panel(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("panel") {
        win.show().map_err(|e| e.to_string())?;
        win.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }
    let loaded = Arc::new(AtomicBool::new(false));
    let shown = loaded.clone();
    WebviewWindowBuilder::new(&app, "panel", WebviewUrl::App("index.html".into()))
        .title("AlpeHuez 开发者面板")
        .inner_size(1280.0, 800.0)
        .min_inner_size(960.0, 640.0)
        .center()
        .visible(false)
        .on_page_load(move |window, payload| {
            if payload.event() == PageLoadEvent::Finished
                && payload.url().as_str().contains("tauri.localhost")
                && !shown.swap(true, Ordering::SeqCst)
            {
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .build()
        .map_err(|e| e.to_string())?;

    // 兜底：若页面始终未触发 tauri.localhost 的 Finished（例如加载异常），3 秒后仍显示窗口，避免面板永远不可见。
    let fallback = loaded.clone();
    let app2 = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        if !fallback.load(Ordering::SeqCst) {
            if let Some(w) = app2.get_webview_window("panel") {
                let _ = w.show();
            }
        }
    });
    Ok(())
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
