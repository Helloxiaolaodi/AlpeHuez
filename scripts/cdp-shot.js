// 截图：主页面滚动后标题栏区域
const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const THORIUM = 'C:\\Users\\Lenovo\\AppData\\Local\\Thorium\\Application\\thorium.exe';
const PORT = 9226;
const USER_DATA = path.join(process.env.TEMP, 'thorium-cdp-shot-' + Date.now());

const browser = spawn(THORIUM, [
  '--headless=new', '--remote-debugging-port=' + PORT,
  '--user-data-dir=' + USER_DATA, '--no-first-run', '--disable-gpu',
  '--window-size=1280,800', 'about:blank'
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
  await send('Emulation.setDeviceMetricsOverride', { width: 1280, height: 800, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url: 'http://127.0.0.1:5173/' });
  await new Promise(r => setTimeout(r, 3000));
  await ev(`(() => { const v = document.querySelector('.view.active'); v.scrollTop = 400; return v.scrollTop; })()`);
  await new Promise(r => setTimeout(r, 600));
  const shot = await send('Page.captureScreenshot', { format: 'jpeg', quality: 80 });
  fs.writeFileSync(path.join(process.env.TEMP, 'titlebar-bleed.jpg'), Buffer.from(shot.data, 'base64'));
  console.log('screenshot saved to', path.join(process.env.TEMP, 'titlebar-bleed.jpg'));
  process.exit(0);
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });
