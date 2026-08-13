import { PASSWORD, COOKIE_NAME, COOKIE_VALUE } from './_auth.js';

export async function onRequestPost(context) {
  const { request } = context;
  const url = new URL(request.url);

  const form = await request.formData().catch(() => null);
  if (!form) {
    return new Response('Bad request', { status: 400 });
  }

  const password = form.get('password') || '';
  let next = form.get('next') || '/myfiles/targetc/TargetC-phenotypes-analysis-260814.html';

  if (!next.startsWith('/')) {
    next = '/myfiles/targetc/TargetC-phenotypes-analysis-260814.html';
  }

  if (password !== PASSWORD) {
    return new Response(null, {
      status: 302,
      headers: {
        Location: `/myfiles/targetc/login.html?error=1&next=${encodeURIComponent(next)}`,
      },
    });
  }

  return new Response(null, {
    status: 302,
    headers: {
      Location: next,
      'Set-Cookie': `${COOKIE_NAME}=${COOKIE_VALUE}; Path=/; Max-Age=604800; SameSite=Lax; Secure`,
    },
  });
}
