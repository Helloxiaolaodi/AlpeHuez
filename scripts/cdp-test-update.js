// CDP 驱动 Edge 无头浏览器，验证主应用 About 页「检查更新」按钮 + 更新弹窗
const { spawn } = require('child_process');
const path = require('path');

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const PORT = 9225;
const USER_DATA = path.join(process.env.TEMP, 'edge-cdp-update-' + Date.now());

const edge = spawn(EDGE, [
  '--headless=new', '--remote-debugging-port=' + PORT,
  '--user-data-dir=' + USER_DATA, '--no-first-run', '--disable-gpu', 'about:blank'
], { stdio: 'ignore' });

let ws, msgId = 0;
const pending = new Map();
function send(method, params = {}) {
  return new Promise((res, rej) => { const id = ++msgId; pending.set(id, { res, rej }); ws.send(JSON.stringify({ id, method, params })); });
}
async function waitForWs() {
  for (let i = 0; i < 50; i++) {
    try { const t = await (await fetch('http://127.0.0.1:' + PORT + '/json')).json(); const p = t.find(x => x.type === 'page'); if (p) return p.webSocketDebuggerUrl; } catch (e) {}
    await new Promise(r => setTimeout(r, 200));
  }
  throw new Error('no cdp');
}
async function ev(expr) {
  const r = await send('Runtime.evaluate', { expression: expr, awaitPromise: true, returnByValue: true });
  if (r.exceptionDetails) throw new Error('JS: ' + (r.exceptionDetails.exception?.description || r.exceptionDetails.text));
  return r.result?.value;
}

(async () => {
  const url = await waitForWs();
  ws = new WebSocket(url);
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
  ws.onmessage = (ev) => { const m = JSON.parse(ev.data); if (m.id && pending.has(m.id)) { const p = pending.get(m.id); pending.delete(m.id); m.error ? p.rej(new Error(m.error.message)) : p.res(m.result); } };
  await send('Page.enable'); await send('Runtime.enable');
  await send('Page.navigate', { url: 'http://127.0.0.1:5173/index.html' });
  await new Promise(r => setTimeout(r, 3500));

  // 切到 About 视图
  const aboutClicked = await ev(`(() => {
    const b = Array.from(document.querySelectorAll('[data-view]')).find(x => x.dataset.view === 'about');
    if (b) { b.click(); return true; } return false;
  })()`);
  console.log('about view clicked:', aboutClicked);
  await new Promise(r => setTimeout(r, 500));

  console.log('btnCheckUpdate exists:', await ev(`!!document.getElementById('btnCheckUpdate')`));
  console.log('updateModal exists:', await ev(`!!document.getElementById('updateModal')`));
  console.log('updateModal hidden:', await ev(`document.getElementById('updateModal').hidden`));
  console.log('versionChip text:', await ev(`document.getElementById('versionChip').textContent`));

  // 无 Tauri 环境：点击按钮应走降级提示（toast）
  await ev(`document.getElementById('btnCheckUpdate').click()`);
  await new Promise(r => setTimeout(r, 600));
  console.log('toast after click:', await ev(`document.querySelector('.toast') ? document.querySelector('.toast').textContent : '(none)'`));

  // 手动打开弹窗验证结构
  await ev(`(() => { const m = document.getElementById('updateModal'); m.hidden = false; document.getElementById('updateCurrentVer').textContent = 'v0.5.0'; document.getElementById('updateLatestVer').textContent = 'v0.5.1'; document.getElementById('updateNotes').textContent = 'test notes'; })()`);
  await new Promise(r => setTimeout(r, 300));
  console.log('modal visible:', await ev(`!document.getElementById('updateModal').hidden`));
  console.log('install btn text:', await ev(`document.getElementById('updateInstallBtn').textContent`));
  console.log('install btn primary class:', await ev(`document.getElementById('updateInstallBtn').classList.contains('primary')`));
  console.log('notes text:', await ev(`document.getElementById('updateNotes').textContent`));

  // 关闭按钮
  await ev(`document.getElementById('updateModalClose').click()`);
  await new Promise(r => setTimeout(r, 200));
  console.log('modal closed after close btn:', await ev(`document.getElementById('updateModal').hidden`));

  process.exit(0);
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });
