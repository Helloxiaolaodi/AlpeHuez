import { COOKIE_NAME, getAccessPassword, getCookieValue } from './_auth.js';

export async function onRequestPost(context) {
  const { request } = context;
  const url = new URL(request.url);

  const form = await request.formData().catch(() => null);
  if (!form) {
    return new Response('Bad request', { status: 400 });
  }

  const password = form.get('password') || '';
  let next = form.get('next') || '/myfiles/galibierhub/';

  if (!next.startsWith('/')) {
    next = '/myfiles/galibierhub/';
  }

  if (password !== getAccessPassword(context)) {
    return new Response(null, {
      status: 302,
      headers: {
        Location: `/myfiles/galibierhub/login.html?error=1&next=${encodeURIComponent(next)}`,
      },
    });
  }

  return new Response(null, {
    status: 302,
    headers: {
      Location: next,
      'Set-Cookie': `${COOKIE_NAME}=${getCookieValue(context)}; Path=/; SameSite=Lax; Secure`,
    },
  });
}
