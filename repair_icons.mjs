import { createHash } from 'node:crypto';
import { readFile, writeFile, unlink } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const linksPath = path.join(here, 'links.json');
const iconsDir = path.join(here, 'icons');

function looksLikeRealImage(buffer) {
  if (buffer.length < 8) return false;
  const head = buffer.subarray(0, 256).toString('utf8').trimStart();
  if (head.startsWith('<!doctype') || head.startsWith('<!DOCTYPE') || head.startsWith('<html') || head.startsWith('<head')) return false;
  if (buffer[0] === 0x89 && buffer[1] === 0x50 && buffer[2] === 0x4e && buffer[3] === 0x47) return true;
  if (buffer[0] === 0xff && buffer[1] === 0xd8 && buffer[2] === 0xff) return true;
  if (buffer[0] === 0x47 && buffer[1] === 0x49 && buffer[2] === 0x46) return true;
  if (head.startsWith('<?xml') || head.startsWith('<svg')) return true;
  if (buffer[0] === 0x00 && buffer[1] === 0x00 && (buffer[2] === 0x01 || buffer[2] === 0x02) && buffer[3] === 0x00) return true;
  return false;
}

async function fetchImage(url) {
  for (let attempt = 1; attempt <= 3; attempt += 1) {
    try {
      const res = await fetch(url, {
        redirect: 'follow',
        signal: AbortSignal.timeout(15000),
        headers: { 'user-agent': 'Mozilla/5.0' },
      });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const buffer = Buffer.from(await res.arrayBuffer());
      if (!looksLikeRealImage(buffer)) throw new Error('HTML or unknown response');
      return { buffer, contentType: res.headers.get('content-type') || '' };
    } catch (err) {
      if (attempt === 3) throw err;
    }
  }
  throw new Error('unreachable');
}

async function main() {
  const links = JSON.parse(await readFile(linksPath, 'utf8'));
  let replaced = 0;

  for (const group of links.icons || []) {
    for (const item of group.children || []) {
      const src = item?.icon?.src;
      if (!src || !src.startsWith('./icons/')) continue;
      const relPath = src.replace(/^\.\//, '');
      const filePath = path.join(iconsDir, path.basename(relPath));
      const buffer = await readFile(filePath);
      if (looksLikeRealImage(buffer)) continue;

      const host = path.basename(src).split('-')[0];
      const candidates = [
        `https://icon.horse/icon/${host}`,
        `https://www.google.com/s2/favicons?domain=${host}&sz=64`,
      ];
      let fetched = null;
      let lastError = null;
      for (const candidate of candidates) {
        try {
          fetched = await fetchImage(candidate);
          break;
        } catch (err) {
          lastError = err;
        }
      }
      if (!fetched) {
        console.error(`FAIL ${src} <= ${host}: ${lastError?.message}`);
        continue;
      }

      const ext = fetched.contentType.includes('png')
        ? 'png'
        : fetched.contentType.includes('jpeg')
          ? 'jpg'
          : fetched.contentType.includes('gif')
            ? 'gif'
            : fetched.contentType.includes('svg')
              ? 'svg'
              : 'ico';
      const newFileName = path.basename(src).replace(/\.[^.]+$/, `.${ext}`);
      const newFilePath = path.join(iconsDir, newFileName);
      await unlink(filePath);
      await writeFile(newFilePath, fetched.buffer);
      item.icon.src = `./icons/${newFileName}`;
      replaced += 1;
      console.log(`REPLACED ${src} -> ./icons/${newFileName} (${fetched.buffer.length} bytes)`);
    }
  }

  if (replaced > 0) {
    const json = JSON.stringify(links, null, 4) + '\n';
    links.md5 = createHash('md5').update(json).digest('hex');
    await writeFile(linksPath, JSON.stringify(links, null, 4) + '\n', 'utf8');
  }
  console.log(`replaced=${replaced}`);
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
