import { COOKIE_NAME, getCookieValue } from './_auth.js';

// 报告区已有各自独立的 middleware 处理，根 middleware 直接放行，
// 避免同一路径触发两套 cookie 检查导致二次登录。
const AREA_PREFIXES = [
  '/myfiles/targetc/',
  '/myfiles/lucuro/',
  '/myfiles/galibierhub/',
  '/myfiles/global-oral/',
];

// 共享资源与公开页面：explorer 资源被所有受保护页面加载（收紧会破坏受保护页）；
// 软件下载页及其数据保持公开。Cloudflare Pages 会把 .html 请求 308 到无扩展名路径，
// 因此软件页同时登记带 .html 与不带 .html 两种形式。
const PUBLIC_PATHS = new Set([
  '/myfiles/login.html',
  '/myfiles/login',
  '/myfiles/explorer.css',
  '/myfiles/explorer.js',
  '/myfiles/softwares/Windows Software Downloads.html',
  '/myfiles/softwares/Windows Software Downloads',
  '/myfiles/softwares/software-data.json',
]);

export async function onRequest(context) {
  const { request, next } = context;
  const url = new URL(request.url);
  // pathname 是百分号编码形式（如 Windows%20Software），需解码后才能与含空格的公开路径匹配。
  let path = url.pathname;
  try { path = decodeURIComponent(path); } catch { /* 非法编码时保留原样 */ }

  // 登录页与登录接口必须绕过，否则会无限重定向。
  if (path.endsWith('/login.html') || path.endsWith('/login')) return next();

  for (const prefix of AREA_PREFIXES) {
    if (path.startsWith(prefix)) return next();
  }

  if (PUBLIC_PATHS.has(path)) return next();

  const cookieHeader = request.headers.get('Cookie') || '';
  const authed = cookieHeader
    .split(';')
    .map((c) => c.trim())
    .includes(`${COOKIE_NAME}=${getCookieValue(context)}`);

  if (authed) {
    return next();
  }

  const loginUrl = `${url.origin}/myfiles/login.html?next=${encodeURIComponent(path)}`;
  return Response.redirect(loginUrl, 302);
}
