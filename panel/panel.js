/* my-nav 开发者面板 */
const $ = (sel) => document.querySelector(sel);

const ICON_EDIT = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>';
const ICON_DEL = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>';
const ICON_GRIP = '<svg class="drag-handle" viewBox="0 0 24 24" fill="currentColor"><circle cx="9" cy="5" r="1.6"/><circle cx="15" cy="5" r="1.6"/><circle cx="9" cy="12" r="1.6"/><circle cx="15" cy="12" r="1.6"/><circle cx="9" cy="19" r="1.6"/><circle cx="15" cy="19" r="1.6"/></svg>';

/* ---------- 语言包 ---------- */
const LOCALES = {
  zh: {
    appName: '开发者面板', appSub: 'my-nav · 本地内容管理', connected: '已连接', uncommitted: '待推送',
    preview: '预览网站 ↗', navCards: '导航卡片', myFiles: 'My Files', deploy: '部署',
    navDesc: '管理首页的网址卡片与分组，保存后自动写入 links.json',
    recompute: '重新计算标签 / VPN', downloadIcons: '下载图标', save: '保存',
    currentGroup: '当前分组', newGroup: '新建分组', rename: '重命名', delete: '删除',
    noCards: '该分组还没有卡片', noMatch: '没有匹配的卡片', newCard: '新建卡片',
    myfilesDesc: '管理文件浏览器中的文件夹与文件，保存后自动写入 data.json',
    folders: '文件夹', new: '新建', files: '文件', newFile: '新建文件', noFiles: '该文件夹还没有文件',
    deployDesc: '提交并推送到 GitHub，Cloudflare Pages 会自动发布',
    repoStatus: '仓库状态', branch: '分支', lastCommit: '最近提交',
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
  },
  en: {
    appName: 'Developer Panel', appSub: 'my-nav · Local Content Manager', connected: 'Connected', uncommitted: 'Uncommitted',
    preview: 'Preview Site ↗', navCards: 'Nav Cards', myFiles: 'My Files', deploy: 'Deploy',
    navDesc: 'Manage nav cards & groups. Saved to links.json',
    recompute: 'Recompute Tags / VPN', downloadIcons: 'Download Icons', save: 'Save',
    currentGroup: 'Current Group', newGroup: 'New Group', rename: 'Rename', delete: 'Delete',
    noCards: 'No cards in this group yet', noMatch: 'No matching cards', newCard: 'New Card',
    myfilesDesc: 'Manage folders & files in the file explorer. Saved to data.json',
    folders: 'Folders', new: 'New', files: 'Files', newFile: 'New File', noFiles: 'No files in this folder yet',
    deployDesc: 'Commit & push to GitHub. Cloudflare Pages auto-deploys',
    repoStatus: 'Repo Status', branch: 'Branch', lastCommit: 'Last Commit',
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
  },
};

let lang = localStorage.getItem('panel-lang') || 'zh';
let theme = localStorage.getItem('panel-theme') || (window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
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
  $('#btnLang').textContent = lang === 'zh' ? 'EN' : '中';
  setStatusLabel();
  renderGroups();
  renderFolders();
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
    return `<div class="checkbox-row"><input type="checkbox" id="f_${f.key}" ${f.value ? 'checked' : ''}><label for="f_${f.key}">${f.label}</label></div>`;
  }
  if (f.type === 'select') {
    const opts = (f.options || []).map((o) => `<option value="${escapeAttr(o.value)}" ${o.value === f.value ? 'selected' : ''}>${escapeHtml(o.label)}</option>`).join('');
    return `<div class="form-row"><label>${f.label}</label><select id="f_${f.key}" class="select">${opts}</select></div>`;
  }
  const tag = f.type === 'textarea' ? 'textarea' : 'input';
  const typeAttr = f.type === 'number' ? 'number' : 'text';
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
async function loadGitStatus() {
  try {
    const res = await api('/api/git-status');
    $('#gitBranch').textContent = res.branch || '—';
    $('#gitLast').textContent = res.last || '—';
    const changes = res.status ? res.status.split('\n').filter(Boolean) : [];
    statusDirty = changes.length > 0;
    setStatusLabel();
    $('#serverStatus').classList.toggle('dirty', statusDirty);
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

/* ---------- 事件绑定 ---------- */
document.querySelectorAll('.tab-btn').forEach((btn) => {
  btn.onclick = () => {
    document.querySelectorAll('.tab-btn').forEach((b) => b.classList.remove('active'));
    document.querySelectorAll('.tab-panel').forEach((p) => p.classList.remove('active'));
    btn.classList.add('active');
    document.getElementById('tab-' + btn.dataset.tab).classList.add('active');
    if (btn.dataset.tab === 'deploy') loadGitStatus();
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
  applyTheme();
  applyLang();
  try {
    await Promise.all([loadLinks(), loadMyfiles()]);
  } catch (e) {
    toast(t('loadFailed') + e.message, 'error');
  }
  loadGitStatus();
  setInterval(loadGitStatus, 10000);
}

init();
