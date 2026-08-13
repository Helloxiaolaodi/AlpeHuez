export async function onRequest(context) {
  const { request, next } = context;
  const url = new URL(request.url);

  // 登录页与登录接口本身不需要校验，否则会死循环
  const path = url.pathname;
  if (path.endsWith('/login.html') || path.endsWith('/login')) {
    return next();
  }

  const COOKIE_NAME = 'targetc_auth';
  const COOKIE_VALUE = btoa('yanglun');
  const cookieHeader = request.headers.get('Cookie') || '';
  const authed = cookieHeader
    .split(';')
    .map((c) => c.trim())
    .includes(`${COOKIE_NAME}=${COOKIE_VALUE}`);

  if (authed) {
    return next();
  }

  const loginUrl = `${url.origin}/myfiles/targetc/login.html?next=${encodeURIComponent(path)}`;
  return Response.redirect(loginUrl, 302);
}
