use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::Emitter;
use tauri::Manager;

use crate::db;

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceStatus {
    pub url: String,
    pub online: bool,
    pub latency_ms: u64,
}

pub fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || loop {
        let statuses = check_all(&app);
        let _ = app.emit(
            "service-status-update",
            serde_json::json!({ "statuses": statuses }),
        );
        std::thread::sleep(Duration::from_secs(30));
    });
}

fn check_all(app: &tauri::AppHandle) -> Vec<ServiceStatus> {
    let mut urls: Vec<String> = Vec::new();
    if let Ok(dir) = app.path().app_data_dir() {
        if let Ok(conn) = db::open(&dir.join("alpehuez.db")) {
            if db::init(&conn).is_ok() {
                if let Ok(workspaces) = db::list_workspaces(&conn) {
                    for ws in workspaces {
                        let links = if ws.role == "leader" {
                            std::fs::read_to_string(crate::repo_root().join("links.json")).ok()
                        } else {
                            db::get_workspace_links(&conn, ws.id)
                                .ok()
                                .map(|v| v.to_string())
                        };
                        if let Some(links) = links {
                            urls.extend(collect_monitored_urls(&links));
                        }
                    }
                }
            }
        }
    }
    urls.sort();
    urls.dedup();
    urls.into_iter()
        .map(|url| {
            let (online, latency_ms) = ping(&url);
            ServiceStatus {
                url,
                online,
                latency_ms,
            }
        })
        .collect()
}

fn collect_monitored_urls(links_json: &str) -> Vec<String> {
    let mut urls = Vec::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(links_json) {
        if let Some(icons) = v.get("icons").and_then(|i| i.as_array()) {
            for group in icons {
                if let Some(children) = group.get("children").and_then(|c| c.as_array()) {
                    for item in children {
                        let monitored = item
                            .get("monitor")
                            .and_then(|m| m.as_bool())
                            .unwrap_or(false);
                        if monitored {
                            if let Some(url) = item.get("url").and_then(|u| u.as_str()) {
                                urls.push(url.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
    urls
}

fn ping(url: &str) -> (bool, u64) {
    let start = Instant::now();
    let result = ureq::head(url).timeout(Duration::from_secs(3)).call();
    let latency_ms = start.elapsed().as_millis() as u64;
    match result {
        Ok(_) => (true, latency_ms),
        // 服务器返回任何 HTTP 状态码（含 4xx/5xx）都算在线——服务可达
        Err(ureq::Error::Status(_, _)) => (true, latency_ms),
        Err(_) => (false, latency_ms),
    }
}
