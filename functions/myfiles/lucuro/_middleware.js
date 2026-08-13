import { COOKIE_NAME, COOKIE_VALUE } from './_auth.js';

export async function onRequest(context) {
  const { request, next } = context;
  const url = new URL(request.url);

  const path = url.pathname;
  if (path.endsWith('/login.html') || path.endsWith('/login')) {
    return next();
  }

  const cookieHeader = request.headers.get('Cookie') || '';
  const authed = cookieHeader
    .split(';')
    .map((c) => c.trim())
    .includes(`${COOKIE_NAME}=${COOKIE_VALUE}`);

  if (authed) {
    return next();
  }

  const loginUrl = `${url.origin}/myfiles/lucuro/login.html?next=${encodeURIComponent(path)}`;
  return Response.redirect(loginUrl, 302);
}
