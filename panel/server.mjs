import { createServer } from 'node:http';
import { readFile, writeFile, mkdir, stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { spawn } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const here = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(here, '..');
const PORT = Number(process.env.PORT || 5173);
const HOST = '127.0.0.1';

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon',
  '.txt': 'text/plain; charset=utf-8',
  '.md': 'text/markdown; charset=utf-8',
  '.qmd': 'text/plain; charset=utf-8',
  '.zip': 'application/zip',
  '.pdf': 'application/pdf',
};

const SCRIPTS = {
  download_icons: 'download_icons.mjs',
  enhance_links: 'enhance_links.mjs',
  repair_icons: 'repair_icons.mjs',
};

function sendJson(res, status, obj) {
  res.writeHead(status, { 'Content-Type': 'application/json; charset=utf-8' });
  res.end(JSON.stringify(obj));
}

function readBody(req) {
  return new Promise((resolve, reject) => {
    let data = '';
    req.on('data', (chunk) => {
      data += chunk;
      if (data.length > 50e6) {
        req.destroy();
        reject(new Error('请求体过大'));
      }
    });
    req.on('end', () => resolve(data));
    req.on('error', reject);
  });
}

function runProcess(cmd, args, cwd, timeoutMs = 120000) {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, { cwd });
    let output = '';
    let timedOut = false;
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill();
    }, timeoutMs);
    child.stdout.on('data', (d) => { output += d; });
    child.stderr.on('data', (d) => { output += d; });
    child.on('close', (code) => {
      clearTimeout(timer);
      resolve({ code, output, timedOut });
    });
    child.on('error', (err) => {
      clearTimeout(timer);
      resolve({ code: -1, output: String(err.message || err), timedOut });
    });
  });
}

async function serveStatic(req, res, urlPath) {
  const rel = urlPath === '/' ? 'index.html' : urlPath.replace(/^\/+/, '');
  const filePath = path.resolve(repoRoot, rel);
  if (filePath !== repoRoot && !filePath.startsWith(repoRoot + path.sep)) {
    res.writeHead(403);
    res.end('Forbidden');
    return;
  }
  try {
    let target = filePath;
    const info = await stat(target);
    if (info.isDirectory()) {
      target = path.join(target, 'index.html');
    }
    const data = await readFile(target);
    const ext = path.extname(target).toLowerCase();
    res.writeHead(200, { 'Content-Type': MIME[ext] || 'application/octet-stream' });
    res.end(data);
  } catch {
    res.writeHead(404);
    res.end('Not found');
  }
}

function recomputeLinksMd5(links) {
  const { md5: _old, ...rest } = links;
  const json = JSON.stringify(rest, null, 4) + '\n';
  return createHash('md5').update(json).digest('hex');
}

async function handleApi(req, res, url) {
  const route = url.pathname;

  if (route === '/api/links' && req.method === 'GET') {
    const data = JSON.parse(await readFile(path.join(repoRoot, 'links.json'), 'utf8'));
    return sendJson(res, 200, { ok: true, data });
  }

  if (route === '/api/links' && req.method === 'POST') {
    const body = JSON.parse(await readBody(req));
    const links = body.data;
    if (!links || !Array.isArray(links.icons)) {
      return sendJson(res, 400, { ok: false, error: '数据格式不正确：缺少 icons 数组' });
    }
    for (const group of links.icons) {
      for (const item of group.children || []) {
        if (!item.title || !item.url) {
          return sendJson(res, 400, { ok: false, error: `卡片「${item.title || '(无标题)'}」缺少标题或 URL` });
        }
      }
    }
    links.md5 = recomputeLinksMd5(links);
    await writeFile(path.join(repoRoot, 'links.json'), JSON.stringify(links, null, 4) + '\n', 'utf8');
    return sendJson(res, 200, { ok: true, md5: links.md5 });
  }

  if (route === '/api/myfiles' && req.method === 'GET') {
    const data = JSON.parse(await readFile(path.join(repoRoot, 'myfiles', 'data.json'), 'utf8'));
    return sendJson(res, 200, { ok: true, data });
  }

  if (route === '/api/myfiles' && req.method === 'POST') {
    const body = JSON.parse(await readBody(req));
    const data = body.data;
    if (!data || !Array.isArray(data.folders)) {
      return sendJson(res, 400, { ok: false, error: '数据格式不正确：缺少 folders 数组' });
    }
    for (const folder of data.folders) {
      if (!folder.slug || !folder.name) {
        return sendJson(res, 400, { ok: false, error: `文件夹「${folder.name || folder.slug || '(无名称)'}」缺少 slug 或名称` });
      }
    }
    await writeFile(path.join(repoRoot, 'myfiles', 'data.json'), JSON.stringify(data, null, 4) + '\n', 'utf8');
    return sendJson(res, 200, { ok: true });
  }

  if (route === '/api/software' && req.method === 'GET') {
    const data = JSON.parse(await readFile(path.join(repoRoot, 'myfiles', 'softwares', 'software-data.json'), 'utf8'));
    return sendJson(res, 200, { ok: true, data });
  }

  if (route === '/api/software' && req.method === 'POST') {
    const body = JSON.parse(await readBody(req));
    const data = body.data;
    if (!data || !Array.isArray(data.categories) || !Array.isArray(data.software)) {
      return sendJson(res, 400, { ok: false, error: '数据格式不正确：缺少 categories/software 数组' });
    }
    await writeFile(path.join(repoRoot, 'myfiles', 'softwares', 'software-data.json'), JSON.stringify(data, null, 4) + '\n', 'utf8');
    return sendJson(res, 200, { ok: true });
  }

  if (route === '/api/run-script' && req.method === 'POST') {
    const body = JSON.parse(await readBody(req));
    const file = SCRIPTS[body.script];
    if (!file) {
      return sendJson(res, 400, { ok: false, error: '未知脚本' });
    }
    const result = await runProcess(process.execPath, [file], repoRoot, 300000);
    return sendJson(res, 200, { ok: result.code === 0, code: result.code, output: result.output, timedOut: result.timedOut });
  }

  if (route === '/api/create-folder' && req.method === 'POST') {
    const body = JSON.parse(await readBody(req));
    const slug = String(body.slug || '').trim().toLowerCase();
    const name = String(body.name || '').trim();
    const isProtected = Boolean(body.protected);
    if (!/^[a-z0-9-]+$/.test(slug)) {
      return sendJson(res, 400, { ok: false, error: 'slug 只能包含小写字母、数字和连字符（如 my-folder）' });
    }
    if (!name) {
      return sendJson(res, 400, { ok: false, error: '请填写文件夹名称' });
    }
    const folderDir = path.join(repoRoot, 'myfiles', slug);
    await mkdir(folderDir, { recursive: true });

    const indexTemplate = await readFile(path.join(repoRoot, 'myfiles', 'softwares', 'index.html'), 'utf8');
    await writeFile(path.join(folderDir, 'index.html'), indexTemplate);

    if (isProtected) {
      const loginTemplate = await readFile(path.join(repoRoot, 'myfiles', 'targetc', 'login.html'), 'utf8');
      let loginHtml = loginTemplate.replaceAll('/myfiles/targetc', `/myfiles/${slug}`);
      loginHtml = loginHtml.replace('TargetC Data Analysis', name);
      loginHtml = loginHtml.replace(`/myfiles/${slug}/TargetC-phenotypes-analysis-260814.html`, `/myfiles/${slug}/`);
      await writeFile(path.join(folderDir, 'login.html'), loginHtml);

      const funcDir = path.join(repoRoot, 'functions', 'myfiles', slug);
      await mkdir(funcDir, { recursive: true });

      const authTemplate = await readFile(path.join(repoRoot, 'functions', 'myfiles', 'targetc', '_auth.js'), 'utf8');
      await writeFile(path.join(funcDir, '_auth.js'), authTemplate.replace("'targetc_auth_v2'", `'${slug}_auth_v2'`));

      const loginJsTemplate = await readFile(path.join(repoRoot, 'functions', 'myfiles', 'targetc', 'login.js'), 'utf8');
      let loginJs = loginJsTemplate.replaceAll('/myfiles/targetc', `/myfiles/${slug}`);
      loginJs = loginJs.replaceAll(`/myfiles/${slug}/TargetC-phenotypes-analysis-260814.html`, `/myfiles/${slug}/`);
      await writeFile(path.join(funcDir, 'login.js'), loginJs);

      const middlewareTemplate = await readFile(path.join(repoRoot, 'functions', 'myfiles', 'targetc', '_middleware.js'), 'utf8');
      await writeFile(path.join(funcDir, '_middleware.js'), middlewareTemplate.replaceAll('/myfiles/targetc', `/myfiles/${slug}`));
    }

    return sendJson(res, 200, { ok: true });
  }

  if (route === '/api/git-status' && req.method === 'GET') {
    const [status, branch, last] = await Promise.all([
      runProcess('git', ['status', '--short'], repoRoot),
      runProcess('git', ['branch', '--show-current'], repoRoot),
      runProcess('git', ['log', '-1', '--oneline'], repoRoot),
    ]);
    return sendJson(res, 200, {
      ok: true,
      status: status.output.trim(),
      branch: branch.output.trim(),
      last: last.output.trim(),
    });
  }

  if (route === '/api/git-push' && req.method === 'POST') {
    const body = JSON.parse(await readBody(req));
    const message = String(body.message || '').trim() || 'Update site content';
    const add = await runProcess('git', ['add', '-A'], repoRoot);
    const commit = await runProcess('git', ['commit', '-m', message], repoRoot);
    const push = await runProcess('git', ['push'], repoRoot, 120000);
    return sendJson(res, 200, {
      ok: true,
      add: add.output,
      commit: commit.output,
      commitCode: commit.code,
      push: push.output,
      pushCode: push.code,
    });
  }

  return sendJson(res, 404, { ok: false, error: '接口不存在' });
}

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, `http://${req.headers.host || 'localhost'}`);
    if (url.pathname.startsWith('/api/')) {
      await handleApi(req, res, url);
    } else if (req.method === 'GET' || req.method === 'HEAD') {
      await serveStatic(req, res, url.pathname);
    } else {
      res.writeHead(405);
      res.end('Method not allowed');
    }
  } catch (err) {
    sendJson(res, 500, { ok: false, error: String(err.message || err) });
  }
});

server.on('error', (err) => {
  if (err.code === 'EADDRINUSE') {
    console.error('');
    console.error(`端口 ${PORT} 已被占用。`);
    console.error('可能面板已经在运行，或另一个程序占用了该端口。');
    console.error(`如需换端口：PORT=${PORT + 1} node panel/server.mjs`);
    console.error('');
    process.exit(1);
  }
  throw err;
});

server.listen(PORT, HOST, () => {
  console.log('');
  console.log('  my-nav 开发者面板已启动');
  console.log(`  编辑面板:  http://${HOST}:${PORT}/panel/`);
  console.log(`  网站预览:  http://${HOST}:${PORT}/`);
  console.log('');
});
