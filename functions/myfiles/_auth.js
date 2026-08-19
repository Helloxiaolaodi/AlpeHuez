export const COOKIE_NAME = 'myfiles_auth_v2';

// Read the access password from the Cloudflare Pages environment variable.
// Never commit a real password to this public repository.
export function getAccessPassword(context) {
  return String((context && context.env && context.env.ALPEHUZ_ACCESS_PASSWORD) || '');
}

export function getCookieValue(context) {
  return btoa(getAccessPassword(context));
}
