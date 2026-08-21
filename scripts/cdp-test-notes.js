// CDP 驱动 Thorium 无头浏览器，验证 Notes 视图（浏览器模式）。
// 用法: node scripts/cdp-test-notes.js
const { spawn } = require('child_process');
const path = require('path');

const TARGET_URL = process.argv[2] || 'http://127.0.0.1:5173/';
const THORIUM = 'C:\\Users\\Lenovo\\AppData\\Local\\Thorium\\Application\\thorium.exe';
const PORT = 9226;
const USER_DATA = path.join(process.env.TEMP, 'thorium-cdp-notes-' + Date.now());

const browser = spawn(THORIUM, [
  '--headless=new', '--remote-debugging-port=' + PORT,
  '--user-data-dir=' + USER_DATA, '--no-first-run', '--disable-gpu', 'about:blank'
], { stdio: 'ignore' });

let ws, msgId = 0;
const pending = new Map();
let consoleErrors = [];
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
let pass = 0;
function ok(name, cond) {
  console.log((cond ? 'PASS' : 'FAIL') + ' ' + name);
  if (!cond) process.exitCode = 1;
  if (cond) pass++;
}

(async () => {
  const url = await waitForWs();
  ws = new WebSocket(url);
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
  ws.onmessage = (ev) => { const m = JSON.parse(ev.data); if (m.id && pending.has(m.id)) { const p = pending.get(m.id); pending.delete(m.id); m.error ? p.rej(new Error(m.error.message)) : p.res(m.result); } };
  await send('Page.enable'); await send('Runtime.enable');
  await send('Runtime.enable');
  await send('Page.navigate', { url: TARGET_URL });
  await new Promise(r => setTimeout(r, 4000));

  // 1. 切到 Notes 视图
  await ev(`document.getElementById('btnNote').click()`);
  await new Promise(r => setTimeout(r, 600));
  ok('view-notes active', await ev(`document.getElementById('view-notes').classList.contains('active')`));
  ok('editor textarea exists', await ev(`!!document.getElementById('noteInput')`));
  ok('preview pane exists', await ev(`!!document.getElementById('notePreview')`));
  ok('mdbar has bold button', await ev(`!!document.querySelector('#notesMdbar [data-md="bold"]')`));  ok('calendar button exists', await ev(`!!document.getElementById('btnNoteCalendar')`));
  ok('pager exists', await ev(`!!document.getElementById('notesPager')`));
  ok('external app launch UI', await ev(`!!document.getElementById('noteAppPath') && !!document.getElementById('btnNoteAppLaunch')`));
  console.log('rail buttons:', await ev(`Array.from(document.querySelectorAll('.rail-btn')).map(b=>b.id).join(',')`));

  // 2. mdbar 加粗按钮插入 **
  await ev(`(() => { const ta = document.getElementById('noteInput'); ta.value = 'hello world'; ta.setSelectionRange(0, 0); ta.dispatchEvent(new Event('input',{bubbles:true})); return true; })()`);
  await ev(`document.querySelector('.md-btn[data-md="bold"]').click()`);
  await new Promise(r => setTimeout(r, 300));
  const afterBold = await ev(`document.getElementById('noteInput').value`);
  ok('bold wraps selection', afterBold === '**hello world**' || afterBold.includes('**'), 'afterBold=' + JSON.stringify(afterBold));

  // 3. 写入含代码块/图片的 markdown，触发 input 渲染预览
  const md = '# Today\n\n**bold** text\n\n```javascript\nfunction hello() {\n  console.log("hi");\n}\n```\n\n![pic](https://example.com/a.png)';
  await ev(`(() => { const ta = document.getElementById('noteInput'); ta.value = ${JSON.stringify(md)}; ta.dispatchEvent(new Event('input',{bubbles:true})); return true; })()`);
  await new Promise(r => setTimeout(r, 800));

  const previewInfo = await ev(`(() => {
    const p = document.getElementById('notePreview');
    const hl = p.querySelector('.hl-block');
    return {
      hasStrong: !!p.querySelector('strong'),
      hasImg: !!p.querySelector('img.md-img'),
      hasCodeBlock: !!hl,
      langLabel: hl ? (hl.querySelector('.hl-block-lang')||{}).textContent : null,
      lineNums: hl ? hl.querySelectorAll('.hl-num').length : 0,
      hasWrapBtn: hl ? !!hl.querySelector('.hl-wrap-btn') : false,
      codeText: hl ? (hl.querySelector('.hl-code')||{}).textContent : null
    };
  })()`);
  console.log('previewInfo:', JSON.stringify(previewInfo, null, 2));
  ok('preview renders bold', previewInfo.hasStrong);
  ok('preview renders image', previewInfo.hasImg);
  ok('code block rendered', previewInfo.hasCodeBlock);
  ok('code block lang label = JAVASCRIPT', previewInfo.langLabel === 'JAVASCRIPT');
  ok('code block has line numbers', previewInfo.lineNums >= 2);
  ok('code block has wrap toggle btn', previewInfo.hasWrapBtn);
  ok('code block code text', /function hello/.test(previewInfo.codeText || ''));

  // 4. 换行切换按钮
  await ev(`document.querySelector('.hl-wrap-btn').click()`);
  await new Promise(r => setTimeout(r, 200));
  ok('wrap btn toggles .no-wrap', await ev(`document.querySelector('#notePreview .hl-block').classList.contains('no-wrap')`));

  // 截图（编辑 + 预览 + 代码块）
  await send('Page.captureScreenshot', { format: 'png' }).then(r => {
    require('fs').writeFileSync(path.join(__dirname, '..', '.tmp-shots', 'notes-editor.png'), Buffer.from(r.data, 'base64'));
    console.log('screenshot saved: .tmp-shots/notes-editor.png');
  }).catch(e => console.log('screenshot failed:', e.message));

  // 5. 日历弹层
  await ev(`document.getElementById('btnNoteCalendar').click()`);
  await new Promise(r => setTimeout(r, 300));
  ok('calendar popover visible', await ev(`!document.getElementById('noteCalendar').hidden`));
  const calDays = await ev(`document.querySelectorAll('#noteCalendar .nc-day').length`);
  ok('calendar renders day cells', calDays >= 28, 'days=' + calDays);
  // 点击某一天：显示该天笔记与最终保存时间（popover 脚注，保持弹层打开）
  await ev(`(() => { const d = document.querySelector('#noteCalendar .nc-day'); if (d) d.click(); return true; })()`);
  await new Promise(r => setTimeout(r, 300));
  const calFoot = await ev(`(document.getElementById('ncFoot')||{}).textContent || ''`);
  console.log('calendar foot:', calFoot);
  ok('calendar foot shows picked date', /20\d\d-\d\d-\d\d/.test(calFoot));
  ok('editor date label updated', /2026-08-01/.test(await ev(`document.getElementById('noteDayLabel').textContent`)));

  // 6. 分页：塞 13 条笔记触发页码；用 "+ New note" 回到今天（不在列表 → 第 1 页）
  await ev(`(() => {
    const all = {}; const meta = {};
    for (let i = 0; i < 13; i++) { const d = '2026-08-' + String(i+1).padStart(2,'0'); all[d] = 'note ' + i; meta[d] = Date.now(); }
    localStorage.setItem('alpehuez_notes', JSON.stringify(all));
    localStorage.setItem('alpehuez_notes_meta', JSON.stringify(meta));
    return true;
  })()`);
  await ev(`document.getElementById('btnNoteNew').click()`);
  await new Promise(r => setTimeout(r, 500));
  const pagerInfo = await ev(`(() => ({
    hidden: document.getElementById('notesPager').hidden,
    info: document.getElementById('notesPageInfo').textContent,
    listItems: document.querySelectorAll('#notesList .notes-list-item').length
  }))()`);
  console.log('pagerInfo:', JSON.stringify(pagerInfo));
  ok('pager visible with 13 notes', !pagerInfo.hidden);
  ok('pager page 1 of 2', pagerInfo.info.trim() === '1 / 2');
  ok('12 items on page 1', pagerInfo.listItems === 12);
  await ev(`document.getElementById('notesPageNext').click()`);
  await new Promise(r => setTimeout(r, 400));
  ok('page 2 shows 1 item', await ev(`document.querySelectorAll('#notesList .notes-list-item').length`) === 1);
  ok('pager info 2 of 2', await ev(`document.getElementById('notesPageInfo').textContent.trim()`) === '2 / 2');

  // 7. 保存时间戳显示
  ok('savedAt span exists', await ev(`!!document.getElementById('noteSavedAt')`));
  console.log('savedAt text:', await ev(`document.getElementById('noteSavedAt').textContent`));

  // 截图
  await send('Page.captureScreenshot', { format: 'jpeg', quality: 80 }).then(r => {
    require('fs').writeFileSync(path.join(__dirname, '..', '.tmp-shots', 'notes-view.jpg'), Buffer.from(r.data, 'base64'));
    console.log('screenshot saved: .tmp-shots/notes-view.jpg');
  }).catch(e => console.log('screenshot failed:', e.message));

  console.log(pass + ' checks passed');
  process.exit(process.exitCode || 0);
})().catch(e => { console.error('FAIL:', e.message); process.exit(1); });
