// CDP 检查主页面滚动时内容是否与 topbar 重叠（标题栏穿模）
const { spawn } = require('child_process');
const path = require('path');

const THORIUM = 'C:\\Users\\Lenovo\\AppData\\Local\\Thorium\\Application\\thorium.exe';
const PORT = 9225;
const USER_DATA = path.join(process.env.TEMP, 'thorium-cdp-test3-' + Date.now());

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
  await send('Page.navigate', { url: 'http://127.0.0.1:5173/' });
  await new Promise(r => setTimeout(r, 3000));

  const info = await ev(`(() => {
    const topbar = document.querySelector('.topbar');
    const view = document.querySelector('.view.active');
    const titleBar = document.getElementById('titleBar');
    const shell = document.querySelector('.app-shell');
    const tb = topbar.getBoundingClientRect();
    const v = view.getBoundingClientRect();
    const t = titleBar ? titleBar.getBoundingClientRect() : null;
    return {
      noTitlebar: document.documentElement.classList.contains('no-titlebar'),
      topbar: { top: tb.top, bottom: tb.bottom, height: tb.height, bg: getComputedStyle(topbar).backgroundColor, z: getComputedStyle(topbar).zIndex },
      view: { top: v.top, bottom: v.bottom, overflow: getComputedStyle(view).overflowY },
      titleBar: t ? { top: t.top, bottom: t.bottom, bg: getComputedStyle(titleBar).backgroundColor, display: getComputedStyle(titleBar).display } : null,
      shell: { top: shell.getBoundingClientRect().top, height: shell.getBoundingClientRect().height }
    };
  })()`);
  console.log('Layout:', JSON.stringify(info, null, 2));

  // 滚动 home view 到底部
  await ev(`(() => { const v = document.querySelector('.view.active'); v.scrollTop = v.scrollHeight; return v.scrollTop; })()`);
  await new Promise(r => setTimeout(r, 500));

  // 检查是否有卡片内容与 topbar 区域重叠
  const overlap = await ev(`(() => {
    const topbar = document.querySelector('.topbar');
    const tb = topbar.getBoundingClientRect();
    const cards = Array.from(document.querySelectorAll('.nav-card, .card-grid .nav-card, .stat-tile, .section-heading'));
    const hits = cards.filter(c => {
      const r = c.getBoundingClientRect();
      return r.top < tb.bottom && r.bottom > tb.top;
    }).map(c => ({ cls: c.className, top: c.getBoundingClientRect().top, bottom: c.getBoundingClientRect().bottom }));
    return { topbarBottom: tb.bottom, overlapping: hits.slice(0, 5) };
  })()`);
  console.log('Overlap check:', JSON.stringify(overlap, null, 2));

  process.exit(0);
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });
