// CDP 驱动 Thorium 无头浏览器，验证 web showcase 行为。
// 用法: node scripts/cdp-test-showcase.js [url]  默认 https://20211003.xyz/
const { spawn } = require('child_process');
const path = require('path');

const TARGET_URL = process.argv[2] || 'https://20211003.xyz/';

const THORIUM = 'C:\\Users\\Lenovo\\AppData\\Local\\Thorium\\Application\\thorium.exe';
const PORT = 9226;
const USER_DATA = path.join(process.env.TEMP, 'thorium-cdp-showcase-' + Date.now());

const browser = spawn(THORIUM, [
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
  await send('Page.navigate', { url: TARGET_URL });
  await new Promise(r => setTimeout(r, 5000));

  console.log('hostname:', await ev('location.hostname'));
  console.log('versionChip:', await ev(`document.getElementById('versionChip').textContent`));
  console.log('tab-sidebar exists:', await ev(`!!document.querySelector('.tab-sidebar')`));
  console.log('rail buttons:', await ev(`Array.from(document.querySelectorAll('.rail-btn')).map(b => b.id).join(',')`));
  console.log('view count:', await ev(`document.querySelectorAll('.view').length`));
  console.log('views:', await ev(`Array.from(document.querySelectorAll('.view')).map(v => v.id).join(',')`));

  // 截图首页
  await send('Page.captureScreenshot', { format: 'png' }).then(r => {
    require('fs').writeFileSync(path.join(__dirname, '..', '.tmp-shots', 'showcase-home.png'), Buffer.from(r.data, 'base64'));
    console.log('screenshot saved: showcase-home.png');
  }).catch(e => console.log('screenshot failed:', e.message));

  // 切到 My Files
  await ev(`document.getElementById('btnMyFiles').click()`);
  await new Promise(r => setTimeout(r, 800));
  console.log('--- My Files ---');
  console.log('myFilesMain hidden:', await ev(`document.getElementById('myFilesMain').hidden`));
  console.log('myFilesWebLock visible:', await ev(`!document.getElementById('myFilesWebLock').hidden`));
  console.log('lock text:', await ev(`document.getElementById('myFilesWebLock').innerText.slice(0, 120)`));

  // 切到 Dev Panel（通过 dev-tool 按钮）
  await ev(`(() => { const b = document.querySelector('.dev-tool-btn'); if (b) { b.click(); return true; } return false; })()`);
  await new Promise(r => setTimeout(r, 800));
  console.log('--- Dev Panel ---');
  console.log('devFrame display:', await ev(`getComputedStyle(document.getElementById('devFrame')).display`));
  console.log('devWebLock visible:', await ev(`!document.getElementById('devWebLock').hidden`));
  console.log('lock text:', await ev(`document.getElementById('devWebLock').innerText.slice(0, 120)`));

  // 切回 Home
  await ev(`document.getElementById('btnHome').click()`);
  await new Promise(r => setTimeout(r, 500));
  console.log('--- Home ---');
  console.log('home active:', await ev(`document.getElementById('view-home').classList.contains('active')`));
  console.log('portal cards:', await ev(`document.querySelectorAll('.nav-card').length`));

  process.exit(0);
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });
