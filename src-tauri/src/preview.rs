use std::fs;
use std::path::Path;

use percent_encoding::percent_decode_str;
use tauri::http::{Request, Response, StatusCode};
use tauri::UriSchemeContext;

use crate::repo_root;

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
/// open_url 命令，用系统默认浏览器打开；普通浏览器（Cloudflare 部署）下不生效。
const LINK_HANDLER: &str = r#"<script>
(function () {
  if (!window.__TAURI__) return;
  var invoke = window.__TAURI__.core.invoke;
  function openExternal(url) {
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
        openExternal(href);
      }
    }
  }, true);
  var origOpen = window.open;
  window.open = function (url) {
    if (url && typeof url === 'string') {
      if (/^https?:/i.test(url)) { openExternal(url); return null; }
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
        "index.html"
    } else {
        decoded.trim_start_matches('/')
    };
    let root = repo_root();
    let full = root.join(rel);

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

    match fs::read(&target) {
        Ok(bytes) => {
            let mime = mime_for(target.to_str().unwrap_or(""));
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
        Err(_) => response(StatusCode::NOT_FOUND, "text/plain", b"Not found".to_vec()),
    }
}
