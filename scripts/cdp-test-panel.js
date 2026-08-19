// CDP 驱动 Edge 无头浏览器，测试面板「新建文件」保存流程
const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const EDGE = 'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe';
const PORT = 9224;
const USER_DATA = path.join(process.env.TEMP, 'edge-cdp-test2-' + Date.now());

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
  await send('Page.navigate', { url: 'http://127.0.0.1:5173/panel/' });
  await new Promise(r => setTimeout(r, 2500));

  // 切到 My Files 标签
  const tabClicked = await ev(`(() => {
    const b = Array.from(document.querySelectorAll('[data-tab]')).find(x => x.dataset.tab === 'myfiles');
    if (b) { b.click(); return true; } return false;
  })()`);
  console.log('myfiles tab clicked:', tabClicked);
  await new Promise(r => setTimeout(r, 500));

  const hasNewFile = await ev(`!!document.getElementById('btnNewFile')`);
  console.log('btnNewFile exists:', hasNewFile);
  if (!hasNewFile) { console.log('body:', (await ev('document.body.innerText')).slice(0, 300)); return; }

  await ev(`document.getElementById('btnNewFile').click()`);
  await new Promise(r => setTimeout(r, 500));
  console.log('modal visible:', await ev(`!document.getElementById('modalBackdrop').hidden`));

  await ev(`document.getElementById('f_name').value = 'CDP Test File'`);
  await new Promise(r => setTimeout(r, 200));
  await ev(`document.getElementById('modalOk').click()`);
  await new Promise(r => setTimeout(r, 800));

  console.log('modal closed:', await ev(`document.getElementById('modalBackdrop').hidden`));
  console.log('toast:', await ev(`document.getElementById('toast').textContent`));

  const data = JSON.parse(fs.readFileSync(path.join(__dirname, '..', 'myfiles', 'data.json'), 'utf8'));
  const found = data.folders.some(f => (f.files || []).some(x => x.name === 'CDP Test File'));
  console.log('file saved:', found);
  if (found) {
    for (const f of data.folders) f.files = (f.files || []).filter(x => x.name !== 'CDP Test File');
    fs.writeFileSync(path.join(__dirname, '..', 'myfiles', 'data.json'), JSON.stringify(data, null, 4) + '\n');
    console.log('cleaned');
  }
  process.exit(0);
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });
