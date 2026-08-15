/* my-nav 开发者面板 */
const $ = (sel) => document.querySelector(sel);

// Tauri 注入脚本定义 window.isTauri（不可配置），顶层不能声明同名 const，否则整脚本 SyntaxError 白屏
const isDesktop = typeof window !== 'undefined' && !!window.__TAURI__;

/* 全局错误捕获：任何 JS 错误显示在页面顶部，避免无声白屏 */
window.addEventListener('error', (e) => {
  try {
    const msg = 'JS 错误: ' + (e.message || 'unknown') + ' @ ' + (e.filename || '').split('/').pop() + ':' + (e.lineno || '?');
    const existing = document.getElementById('js-error');
    if (existing) { existing.textContent = msg; existing.hidden = false; return; }
    const div = document.createElement('div');
    div.id = 'js-error';
    div.style.cssText = 'position:fixed;top:0;left:0;right:0;z-index:99999;background:#ef4444;color:#fff;font:12px/1.5 sans-serif;padding:6px 16px;white-space:pre-wrap;';
    div.textContent = msg;
    document.body.appendChild(div);
  } catch (_) { /* 忽略 */ }
});

const ICON_EDIT = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>';
const ICON_DEL = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>';
const ICON_GRIP = '<svg class="drag-handle" viewBox="0 0 24 24" fill="currentColor"><circle cx="9" cy="5" r="1.6"/><circle cx="15" cy="5" r="1.6"/><circle cx="9" cy="12" r="1.6"/><circle cx="15" cy="12" r="1.6"/><circle cx="9" cy="19" r="1.6"/><circle cx="15" cy="19" r="1.6"/></svg>';

/* ---------- 语言包 ---------- */
const LOCALES = {
  zh: {
    appName: 'AlpeHuez 开发者面板', appSub: 'AlpeHuez · 本地内容管理', connected: '已连接', uncommitted: '待推送',
    preview: '预览网站 ↗', navCards: '导航卡片', myFiles: 'My Files', deploy: '部署', appearance: '外观',
    navDesc: '管理首页的网址卡片与分组，保存后自动写入 links.json',
    recompute: '重新计算标签 / VPN', downloadIcons: '下载图标', save: '保存',
    currentGroup: '当前分组', newGroup: '新建分组', rename: '重命名', delete: '删除',
    noCards: '该分组还没有卡片', noMatch: '没有匹配的卡片', newCard: '新建卡片',
    myfilesDesc: '管理文件浏览器中的文件夹与文件，保存后自动写入 data.json',
    folders: '文件夹', new: '新建', files: '文件', newFile: '新建文件', noFiles: '该文件夹还没有文件',
    deployDesc: '提交并推送到 GitHub，Cloudflare Pages 会自动发布',
    repoStatus: '仓库状态', branch: '分支', lastCommit: '最近提交',
    devTimeline: '开发时间轴', timelineEmpty: '暂无提交记录', timelineError: '无法读取提交历史：',
    sysTitle: '系统资源', cpu: 'CPU', mem: '内存', disk: '磁盘',
    appearanceDesc: '自定义面板背景与主题外观',
    bgPreset: '背景预设', presetDefault: '默认', presetMountain: '高山', presetGrid: '网格', presetCustom: '自定义',
    bgUpload: '上传背景图', upload: '上传', bgUploadHint: '上传后自动切换到「自定义」预设，图片保存在本地（%APPDATA%），不会进入 git 仓库。',
    bgAdjust: '调整', blur: '模糊', overlay: '暗色遮罩',
    sidebarBg: '侧边栏独立背景', sidebarBgHint: '为左侧导航栏单独设置竖版背景图（骑行、高山剪影等），自动加深色遮罩保证图标清晰。',
    clear: '清除', bgSaved: '背景已保存',
    defaultBrowser: '默认浏览器', browserHint: '选择打开网址时使用的浏览器，留空则使用系统默认。', systemDefault: '系统默认', refresh: '刷新', browserSaved: '浏览器设置已保存',
    drawerTitle: '日历 / 待办', todayTodo: '今日待办', todoPlaceholder: '添加待办…', noTodo: '暂无待办',
    calWeekdays: ['日', '一', '二', '三', '四', '五', '六'],
    commitPush: '提交并推送', commitMsg: '提交信息', commitPlaceholder: '例如：新增 3 个导航卡片',
    savePush: '保存并推送', commitHint: '将当前所有改动（links.json、data.json、图标、新文件夹）一起提交并推送到 GitHub。',
    log: '运行日志', edit: '编辑', cancel: '取消', ok: '确定', close: '关闭',
    searchPlaceholder: '搜索卡片…', clear: '清除', loading: '加载中…',
    saving: '保存中…', running: '运行中…', pushing: '推送中…',
    savedLinks: 'links.json 已保存', savedMyfiles: 'data.json 已保存',
    iconsDone: '图标已下载并更新', enhanceDone: '标签 / VPN 已重新计算',
    scriptError: '脚本运行出错，请查看日志', fillRequired: '请填写必填项',
    modified: '已修改，记得点「保存」', groupCreated: '已新建分组，记得点「保存」',
    groupRenamed: '已重命名，记得点「保存」', groupDeleted: '已删除分组，记得点「保存」',
    cardDeleted: '已删除，记得点「保存」', folderRemoved: '已移除，记得点「保存」',
    folderCreated: '文件夹已创建（含页面样板文件）', loadFailed: '加载数据失败：',
    editCard: '编辑卡片', renameGroup: '重命名分组', newFolder: '新建文件夹', editFile: '编辑文件',
    title: '标题', url: 'URL', iconUrl: '图标 URL',
    iconHint: '留空则显示字母图标；保存后点「下载图标」可自动抓取 favicon',
    description: '描述', tags: '标签（逗号分隔）', vpnRequired: '需要 VPN 才能访问',
    groupName: '分组名称', slug: 'slug（URL 标识，小写字母/数字/连字符）',
    displayName: '显示名称', passwordProtected: '需要密码保护',
    fileName: '文件名', kind: '类型', report: '报告 report', source: '源码 source',
    size: '大小', fileUrl: 'URL（相对路径，留空用文件名）',
    qmd: '源码 QMD 文件名（可选）', figures: '图表 ZIP 文件名（可选）',
    confirmDeleteGroup: (n) => `确定删除分组「${n}」及其全部卡片？`,
    confirmDeleteCard: (n) => `确定删除卡片「${n}」？`,
    confirmRemoveFolder: (n) => `确定从 My Files 中移除「${n}」？\n（仅从 data.json 移除，磁盘上的文件保留）`,
    confirmDeleteFile: (n) => `确定删除文件「${n}」？`,
    enterCommitMsg: '请填写提交信息', pushed: '已推送到 GitHub', pushIncomplete: '推送未完全成功，请查看日志',
    clean: '工作区干净，无未提交改动', gitError: '无法读取 git 状态：',
    report: '报告', source: '源码', protected: '保护', direct: '直连', vpn: 'VPN',
    loginHint: '请输入密码进入开发者面板', passwordPlaceholder: '密码', wrongPassword: '密码错误', enter: '进入',
    changePwd: '修改密码', changePwdTitle: '修改密码', oldPassword: '旧密码', newPassword: '新密码', confirmPassword: '确认新密码',
    pwdChanged: '密码已修改', pwdMismatch: '两次输入的新密码不一致', pwdTooShort: '新密码至少 4 位', oldPwdWrong: '旧密码错误',
  },
  en: {
    appName: 'AlpeHuez Dev Panel', appSub: 'AlpeHuez · Local Content Manager', connected: 'Connected', uncommitted: 'Uncommitted',
    preview: 'Preview Site ↗', navCards: 'Nav Cards', myFiles: 'My Files', deploy: 'Deploy', appearance: 'Appearance',
    navDesc: 'Manage nav cards & groups. Saved to links.json',
    recompute: 'Recompute Tags / VPN', downloadIcons: 'Download Icons', save: 'Save',
    currentGroup: 'Current Group', newGroup: 'New Group', rename: 'Rename', delete: 'Delete',
    noCards: 'No cards in this group yet', noMatch: 'No matching cards', newCard: 'New Card',
    myfilesDesc: 'Manage folders & files in the file explorer. Saved to data.json',
    folders: 'Folders', new: 'New', files: 'Files', newFile: 'New File', noFiles: 'No files in this folder yet',
    deployDesc: 'Commit & push to GitHub. Cloudflare Pages auto-deploys',
    repoStatus: 'Repo Status', branch: 'Branch', lastCommit: 'Last Commit',
    devTimeline: 'Dev Timeline', timelineEmpty: 'No commits yet', timelineError: 'Failed to read commit history: ',
    sysTitle: 'System Resources', cpu: 'CPU', mem: 'Memory', disk: 'Disk',
    appearanceDesc: 'Customize panel background & theme',
    bgPreset: 'Background Preset', presetDefault: 'Default', presetMountain: 'Mountain', presetGrid: 'Grid', presetCustom: 'Custom',
    bgUpload: 'Upload Background', upload: 'Upload', bgUploadHint: 'Switches to "Custom" preset automatically. Image is stored locally (%APPDATA%), never in the git repo.',
    bgAdjust: 'Adjust', blur: 'Blur', overlay: 'Dark Overlay',
    sidebarBg: 'Sidebar Background', sidebarBgHint: 'Set a separate portrait background for the left nav (cycling, mountain silhouettes…). A dark overlay keeps icons legible.',
    clear: 'Clear', bgSaved: 'Background saved',
    defaultBrowser: 'Default Browser', browserHint: 'Choose which browser opens links. Leave empty to use the system default.', systemDefault: 'System Default', refresh: 'Refresh', browserSaved: 'Browser setting saved',
    drawerTitle: 'Calendar / Todos', todayTodo: 'Today Todos', todoPlaceholder: 'Add todo…', noTodo: 'No todos',
    calWeekdays: ['Su', 'Mo', 'Tu', 'We', 'Th', 'Fr', 'Sa'],
    commitPush: 'Commit & Push', commitMsg: 'Commit Message', commitPlaceholder: 'e.g. Add 3 nav cards',
    savePush: 'Save & Push', commitHint: 'Commits all current changes (links.json, data.json, icons, new folders) and pushes to GitHub.',
    log: 'Log', edit: 'Edit', cancel: 'Cancel', ok: 'OK', close: 'Close',
    searchPlaceholder: 'Search cards…', clear: 'Clear', loading: 'Loading…',
    saving: 'Saving…', running: 'Running…', pushing: 'Pushing…',
    savedLinks: 'links.json saved', savedMyfiles: 'data.json saved',
    iconsDone: 'Icons downloaded & updated', enhanceDone: 'Tags / VPN recomputed',
    scriptError: 'Script error — check the log', fillRequired: 'Please fill in required fields',
    modified: 'Modified — remember to Save', groupCreated: 'Group created — remember to Save',
    groupRenamed: 'Renamed — remember to Save', groupDeleted: 'Group deleted — remember to Save',
    cardDeleted: 'Deleted — remember to Save', folderRemoved: 'Removed — remember to Save',
    folderCreated: 'Folder created (with page templates)', loadFailed: 'Failed to load data: ',
    editCard: 'Edit Card', renameGroup: 'Rename Group', newFolder: 'New Folder', editFile: 'Edit File',
    title: 'Title', url: 'URL', iconUrl: 'Icon URL',
    iconHint: 'Leave empty for a letter avatar; click "Download Icons" after saving to auto-fetch the favicon',
    description: 'Description', tags: 'Tags (comma-separated)', vpnRequired: 'Requires VPN',
    groupName: 'Group Name', slug: 'slug (URL id, lowercase letters/numbers/hyphens)',
    displayName: 'Display Name', passwordProtected: 'Password protected',
    fileName: 'File Name', kind: 'Type', report: 'Report', source: 'Source',
    size: 'Size', fileUrl: 'URL (relative, defaults to filename)',
    qmd: 'Source QMD filename (optional)', figures: 'Figures ZIP filename (optional)',
    confirmDeleteGroup: (n) => `Delete group "${n}" and all its cards?`,
    confirmDeleteCard: (n) => `Delete card "${n}"?`,
    confirmRemoveFolder: (n) => `Remove "${n}" from My Files?\n(Only removed from data.json; files stay on disk)`,
    confirmDeleteFile: (n) => `Delete file "${n}"?`,
    enterCommitMsg: 'Please enter a commit message', pushed: 'Pushed to GitHub', pushIncomplete: 'Push incomplete — check the log',
    clean: 'Working tree clean', gitError: 'Failed to read git status: ',
    report: 'Report', source: 'Source', protected: 'Protected', direct: 'Direct', vpn: 'VPN',
    loginHint: 'Enter password to open the developer panel', passwordPlaceholder: 'Password', wrongPassword: 'Wrong password', enter: 'Enter',
    changePwd: 'Change Password', changePwdTitle: 'Change Password', oldPassword: 'Old Password', newPassword: 'New Password', confirmPassword: 'Confirm New Password',
    pwdChanged: 'Password updated', pwdMismatch: 'New passwords do not match', pwdTooShort: 'New password must be at least 4 characters', oldPwdWrong: 'Old password is wrong',
  },
};

let lang = 'zh';
try { lang = localStorage.getItem('panel-lang') || 'zh'; } catch (e) {}
let theme = 'dark';
try { theme = localStorage.getItem('panel-theme') || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'); } catch (e) {}
let statusDirty = false;

function t(key, ...args) {
  const val = LOCALES[lang][key];
  return typeof val === 'function' ? val(...args) : (val ?? key);
}

function applyLang() {
  document.documentElement.lang = lang === 'zh' ? 'zh-CN' : 'en';
  document.querySelectorAll('[data-i18n]').forEach((el) => { el.textContent = t(el.dataset.i18n); });
  document.querySelectorAll('[data-i18n-placeholder]').forEach((el) => { el.placeholder = t(el.dataset.i18nPlaceholder); });
  document.querySelectorAll('[data-i18n-aria]').forEach((el) => { el.setAttribute('aria-label', t(el.dataset.i18nAria)); });
  document.querySelectorAll('[data-i18n-title]').forEach((el) => { el.setAttribute('title', t(el.dataset.i18nTitle)); });
  $('#btnLang').textContent = lang === 'zh' ? 'EN' : '中';
  setStatusLabel();
  renderGroups();
  renderFolders();
  renderCalendar();
  loadGitStatus();
}

function applyTheme() {
  document.documentElement.setAttribute('data-theme', theme);
  $('#iconSun').hidden = theme === 'dark';
  $('#iconMoon').hidden = theme === 'light';
}

function setStatusLabel() {
  const label = $('#serverStatus').querySelector('[data-i18n]');
  if (label) label.textContent = statusDirty ? t('uncommitted') : t('connected');
}

let links = null;
let myfiles = null;
let currentGroup = 0;
let currentFolder = null;
let filterTerm = '';
let dragIndex = null;

/* ---------- 工具函数 ---------- */
function escapeHtml(s) {
  return String(s ?? '').replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
}
function escapeAttr(s) { return escapeHtml(s); }
function today() {
  const d = new Date();
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

async function api(path, opts = {}) {
  if (isDesktop) {
    const { invoke } = window.__TAURI__.core;
    const body = opts.body ? JSON.parse(opts.body) : {};
    const method = opts.method || 'GET';
    if (path === '/api/links') {
      if (method === 'GET') return { ok: true, data: await invoke('read_json', { path: 'links.json' }) };
      const res = await invoke('write_json', { path: 'links.json', data: body.data });
      return { ok: true, md5: res.md5 };
    }
    if (path === '/api/myfiles') {
      if (method === 'GET') return { ok: true, data: await invoke('read_json', { path: 'myfiles/data.json' }) };
      await invoke('write_json', { path: 'myfiles/data.json', data: body.data });
      return { ok: true };
    }
    if (path === '/api/run-script') return await invoke('run_script', { script: body.script });
    if (path === '/api/create-folder') {
      await invoke('create_folder', { slug: body.slug, name: body.name, protected: body.protected });
      return { ok: true };
    }
    if (path === '/api/git-status') return await invoke('git_status');
    if (path === '/api/git-log') return await invoke('git_log');
    if (path === '/api/sys-stats') return await invoke('sys_stats');
    if (path === '/api/bg-config') {
      if (method === 'GET') return await invoke('get_bg_config');
      await invoke('set_bg_config', { config: body.config });
      return { ok: true };
    }
    if (path === '/api/save-bg-image') return await invoke('save_bg_image', { data: body.data, ext: body.ext });
    if (path === '/api/list-browsers') return await invoke('list_browsers');
    if (path === '/api/browser-config') {
      if (method === 'GET') return await invoke('get_browser_config');
      await invoke('set_browser_config', { config: body.config });
      return { ok: true };
    }
    if (path === '/api/git-push') return await invoke('git_push', { message: body.message });
    throw new Error('未知接口: ' + path);
  }
  if (path === '/api/git-log' || path === '/api/sys-stats' || path === '/api/bg-config' || path === '/api/save-bg-image' || path === '/api/list-browsers' || path === '/api/browser-config') return null; // 浏览器模式无此接口
  const res = await fetch(path, { headers: { 'Content-Type': 'application/json' }, ...opts });
  const data = await res.json().catch(() => ({ ok: false, error: t('loadFailed') }));
  if (!res.ok || data.ok === false) throw new Error(data.error || `HTTP ${res.status}`);
  return data;
}

let toastTimer;
function toast(msg, type = '') {
  const el = $('#toast');
  el.textContent = msg;
  el.className = 'toast ' + type;
  el.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { el.hidden = true; }, 2600);
}

function appendLog(text) {
  const el = $('#logArea');
  if (el.textContent === '—') el.textContent = '';
  el.textContent += (text || '') + '\n';
  el.scrollTop = el.scrollHeight;
}

/* ---------- 模态框 ---------- */
function fieldHtml(f) {
  if (f.type === 'checkbox') {
    return `<div class="checkbox-row"><label class="toggle"><input type="checkbox" id="f_${f.key}" ${f.value ? 'checked' : ''}><span class="toggle-slider"></span></label><label for="f_${f.key}">${f.label}</label></div>`;
  }
  if (f.type === 'select') {
    const opts = (f.options || []).map((o) => `<option value="${escapeAttr(o.value)}" ${o.value === f.value ? 'selected' : ''}>${escapeHtml(o.label)}</option>`).join('');
    return `<div class="form-row"><label>${f.label}</label><select id="f_${f.key}" class="select">${opts}</select></div>`;
  }
  const tag = f.type === 'textarea' ? 'textarea' : 'input';
  const typeAttr = f.type === 'number' ? 'number' : (f.type === 'password' ? 'password' : 'text');
  const extra = f.type === 'textarea' ? 'rows="3"' : `type="${typeAttr}"`;
  const hint = f.hint ? `<p class="hint">${f.hint}</p>` : '';
  return `<div class="form-row"><label>${f.label}</label><${tag} id="f_${f.key}" class="input" ${extra} value="${escapeAttr(f.value ?? '')}" placeholder="${escapeAttr(f.placeholder || '')}" ${f.required ? 'required' : ''}></${tag}>${hint}</div>`;
}

function buildForm(fields) {
  let html = '';
  let row = [];
  const flush = () => {
    if (row.length === 1) html += row[0];
    else if (row.length === 2) html += `<div class="form-grid-2">${row[0]}${row[1]}</div>`;
    row = [];
  };
  for (const f of fields) {
    const el = fieldHtml(f);
    if (f.half) { row.push(el); if (row.length === 2) flush(); }
    else { flush(); html += el; }
  }
  flush();
  return html;
}

function openModal(title, fields, onOk) {
  $('#modalBody').innerHTML = buildForm(fields);
  $('#modalTitle').textContent = title;
  $('#modalBackdrop').hidden = false;
  const okBtn = $('#modalOk');
  const cleanup = () => {
    $('#modalBackdrop').hidden = true;
    okBtn.onclick = null;
    $('#modalCancel').onclick = null;
    $('#modalClose').onclick = null;
    $('#modalBackdrop').onclick = null;
  };
  okBtn.onclick = () => {
    const values = {};
    for (const f of fields) {
      const el = document.getElementById('f_' + f.key);
      if (!el) continue;
      values[f.key] = f.type === 'checkbox' ? el.checked : el.value.trim();
    }
    if (fields.some((f) => f.required && !values[f.key])) {
      toast(t('fillRequired'), 'error');
      return;
    }
    cleanup();
    onOk(values);
  };
  $('#modalCancel').onclick = cleanup;
  $('#modalClose').onclick = cleanup;
  $('#modalBackdrop').onclick = (e) => { if (e.target === $('#modalBackdrop')) cleanup(); };
}

/* ---------- 导航卡片 ---------- */
function renderGroups() {
  const sel = $('#groupSelect');
  sel.innerHTML = ((links && links.icons) || []).map((g, i) =>
    `<option value="${i}">${escapeHtml(g.title)} (${(g.children || []).length})</option>`).join('');
  sel.value = currentGroup;
  renderCards();
}

function iconFallback(el) {
  const title = el.dataset.title || '';
  el.outerHTML = `<span class="letter">${escapeHtml((title || '?')[0].toUpperCase())}</span>`;
}

function renderCards() {
  const group = links && links.icons[currentGroup];
  const list = $('#cardList');
  const empty = $('#cardEmpty');
  const term = filterTerm.trim().toLowerCase();
  const all = (group && group.children) || [];
  const filtered = term
    ? all.map((item, i) => ({ item, i })).filter(({ item }) =>
        (item.title || '').toLowerCase().includes(term)
        || (item.url || '').toLowerCase().includes(term)
        || (item.tags || []).some((tag) => tag.toLowerCase().includes(term)))
    : all.map((item, i) => ({ item, i }));
  if (!filtered.length) {
    list.innerHTML = '';
    empty.hidden = false;
    empty.querySelector('p').textContent = term ? t('noMatch') : t('noCards');
    return;
  }
  empty.hidden = true;
  list.innerHTML = filtered.map(({ item, i }) => {
    const icon = item.icon && item.icon.src
      ? `<img src="${escapeAttr(item.icon.src)}" data-title="${escapeAttr(item.title)}" alt="" onerror="iconFallback(this)">`
      : `<span class="letter">${escapeHtml((item.title || '?')[0].toUpperCase())}</span>`;
    const vpn = item.isVpnRequired
      ? `<span class="badge badge-vpn">${t('vpn')}</span>`
      : `<span class="badge badge-direct">${t('direct')}</span>`;
    const tags = (item.tags || []).map((tag) => `<span class="tag">${escapeHtml(tag)}</span>`).join('');
    return `<div class="card-item" draggable="true" data-index="${i}">
      ${ICON_GRIP}
      <div class="card-icon">${icon}</div>
      <div class="card-info">
        <div class="card-title-row"><span class="name">${escapeHtml(item.title)}</span>${vpn}</div>
        <div class="card-url">${escapeHtml(item.url)}</div>
        ${tags ? `<div class="card-tags">${tags}</div>` : ''}
      </div>
      <div class="card-actions">
        <button class="icon-btn" data-edit="${i}" title="${t('edit')}">${ICON_EDIT}</button>
        <button class="icon-btn danger" data-del="${i}" title="${t('delete')}">${ICON_DEL}</button>
      </div>
    </div>`;
  }).join('');
}

function openCardModal(item, index) {
  openModal(index === null ? t('newCard') : t('editCard'), [
    { key: 'title', label: t('title'), value: item.title, required: true },
    { key: 'url', label: t('url'), value: item.url, required: true, placeholder: 'https://' },
    { key: 'icon', label: t('iconUrl'), value: (item.icon && item.icon.src) || '', placeholder: 'https://example.com/favicon.ico', hint: t('iconHint') },
    { key: 'description', label: t('description'), value: item.description || '' },
    { key: 'tags', label: t('tags'), value: (item.tags || []).join(', ') },
    { key: 'vpn', label: t('vpnRequired'), type: 'checkbox', value: !!item.isVpnRequired },
  ], (values) => {
    const card = {
      icon: { text: '', itemType: 2, src: values.icon, backgroundColor: '' },
      sort: item.sort ?? 99999,
      title: values.title,
      url: values.url,
      openMethod: item.openMethod ?? 1,
      lanUrl: item.lanUrl ?? '',
      description: values.description,
      tags: values.tags ? values.tags.split(/[,，]/).map((tag) => tag.trim()).filter(Boolean) : [],
      isVpnRequired: values.vpn,
      clickCount: item.clickCount ?? 0,
    };
    const group = links.icons[currentGroup];
    if (index === null) group.children.push(card);
    else group.children[index] = card;
    renderCards();
    renderGroups();
    toast(t('modified'));
  });
}

async function runScript(script, btn, successMsg) {
  btn.disabled = true;
  const original = btn.textContent;
  btn.textContent = t('running');
  try {
    const res = await api('/api/run-script', { method: 'POST', body: JSON.stringify({ script }) });
    appendLog(res.output);
    if (res.ok) {
      toast(successMsg, 'success');
      if (script === 'download_icons' || script === 'enhance_links') await loadLinks();
    } else {
      toast(t('scriptError'), 'error');
    }
  } catch (e) {
    toast(e.message, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = original;
  }
}

/* ---------- My Files ---------- */
function renderFolders() {
  const list = $('#folderList');
  list.innerHTML = ((myfiles && myfiles.folders) || []).map((f, i) => {
    const count = (f.files || []).length + (f.folders || []).length;
    return `<div class="folder-item ${f.slug === currentFolder ? 'active' : ''}" data-folder="${i}">
      <span class="fname">${escapeHtml(f.name)}</span>
      ${f.protected ? `<span class="badge badge-vpn">${t('protected')}</span>` : ''}
      <span class="fcount">${count}</span>
      <button class="icon-btn danger fdel" data-fdel="${i}" title="${t('delete')}">${ICON_DEL}</button>
    </div>`;
  }).join('');
  renderFiles();
}

function renderFiles() {
  const folder = ((myfiles && myfiles.folders) || []).find((f) => f.slug === currentFolder);
  const list = $('#fileList');
  const empty = $('#fileEmpty');
  $('#fileColTitle').textContent = folder ? `${t('files')} · ${folder.name}` : t('files');
  if (!folder || !(folder.files || []).length) {
    list.innerHTML = '';
    empty.hidden = false;
    return;
  }
  empty.hidden = true;
  list.innerHTML = folder.files.map((f, i) => {
    const kindLabel = f.kind === 'source' ? t('source') : t('report');
    return `<div class="file-item">
      <span class="fname">${escapeHtml(f.name)}</span>
      <span class="fmeta">${kindLabel}${f.size ? ' · ' + escapeHtml(f.size) : ''}</span>
      <div class="factions">
        <button class="icon-btn" data-fedit="${i}" title="${t('edit')}">${ICON_EDIT}</button>
        <button class="icon-btn danger" data-fdel="${i}" title="${t('delete')}">${ICON_DEL}</button>
      </div>
    </div>`;
  }).join('');
}

function openFileModal(file, index) {
  openModal(index === null ? t('newFile') : t('editFile'), [
    { key: 'name', label: t('fileName'), value: file.name, required: true },
    { key: 'kind', label: t('kind'), type: 'select', options: [{ value: 'report', label: t('report') }, { value: 'source', label: t('source') }], value: file.kind || 'report' },
    { key: 'size', label: t('size'), value: file.size || '', placeholder: '25 KB' },
    { key: 'url', label: t('fileUrl'), value: file.url || '' },
    { key: 'qmd', label: t('qmd'), value: (file.source && file.source.qmd) || '' },
    { key: 'figures', label: t('figures'), value: (file.source && file.source.figures) || '' },
  ], (values) => {
    const f = { name: values.name, kind: values.kind };
    if (values.size) f.size = values.size;
    if (values.url) f.url = values.url;
    if (values.qmd || values.figures) {
      f.source = {};
      if (values.qmd) f.source.qmd = values.qmd;
      if (values.figures) f.source.figures = values.figures;
    }
    const folder = (myfiles.folders || []).find((fd) => fd.slug === currentFolder);
    if (!folder) return;
    if (index === null) folder.files.push(f);
    else folder.files[index] = f;
    renderFiles();
    renderFolders();
    toast(t('modified'));
  });
}

/* ---------- 部署 ---------- */
function setBar(sel, pct) {
  const bar = $(sel);
  bar.style.width = pct + '%';
  bar.classList.toggle('warn', pct >= 60);
  bar.classList.toggle('danger', pct >= 85);
}

async function loadSysStats() {
  try {
    const s = await api('/api/sys-stats');
    if (!s) return;
    const cpu = Math.round(s.cpu);
    const memPct = s.memTotal ? Math.round((s.memUsed / s.memTotal) * 100) : 0;
    const diskPct = s.diskTotal ? Math.round((s.diskUsed / s.diskTotal) * 100) : 0;
    $('#cpuVal').textContent = cpu + '%';
    $('#memVal').textContent = memPct + '%';
    $('#diskVal').textContent = diskPct + '%';
    setBar('#cpuBar', cpu);
    setBar('#memBar', memPct);
    setBar('#diskBar', diskPct);
  } catch (e) { /* 静默：桌面模式才可用 */ }
}

async function loadGitTimeline() {
  const el = $('#gitTimeline');
  try {
    const res = await api('/api/git-log');
    if (!res || !res.length) {
      el.innerHTML = `<div class="muted">${t('timelineEmpty')}</div>`;
      return;
    }
    el.innerHTML = res.map((c, i) => `
      <div class="tl-item ${i === 0 ? 'latest' : ''}">
        <div class="tl-node"></div>
        <div class="tl-body">
          <div class="tl-msg">${escapeHtml(c.message)}</div>
          <div class="tl-meta"><code>${escapeHtml(c.short)}</code> · ${escapeHtml(c.date)} · ${escapeHtml(c.author)}</div>
        </div>
      </div>`).join('');
  } catch (e) {
    el.innerHTML = `<div class="muted">${t('timelineError')}${escapeHtml(e.message)}</div>`;
  }
}

async function loadGitStatus() {
  try {
    const res = await api('/api/git-status');
    $('#gitBranch').textContent = res.branch || '—';
    $('#gitLast').textContent = res.last || '—';
    const changes = res.status ? res.status.split('\n').filter(Boolean) : [];
    statusDirty = changes.length > 0;
    setStatusLabel();
    $('#serverStatus').classList.toggle('dirty', statusDirty);
    $('#deployBadge').classList.toggle('show', statusDirty);
    const el = $('#gitChanges');
    if (!changes.length) {
      el.innerHTML = `<div class="muted">${t('clean')}</div>`;
    } else {
      el.innerHTML = changes.map((line) => {
        const st = line.slice(0, 2).trim() || '??';
        const file = line.slice(3);
        return `<div class="change-line"><span class="st ${st}">${st}</span><span>${escapeHtml(file)}</span></div>`;
      }).join('');
    }
  } catch (e) {
    $('#gitChanges').innerHTML = `<div class="muted">${t('gitError')}${escapeHtml(e.message)}</div>`;
  }
}

/* ---------- 外观 / 自定义背景 ---------- */
let bgConfig = null;

function convertFileSrc(path) {
  try {
    return window.__TAURI__.core.convertFileSrc(path);
  } catch (e) {
    return path;
  }
}

function applyBg() {
  try {
    const bg = bgConfig || {};
    const layer = $('#bgLayer');
    const preset = bg.preset || 'default';
    const blur = bg.blur ?? 0;
    const overlay = bg.overlay ?? 55;
    if (preset === 'custom' && bg.imagePath) {
      const url = isDesktop ? convertFileSrc(bg.imagePath) : bg.imagePath;
      layer.style.backgroundImage = `url("${url}")`;
    } else {
      layer.style.backgroundImage = `var(--preset-${preset})`;
    }
    layer.style.filter = `blur(${blur}px)`;
    document.documentElement.style.setProperty('--bg-overlay', (overlay / 100).toFixed(2));
    const sidebar = $('.sidebar');
    if (bg.sidebarImage) {
      const url = isDesktop ? convertFileSrc(bg.sidebarImage) : bg.sidebarImage;
      sidebar.style.backgroundImage = `url("${url}")`;
    } else {
      sidebar.style.backgroundImage = 'none';
    }
    document.querySelectorAll('.preset-btn').forEach((b) => b.classList.toggle('active', b.dataset.preset === preset));
    $('#bgBlur').value = blur;
    $('#bgBlurVal').textContent = blur + 'px';
    $('#bgOverlay').value = overlay;
    $('#bgOverlayVal').textContent = overlay + '%';
  } catch (e) { /* 静默：背景样式失败不应阻断面板 */ }
}

async function loadBgConfig() {
  try {
    const res = await api('/api/bg-config');
    bgConfig = res || {};
  } catch (e) {
    bgConfig = {};
  }
  applyBg();
}

async function saveBgConfig() {
  try {
    await api('/api/bg-config', { method: 'POST', body: JSON.stringify({ config: bgConfig }) });
  } catch (e) { /* 浏览器模式忽略 */ }
}

function uploadBgFile(input, key) {
  const file = input.files[0];
  if (!file) return;
  const ext = (file.name.split('.').pop() || 'png').toLowerCase();
  const reader = new FileReader();
  reader.onload = async () => {
    const data = String(reader.result).split(',')[1];
    try {
      const path = await api('/api/save-bg-image', { method: 'POST', body: JSON.stringify({ data, ext }) });
      bgConfig[key] = path;
      if (key === 'imagePath') bgConfig.preset = 'custom';
      applyBg();
      saveBgConfig();
      toast(t('bgSaved'), 'success');
    } catch (e) {
      toast(e.message, 'error');
    }
  };
  reader.readAsDataURL(file);
}

/* ---------- 默认浏览器 ---------- */
let browserConfig = null;

async function loadBrowsers() {
  try {
    const list = await api('/api/list-browsers');
    if (!list) return;
    const sel = $('#browserSelect');
    const current = (browserConfig && browserConfig.path) || '';
    sel.innerHTML = `<option value="">${t('systemDefault')}</option>` + list.map((b) =>
      `<option value="${escapeAttr(b.path)}" ${b.path === current ? 'selected' : ''}>${escapeHtml(b.name)}</option>`).join('');
  } catch (e) { /* 静默 */ }
}

async function loadBrowserConfig() {
  try {
    const res = await api('/api/browser-config');
    browserConfig = res || {};
  } catch (e) {
    browserConfig = {};
  }
  await loadBrowsers();
}

/* ---------- 事件绑定 ---------- */
document.querySelectorAll('.tab-btn').forEach((btn) => {
  btn.onclick = () => {
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.querySelectorAll('.tab-panel').forEach((p) => p.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
    if (btn.dataset.tab === 'deploy') { loadGitStatus(); loadGitTimeline(); loadSysStats(); }
  };
});

$('#btnLang').onclick = () => {
  lang = lang === 'zh' ? 'en' : 'zh';
  localStorage.setItem('panel-lang', lang);
  applyLang();
};

$('#btnTheme').onclick = () => {
  theme = theme === 'dark' ? 'light' : 'dark';
  localStorage.setItem('panel-theme', theme);
  applyTheme();
};

$('#btnPreview').addEventListener('click', (e) => {
  if (isDesktop) {
    e.preventDefault();
    const { WebviewWindow } = window.__TAURI__.webviewWindow;
    new WebviewWindow('preview', {
      url: 'http://nav.localhost/index.html',
      title: 'my-nav 网站预览',
      width: 1280,
      height: 800,
    });
  }
});

/* ---------- 外观事件 ---------- */
$('#presetGrid').addEventListener('click', (e) => {
  const btn = e.target.closest('.preset-btn');
  if (!btn) return;
  bgConfig.preset = btn.dataset.preset;
  applyBg();
  saveBgConfig();
});

$('#btnUploadBg').onclick = () => uploadBgFile($('#bgFile'), 'imagePath');
$('#btnUploadSidebar').onclick = () => uploadBgFile($('#sidebarFile'), 'sidebarImage');
$('#btnClearSidebar').onclick = () => {
  delete bgConfig.sidebarImage;
  applyBg();
  saveBgConfig();
};

$('#bgBlur').addEventListener('input', () => {
  bgConfig.blur = Number($('#bgBlur').value);
  $('#bgBlurVal').textContent = bgConfig.blur + 'px';
  applyBg();
});
$('#bgBlur').addEventListener('change', saveBgConfig);

$('#bgOverlay').addEventListener('input', () => {
  bgConfig.overlay = Number($('#bgOverlay').value);
  $('#bgOverlayVal').textContent = bgConfig.overlay + '%';
  applyBg();
});
$('#bgOverlay').addEventListener('change', saveBgConfig);

/* ---------- 浏览器事件 ---------- */
$('#browserSelect').addEventListener('change', async () => {
  browserConfig.path = $('#browserSelect').value;
  try {
    await api('/api/browser-config', { method: 'POST', body: JSON.stringify({ config: browserConfig }) });
    toast(t('browserSaved'), 'success');
  } catch (e) { /* 浏览器模式忽略 */ }
});
$('#btnRefreshBrowsers').onclick = loadBrowsers;

/* ---------- 右侧抽屉（日历 / 待办） ---------- */
let todos = [];
try { todos = JSON.parse(localStorage.getItem('panel-todos') || '[]'); } catch (e) { todos = []; }

const ICON_RADIO = '<svg class="radio-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4.9 19.1C1 15.2 1 8.8 4.9 4.9"/><path d="M7.8 16.2c-2.3-2.3-2.3-6.1 0-8.5"/><circle cx="12" cy="12" r="2"/><path d="M16.2 7.8c2.3 2.3 2.3 6.1 0 8.5"/><path d="M19.1 4.9C23 8.8 23 15.2 19.1 19.1"/></svg>';

function renderCalendar() {
  const now = new Date();
  const year = now.getFullYear();
  const month = now.getMonth();
  const firstDay = new Date(year, month, 1).getDay();
  const daysInMonth = new Date(year, month + 1, 0).getDate();
  const today = now.getDate();
  const wd = t('calWeekdays');
  let html = `<div class="cal-head">${lang === 'zh' ? year + '年' + (month + 1) + '月' : (month + 1) + '/' + year}</div><div class="cal-grid">`;
  wd.forEach((w) => { html += `<div class="cal-dow">${w}</div>`; });
  for (let i = 0; i < firstDay; i++) html += '<div class="cal-cell empty"></div>';
  for (let d = 1; d <= daysInMonth; d++) {
    html += `<div class="cal-cell ${d === today ? 'today' : ''}">${d}</div>`;
  }
  html += '</div>';
  $('#drawerCal').innerHTML = html;
}

function renderTodos() {
  const list = $('#todoList');
  if (!todos.length) {
    list.innerHTML = `<div class="muted">${t('noTodo')}</div>`;
    return;
  }
  list.innerHTML = todos.map((td, i) => `
    <div class="todo-item ${td.done ? 'done' : ''}" data-todo="${i}">
      ${ICON_RADIO}
      <span class="todo-text">${escapeHtml(td.text)}</span>
      <button class="todo-del" data-todel="${i}">✕</button>
    </div>`).join('');
}

function saveTodos() {
  localStorage.setItem('panel-todos', JSON.stringify(todos));
}

$('#btnAddTodo').onclick = () => {
  const text = $('#todoInput').value.trim();
  if (!text) return;
  todos.push({ text, done: false });
  $('#todoInput').value = '';
  saveTodos();
  renderTodos();
};
$('#todoInput').addEventListener('keydown', (e) => { if (e.key === 'Enter') $('#btnAddTodo').click(); });
$('#todoList').addEventListener('click', (e) => {
  const del = e.target.closest('[data-todel]');
  if (del) {
    todos.splice(Number(del.dataset.todel), 1);
    saveTodos();
    renderTodos();
    return;
  }
  const item = e.target.closest('.todo-item');
  if (item) {
    const i = Number(item.dataset.todo);
    todos[i].done = !todos[i].done;
    saveTodos();
    renderTodos();
  }
});

$('#groupSelect').onchange = () => {
  currentGroup = Number($('#groupSelect').value);
  renderCards();
};

$('#cardSearch').addEventListener('input', () => {
  filterTerm = $('#cardSearch').value;
  $('#btnClearSearch').hidden = !filterTerm;
  renderCards();
});

$('#btnClearSearch').onclick = () => {
  $('#cardSearch').value = '';
  filterTerm = '';
  $('#btnClearSearch').hidden = true;
  renderCards();
};

$('#btnNewGroup').onclick = () => {
  openModal(t('newGroup'), [{ key: 'title', label: t('groupName'), value: '', required: true }], (v) => {
    links.icons.push({ title: v.title, sort: 0, children: [] });
    currentGroup = links.icons.length - 1;
    renderGroups();
    toast(t('groupCreated'));
  });
};

$('#btnRenameGroup').onclick = () => {
  const g = links.icons[currentGroup];
  openModal(t('renameGroup'), [{ key: 'title', label: t('groupName'), value: g.title, required: true }], (v) => {
    g.title = v.title;
    renderGroups();
    toast(t('groupRenamed'));
  });
};

$('#btnDeleteGroup').onclick = () => {
  const g = links.icons[currentGroup];
  if (!confirm(t('confirmDeleteGroup', g.title))) return;
  links.icons.splice(currentGroup, 1);
  currentGroup = Math.max(0, currentGroup - 1);
  renderGroups();
  toast(t('groupDeleted'));
};

$('#btnNewCardEmpty').onclick = () => openCardModal({ title: '', url: '', icon: {}, description: '', tags: [], isVpnRequired: false }, null);

$('#cardList').addEventListener('click', (e) => {
  const editBtn = e.target.closest('[data-edit]');
  const delBtn = e.target.closest('[data-del]');
  if (editBtn) {
    const i = Number(editBtn.dataset.edit);
    openCardModal(links.icons[currentGroup].children[i], i);
  }
  if (delBtn) {
    const i = Number(delBtn.dataset.del);
    const item = links.icons[currentGroup].children[i];
    if (confirm(t('confirmDeleteCard', item.title))) {
      links.icons[currentGroup].children.splice(i, 1);
      renderCards();
      renderGroups();
      toast(t('cardDeleted'));
    }
  }
});

/* 拖拽排序 */
$('#cardList').addEventListener('dragstart', (e) => {
  const item = e.target.closest('.card-item');
  if (!item || filterTerm) { e.preventDefault(); return; }
  dragIndex = Number(item.dataset.index);
  item.classList.add('dragging');
  e.dataTransfer.effectAllowed = 'move';
  e.dataTransfer.setData('text/plain', String(dragIndex));
});

$('#cardList').addEventListener('dragover', (e) => {
  if (dragIndex === null) return;
  e.preventDefault();
  e.dataTransfer.dropEffect = 'move';
  const item = e.target.closest('.card-item');
  document.querySelectorAll('.card-item').forEach((el) => el.classList.remove('drag-over'));
  if (item) item.classList.add('drag-over');
});

$('#cardList').addEventListener('drop', (e) => {
  if (dragIndex === null) return;
  e.preventDefault();
  const item = e.target.closest('.card-item');
  if (item) {
    const overIndex = Number(item.dataset.index);
    const group = links.icons[currentGroup];
    const [moved] = group.children.splice(dragIndex, 1);
    group.children.splice(overIndex, 0, moved);
    renderCards();
    renderGroups();
    toast(t('modified'));
  }
});

$('#cardList').addEventListener('dragend', () => {
  dragIndex = null;
  document.querySelectorAll('.card-item').forEach((el) => el.classList.remove('dragging', 'drag-over'));
});

$('#btnSaveLinks').onclick = async () => {
  const btn = $('#btnSaveLinks');
  btn.disabled = true;
  btn.textContent = t('saving');
  try {
    await api('/api/links', { method: 'POST', body: JSON.stringify({ data: links }) });
    toast(t('savedLinks'), 'success');
  } catch (e) {
    toast(e.message, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = t('save');
  }
};

$('#btnDownloadIcons').onclick = () => runScript('download_icons', $('#btnDownloadIcons'), t('iconsDone'));
$('#btnEnhance').onclick = () => runScript('enhance_links', $('#btnEnhance'), t('enhanceDone'));

$('#folderList').addEventListener('click', (e) => {
  const delBtn = e.target.closest('[data-fdel]');
  if (delBtn) {
    e.stopPropagation();
    const i = Number(delBtn.dataset.fdel);
    const f = myfiles.folders[i];
    if (confirm(t('confirmRemoveFolder', f.name))) {
      myfiles.folders.splice(i, 1);
      if (currentFolder === f.slug) currentFolder = (myfiles.folders[0] && myfiles.folders[0].slug) || null;
      renderFolders();
      toast(t('folderRemoved'));
    }
    return;
  }
  const folderEl = e.target.closest('[data-folder]');
  if (folderEl) {
    currentFolder = myfiles.folders[Number(folderEl.dataset.folder)].slug;
    renderFolders();
  }
});

$('#btnNewFolder').onclick = () => {
  openModal(t('newFolder'), [
    { key: 'slug', label: t('slug'), value: '', required: true, placeholder: 'my-folder' },
    { key: 'name', label: t('displayName'), value: '', required: true },
    { key: 'protected', label: t('passwordProtected'), type: 'checkbox', value: false },
  ], async (v) => {
    try {
      await api('/api/create-folder', { method: 'POST', body: JSON.stringify({ slug: v.slug, name: v.name, protected: v.protected }) });
      myfiles.folders.push({ slug: v.slug, name: v.name, protected: v.protected, dateAdded: today(), folders: [], files: [] });
      currentFolder = v.slug;
      renderFolders();
      toast(t('folderCreated'), 'success');
    } catch (e) {
      toast(e.message, 'error');
    }
  });
};

$('#fileList').addEventListener('click', (e) => {
  const editBtn = e.target.closest('[data-fedit]');
  const delBtn = e.target.closest('[data-fdel]');
  const folder = (myfiles.folders || []).find((f) => f.slug === currentFolder);
  if (!folder) return;
  if (editBtn) {
    const i = Number(editBtn.dataset.fedit);
    openFileModal(folder.files[i], i);
  }
  if (delBtn) {
    const i = Number(delBtn.dataset.fdel);
    const f = folder.files[i];
    if (confirm(t('confirmDeleteFile', f.name))) {
      folder.files.splice(i, 1);
      renderFiles();
      renderFolders();
      toast(t('cardDeleted'));
    }
  }
});

$('#btnNewFile').onclick = () => openFileModal({ name: '', kind: 'report', size: '', url: '', source: {} }, null);
$('#btnNewFileEmpty').onclick = () => openFileModal({ name: '', kind: 'report', size: '', url: '', source: {} }, null);

$('#btnSaveMyfiles').onclick = async () => {
  const btn = $('#btnSaveMyfiles');
  btn.disabled = true;
  btn.textContent = t('saving');
  try {
    await api('/api/myfiles', { method: 'POST', body: JSON.stringify({ data: myfiles }) });
    toast(t('savedMyfiles'), 'success');
  } catch (e) {
    toast(e.message, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = t('save');
  }
};

$('#btnPush').onclick = async () => {
  const btn = $('#btnPush');
  const msg = $('#commitMsg').value.trim();
  if (!msg) { toast(t('enterCommitMsg'), 'error'); return; }
  btn.disabled = true;
  btn.textContent = t('pushing');
  appendLog('$ git add -A && git commit && git push');
  try {
    const res = await api('/api/git-push', { method: 'POST', body: JSON.stringify({ message: msg }) });
    if (res.add) appendLog(res.add);
    if (res.commit) appendLog(res.commit);
    if (res.push) appendLog(res.push);
    if (res.commitCode === 0 && res.pushCode === 0) {
      toast(t('pushed'), 'success');
      $('#commitMsg').value = '';
    } else {
      toast(t('pushIncomplete'), 'error');
    }
    loadGitStatus();
  } catch (e) {
    toast(e.message, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = t('savePush');
  }
};

/* ---------- 初始化 ---------- */
async function loadLinks() {
  const res = await api('/api/links');
  links = res.data;
  currentGroup = Math.min(currentGroup, Math.max(0, (links.icons || []).length - 1));
  renderGroups();
}

async function loadMyfiles() {
  const res = await api('/api/myfiles');
  myfiles = res.data;
  if (!currentFolder && (myfiles.folders || []).length) currentFolder = myfiles.folders[0].slug;
  renderFolders();
}

async function init() {
  try { applyTheme(); } catch (e) { /* 忽略 */ }
  try { applyLang(); } catch (e) { /* 忽略 */ }
  if (isDesktop) $('#appearanceBtn').hidden = false;
  await loadBgConfig();
  await loadBrowserConfig();
  renderCalendar();
  renderTodos();
  try {
    await Promise.all([loadLinks(), loadMyfiles()]);
  } catch (e) {
    toast(t('loadFailed') + e.message, 'error');
  }
  loadGitStatus();
  setInterval(() => {
    if (document.getElementById('tab-deploy').classList.contains('active')) loadGitStatus();
  }, 10000);
}

/* ---------- 登录门禁（仅桌面应用） ---------- */
async function initAuth() {
  try { applyTheme(); } catch (e) { /* 忽略 */ }
  try { applyLang(); } catch (e) { /* 语言渲染失败也要显示登录框，避免白屏 */ }
  const backdrop = $('#loginBackdrop');
  const errorEl = $('#loginError');
  const pwdInput = $('#loginPassword');
  backdrop.hidden = false;
  pwdInput.focus();
  async function tryLogin() {
    const input = pwdInput.value;
    if (!input) return;
    try {
      const ok = await window.__TAURI__.core.invoke('verify_password', { input });
      if (ok) {
        backdrop.hidden = true;
        $('#btnChangePwd').hidden = false;
        init();
      } else {
        errorEl.hidden = false;
        pwdInput.value = '';
        pwdInput.focus();
      }
    } catch (e) {
      errorEl.hidden = false;
      pwdInput.value = '';
      pwdInput.focus();
    }
  }
  $('#btnLogin').onclick = tryLogin;
  pwdInput.addEventListener('keydown', (e) => { if (e.key === 'Enter') tryLogin(); });
}

/* ---------- 修改密码（仅桌面应用） ---------- */
$('#btnChangePwd').addEventListener('click', () => {
  openModal(t('changePwdTitle'), [
    { key: 'old', label: t('oldPassword'), type: 'password', required: true },
    { key: 'new', label: t('newPassword'), type: 'password', required: true },
    { key: 'confirm', label: t('confirmPassword'), type: 'password', required: true },
  ], async (values) => {
    if (values.new !== values.confirm) {
      toast(t('pwdMismatch'), 'error');
      return;
    }
    try {
      await window.__TAURI__.core.invoke('change_password', { old: values.old, new: values.new });
      toast(t('pwdChanged'), 'success');
    } catch (e) {
      toast(e.message || t('oldPwdWrong'), 'error');
    }
  });
});

if (isDesktop) {
  initAuth();
} else {
  init();
}
