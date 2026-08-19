use std::path::Path;

#[cfg(not(target_os = "android"))]
use std::fs;

use percent_encoding::percent_decode_str;
use tauri::http::{Request, Response, StatusCode};
use tauri::UriSchemeContext;

#[cfg(not(target_os = "android"))]
use crate::repo_root;

/// Android 上没有仓库目录，门户文件在编译期用 include_dir 打进二进制，
/// nav:// 直接从嵌入资源提供。目录内容由 build.rs 在 Android 目标编译前
/// 从仓库根目录拷贝（仅移动端真正用到的文件）。
#[cfg(target_os = "android")]
use include_dir::{include_dir, Dir};

#[cfg(target_os = "android")]
const WEBROOT: Dir = include_dir!("$CARGO_MANIFEST_DIR/android-webroot");

/// 桌面打包版（无仓库目录）同样把网页资源嵌入二进制，首次启动播种到
/// app_data_dir/webroot 后作为仓库根使用。内容由 build.rs 在编译前从仓库拷贝。
#[cfg(not(target_os = "android"))]
use include_dir::{include_dir, Dir};

#[cfg(not(target_os = "android"))]
const WEBROOT: Dir = include_dir!("$CARGO_MANIFEST_DIR/webroot");

/// 把嵌入的网页资源播种到目标目录，仅写入缺失文件（不覆盖已存在的用户数据）。
/// 打包版首次启动、本地没有仓库目录时调用。
#[cfg(not(target_os = "android"))]
pub fn materialize(dest: &Path) {
    fn walk(dir: &Dir, base: &Path) {
        let _ = fs::create_dir_all(base);
        for file in dir.files() {
            let target = base.join(file.path().file_name().expect("文件应有文件名"));
            if !target.exists() {
                let _ = fs::write(&target, file.contents());
            }
        }
        for sub in dir.dirs() {
            walk(sub, &base.join(sub.path().file_name().expect("目录应有目录名")));
        }
    }
    walk(&WEBROOT, dest);
}

const MIME: &[(&str, &str)] = &[
    (".html", "text/html; charset=utf-8"),
    (".js", "text/javascript; charset=utf-8"),
    (".mjs", "text/javascript; charset=utf-8"),
    (".css", "text/css; charset=utf-8"),
    (".json", "application/json; charset=utf-8"),
    (".png", "image/png"),
    (".jpg", "image/jpeg"),
    (".jpeg", "image/jpeg"),
    (".gif", "image/gif"),
    (".svg", "image/svg+xml"),
    (".webp", "image/webp"),
    (".woff2", "font/woff2"),
    (".ico", "image/x-icon"),
    (".txt", "text/plain; charset=utf-8"),
    (".md", "text/markdown; charset=utf-8"),
    (".qmd", "text/plain; charset=utf-8"),
    (".zip", "application/zip"),
    (".pdf", "application/pdf"),
];

fn mime_for(path: &str) -> &'static str {
    let ext = Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();
    MIME.iter()
        .find(|(k, _)| *k == ext)
        .map(|(_, v)| *v)
        .unwrap_or("application/octet-stream")
}

fn response(status: StatusCode, mime: &str, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("Content-Type", mime)
        .body(body)
        .expect("构造响应失败")
}

/// 注入到所有 HTML 页面的链接拦截脚本：
/// 在 Tauri 环境（有 __TAURI__）下，把 target=_blank 链接和 window.open 转发到
/// open_url 命令，由启动方式配置决定内部打开还是外部浏览器打开；
/// 普通浏览器（Cloudflare 部署）下不生效。
const LINK_HANDLER: &str = r#"<script>
(function () {
  if (!window.__TAURI__) return;
  var invoke = window.__TAURI__.core.invoke;
  function routeLink(url) {
    if (url && /^https?:/i.test(url)) invoke('open_url', { url: url });
  }
  document.addEventListener('click', function (e) {
    var a = e.target && e.target.closest ? e.target.closest('a') : null;
    if (!a) return;
    var href = a.getAttribute('href');
    if (!href) return;
    if (a.getAttribute('target') === '_blank' || a.hasAttribute('download')) {
      if (/^https?:/i.test(href)) {
        e.preventDefault();
        routeLink(href);
      }
    }
  }, true);
  var origOpen = window.open;
  window.open = function (url) {
    if (url && typeof url === 'string') {
      if (/^https?:/i.test(url)) { routeLink(url); return null; }
      if (url.charAt(0) === '/' || url.charAt(0) === '.' || url.charAt(0) === '#') {
        window.location.href = url;
        return null;
      }
    }
    return origOpen.apply(window, arguments);
  };
})();
</script>"#;

pub fn handler(
    _ctx: UriSchemeContext<tauri::Wry>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let path = request.uri().path();
    let decoded = percent_decode_str(path).decode_utf8_lossy();
    let rel = if decoded == "/" {
        "index.html".to_string()
    } else {
        decoded.trim_start_matches('/').to_string()
    };

    // Android：从嵌入资源读取；桌面：从 repo_root 文件系统读取（含目录回退到 index.html）。
    #[cfg(target_os = "android")]
    let (bytes, mime_path) = {
        let rel2 = if rel.ends_with('/') {
            format!("{}index.html", rel)
        } else {
            rel.clone()
        };
        let bytes = WEBROOT.get_file(&rel2).map(|f| f.contents().to_vec());
        (bytes, rel2)
    };

    #[cfg(not(target_os = "android"))]
    let (bytes, mime_path) = {
        let root = repo_root();
        let full = root.join(&rel);

        let normalized = match full.canonicalize() {
            Ok(p) => p,
            Err(_) => {
                if !full.starts_with(root) {
                    return response(StatusCode::FORBIDDEN, "text/plain", b"Forbidden".to_vec());
                }
                return response(StatusCode::NOT_FOUND, "text/plain", b"Not found".to_vec());
            }
        };
        if !normalized.starts_with(root) {
            return response(StatusCode::FORBIDDEN, "text/plain", b"Forbidden".to_vec());
        }

        let target = if normalized.is_dir() {
            normalized.join("index.html")
        } else {
            normalized
        };
        let mime_path = target.to_str().unwrap_or("").to_string();
        (fs::read(&target).ok(), mime_path)
    };

    match bytes {
        Some(bytes) => {
            let mime = mime_for(&mime_path);
            if mime.starts_with("text/html") {
                let mut html = String::from_utf8_lossy(&bytes).to_string();
                if let Some(pos) = html.rfind("</body>") {
                    html.insert_str(pos, LINK_HANDLER);
                } else {
                    html.push_str(LINK_HANDLER);
                }
                response(StatusCode::OK, mime, html.into_bytes())
            } else {
                response(StatusCode::OK, mime, bytes)
            }
        }
        None => response(StatusCode::NOT_FOUND, "text/plain", b"Not found".to_vec()),
    }
}

#[cfg(all(test, not(target_os = "android")))]
mod tests {
    use super::*;

    #[test]
    fn materialize_seeds_webroot() {
        let dest = std::env::temp_dir().join(format!(
            "alpehuez-webroot-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dest);
        materialize(&dest);

        let expect_exists = [
            "index.html",
            "links.json",
            "fonts/Inter.woff2",
            "panel/index.html",
            "panel/panel.js",
            "myfiles/index.html",
            "myfiles/login.html",
            "myfiles/softwares/Windows Software Downloads.html",
            "myfiles/galibierhub/index.html",
        ];
        for rel in expect_exists {
            assert!(
                dest.join(rel).exists(),
                "嵌入资源缺失: {rel}（build.rs 是否已暂存 webroot？）"
            );
        }
        // 个人大报告目录不打包
        assert!(!dest.join("myfiles/targetc").exists());
        assert!(!dest.join("myfiles/global-oral").exists());

        // 打包版 data.json 不公开任何文件夹（个人报告不进包，softwares 已移出 My Files 界面）
        let data: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dest.join("myfiles/data.json")).expect("读取 data.json"),
        )
        .expect("解析 data.json");
        let folders = data["folders"]
            .as_array()
            .expect("folders 应为数组");
        assert!(folders.is_empty(), "打包版 My Files 不应列出文件夹");

        // 播种不应覆盖已存在的文件（用户数据保护）
        std::fs::write(dest.join("links.json"), b"user-modified").expect("写入 links.json");
        materialize(&dest);
        assert_eq!(
            std::fs::read_to_string(dest.join("links.json")).expect("读取 links.json"),
            "user-modified"
        );

        let _ = std::fs::remove_dir_all(&dest);
    }
}
