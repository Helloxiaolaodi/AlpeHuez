import { createHash } from 'node:crypto';
import { mkdir, writeFile, readFile, access } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const linksPath = path.join(here, 'links.json');
const iconsDir = path.join(here, 'icons');

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

function sourceKey(src) {
  try {
    const url = new URL(src);
    let key = url.hostname;
    if (!/\/favicon\.ico$/i.test(url.pathname) && url.pathname !== '/') {
      key += url.pathname.replace(/\/+$/, '').replace(/[^A-Za-z0-9._-]+/g, '-');
    }
    return key.toLowerCase().replace(/^\.+|\.+$/g, '') || 'icon';
  } catch {
    return createHash('sha1').update(src).digest('hex').slice(0, 12);
  }
}

function iconFileName(src, index) {
  const ext = /\.(png|jpe?g|gif|svg|webp|ico)$/i.test(src) ? src.match(/\.(png|jpe?g|gif|svg|webp|ico)$/i)[1].toLowerCase() : 'ico';
  return `${sourceKey(src)}-${index + 1}.${ext}`;
}

async function fetchWithRetry(url, retries = 2, timeoutMs = 8000) {
  let lastError;
  for (let attempt = 1; attempt <= retries; attempt += 1) {
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), timeoutMs);
      const res = await fetch(url, {
        redirect: 'follow',
        signal: controller.signal,
        headers: {
          'user-agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/124 Safari/537.36',
        },
      });
      clearTimeout(timer);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buffer = Buffer.from(await res.arrayBuffer());
      if (buffer.length < 64) throw new Error('Response too small to be an icon');
      return { buffer, finalUrl: res.url, contentType: res.headers.get('content-type') || '' };
    } catch (err) {
      lastError = err;
      if (attempt < retries) await sleep(attempt * 800);
    }
  }
  throw lastError;
}

function looksLikeHtml(fetched) {
  if (!fetched) return true;
  const head = fetched.buffer.subarray(0, 512).toString('utf8').trimStart();
  return head.startsWith('<') || /text\/html/i.test(fetched.contentType);
}

async function resolveFaviconFromPage(pageUrl) {
  const { buffer, finalUrl } = await fetchWithRetry(pageUrl);
  const html = buffer.toString('utf8');
  const patterns = [
    /<link[^>]+rel=["'][^"']*(?:shortcut\s+)?icon[^"']*["'][^>]*>/gi,
    /<link[^>]+rel=["'][^"']*apple-touch-icon[^"']*["'][^>]*>/gi,
  ];
  for (const pattern of patterns) {
    const match = html.match(pattern);
    if (match) {
      const hrefMatch = match[0].match(/href=["']([^"']+)["']/i);
      if (hrefMatch) {
        const iconUrl = new URL(hrefMatch[1], finalUrl).toString();
        try {
          return await fetchWithRetry(iconUrl);
        } catch {
          // Try the next candidate if the explicit icon fails.
        }
      }
    }
  }
  return null;
}

function fallbackCandidates(src) {
  const candidates = [];
  try {
    const host = new URL(src).hostname;
    candidates.push(`https://icon.horse/icon/${host}`);
    candidates.push(`https://www.google.com/s2/favicons?domain=${host}&sz=64`);
    candidates.push(`https://icons.duckduckgo.com/ip3/${host}.ico`);
    if (host === 'gist.github.com' || host === 'github.com') {
      candidates.push('https://github.githubassets.com/favicons/favicon.svg');
    }
    if (host === 'huggingface.co' || host === 'hugging-face.cn') {
      candidates.push('https://huggingface.co/front/assets/huggingface_logo-noborder.svg');
    }
    if (host === 'open.spotify.com') {
      candidates.push('https://open.spotifycdn.com/cdn/images/favicon32.b64e8c03.png');
    }
  } catch {
    // Fallback list cannot be derived; the caller still reports the original failure.
  }
  return candidates;
}

async function fetchFallback(src) {
  for (const candidate of fallbackCandidates(src)) {
    try {
      const result = await fetchWithRetry(candidate);
      if (!looksLikeHtml(result)) return result;
    } catch {
      // Try the next fallback source.
    }
  }
  return null;
}

async function main() {
  const links = JSON.parse(await readFile(linksPath, 'utf8'));
  const sources = [];
  const seen = new Set();
  for (const group of links.icons || []) {
    for (const item of group.children || []) {
      const src = item?.icon?.src;
      if (!src || src.startsWith('./icons/') || seen.has(src)) continue;
      seen.add(src);
      sources.push(src);
    }
  }

  await mkdir(iconsDir, { recursive: true });
  const results = [];
  const missing = [];

  const downloadOne = async (src, index) => {
    const fileName = iconFileName(src, index);
    const localPath = path.join(iconsDir, fileName);
    const relativePath = `./icons/${fileName}`;
    let fetched;

    try {
      const existingFiles = await Promise.all(
        ['.ico', '.png', '.jpg', '.jpeg', '.gif', '.svg', '.webp'].map(async (ext) => {
          const candidate = localPath.replace(/\.[^.]+$/, ext);
          try {
            await access(candidate);
            return candidate;
          } catch {
            return null;
          }
        }),
      );
      const existing = existingFiles.find(Boolean);
      if (existing) {
        const existingRelativePath = `./icons/${path.basename(existing)}`;
        results.push({ src, local: existingRelativePath, size: 0 });
        console.log(`SKIP ${String(index + 1).padStart(3)} ${existingRelativePath} <= ${src}`);
        return;
      }

      if (new URL(src).pathname.replace(/\/+$/, '') !== '' && !/favicon\.ico$/i.test(new URL(src).pathname)) {
        try {
          fetched = await resolveFaviconFromPage(src);
        } catch {
          // The direct page may still expose a favicon later in the HTML.
        }
      }
      if (!fetched) {
        try {
          fetched = await fetchWithRetry(src);
        } catch {
          // Fall back to parsing the page or an icon service below.
        }
      }
      if (looksLikeHtml(fetched)) {
        try {
          const pageFetched = await resolveFaviconFromPage(src);
          if (pageFetched) fetched = pageFetched;
        } catch {
          // Continue to the icon service fallback.
        }
      }
      if (looksLikeHtml(fetched)) {
        try {
          const fallbackFetched = await fetchFallback(src);
          if (fallbackFetched) fetched = fallbackFetched;
        } catch {
          // Report the original source as missing below.
        }
      }
      if (!fetched || looksLikeHtml(fetched)) throw new Error('No usable icon response after fallbacks');
      const guessedExt = fetched.contentType.match(/image\/(png|jpeg|gif|svg\+xml|webp|x-icon)/i)?.[1] || '';
      const finalExt = ['png', 'jpeg', 'gif', 'svg+xml', 'webp', 'x-icon'].includes(guessedExt)
        ? (guessedExt === 'x-icon' ? 'ico' : guessedExt === 'svg+xml' ? 'svg' : guessedExt === 'jpeg' ? 'jpg' : guessedExt)
        : 'ico';
      const finalFileName = fileName.replace(/\.[^.]+$/, `.${finalExt}`);
      const finalPath = path.join(iconsDir, finalFileName);
      const finalRelativePath = `./icons/${finalFileName}`;
      await writeFile(finalPath, fetched.buffer);
      results.push({ src, local: finalRelativePath, size: fetched.buffer.length });
      console.log(`OK ${String(index + 1).padStart(3)} ${finalRelativePath} <= ${src} (${fetched.buffer.length} bytes)`);
    } catch (err) {
      missing.push(src);
      console.error(`FAIL ${String(index + 1).padStart(3)} ${src}: ${err.message}`);
    }
  };

  const concurrency = 8;
  const queue = sources.map((src, index) => ({ src, index }));
  const workers = Array.from({ length: concurrency }, async () => {
    while (queue.length > 0) {
      const next = queue.shift();
      if (next) await downloadOne(next.src, next.index);
    }
  });
  await Promise.all(workers);

  const bySource = new Map(results.map((r) => [r.src, r.local]));
  let rewritten = 0;
  for (const group of links.icons || []) {
    for (const item of group.children || []) {
      const src = item?.icon?.src;
      if (src && bySource.has(src)) {
        item.icon.src = bySource.get(src);
        rewritten += 1;
      }
    }
  }

  if (rewritten > 0) {
    const json = JSON.stringify(links, null, 4) + '\n';
    const md5 = createHash('md5').update(json).digest('hex');
    links.md5 = md5;
    await writeFile(linksPath, JSON.stringify(links, null, 4) + '\n', 'utf8');
  }

  console.log(`\nDownloaded ${results.length}/${sources.length}, rewritten ${rewritten} icon references.`);
  if (missing.length) {
    console.log(`Missing ${missing.length}:`);
    missing.forEach((src) => console.log(`  ${src}`));
  }
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
