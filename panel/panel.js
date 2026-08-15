/* my-nav 开发者面板 */
const $ = (sel) => document.querySelector(sel);

const ICON_EDIT = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17 3a2.85 2.83 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>';
const ICON_DEL = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>';
const ICON_X = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6L6 18"/><path d="M6 6l12 12"/></svg>';

let links = null;
let myfiles = null;
let currentGroup = 0;
let currentFolder = null;

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
  const data = await res.json().catch(() => ({ ok: false, error: '响应解析失败' }));
  if (!res.ok || data.ok === false) throw new Error(data.error || `请求失败 (${res.status})`);
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
      toast('请填写必填项', 'error');
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
  sel.innerHTML = (links.icons || []).map((g, i) =>
    `<option value="${i}">${escapeHtml(g.title)} (${(g.children || []).length})</option>`).join('');
  sel.value = currentGroup;
  renderCards();
}

function iconFallback(el) {
  const title = el.dataset.title || '';
  el.outerHTML = `<span class="letter">${escapeHtml((title || '?')[0].toUpperCase())}</span>`;
}

function renderCards() {
  const group = links.icons[currentGroup];
  const list = $('#cardList');
  const empty = $('#cardEmpty');
  if (!group || !(group.children || []).length) {
    list.innerHTML = '';
    empty.hidden = false;
    return;
  }
  empty.hidden = true;
  list.innerHTML = group.children.map((item, i) => {
    const icon = item.icon && item.icon.src
      ? `<img src="${escapeAttr(item.icon.src)}" data-title="${escapeAttr(item.title)}" alt="" onerror="iconFallback(this)">`
      : `<span class="letter">${escapeHtml((item.title || '?')[0].toUpperCase())}</span>`;
    const vpn = item.isVpnRequired
      ? '<span class="badge badge-vpn">VPN</span>'
      : '<span class="badge badge-direct">直连</span>';
    const tags = (item.tags || []).map((t) => `<span class="tag">${escapeHtml(t)}</span>`).join('');
    return `<div class="card-item">
      <div class="card-icon">${icon}</div>
      <div class="card-info">
        <div class="card-title-row"><span class="name">${escapeHtml(item.title)}</span>${vpn}</div>
        <div class="card-url">${escapeHtml(item.url)}</div>
        ${tags ? `<div class="card-tags">${tags}</div>` : ''}
      </div>
      <div class="card-actions">
        <button class="icon-btn" data-edit="${i}" title="编辑">${ICON_EDIT}</button>
        <button class="icon-btn danger" data-del="${i}" title="删除">${ICON_DEL}</button>
      </div>
    </div>`;
  }).join('');
}

function openCardModal(item, index) {
  openModal(index === null ? '新建卡片' : '编辑卡片', [
    { key: 'title', label: '标题', value: item.title, required: true },
    { key: 'url', label: 'URL', value: item.url, required: true, placeholder: 'https://' },
    { key: 'icon', label: '图标 URL', value: (item.icon && item.icon.src) || '', placeholder: 'https://example.com/favicon.ico', hint: '留空则显示字母图标；保存后点「下载图标」可自动抓取 favicon' },
    { key: 'description', label: '描述', value: item.description || '' },
    { key: 'tags', label: '标签（逗号分隔）', value: (item.tags || []).join(', ') },
    { key: 'vpn', label: '需要 VPN 才能访问', type: 'checkbox', value: !!item.isVpnRequired },
  ], (values) => {
    const card = {
      icon: { text: '', itemType: 2, src: values.icon, backgroundColor: '' },
      sort: item.sort ?? 99999,
      title: values.title,
      url: values.url,
      openMethod: item.openMethod ?? 1,
      lanUrl: item.lanUrl ?? '',
      description: values.description,
      tags: values.tags ? values.tags.split(/[,，]/).map((t) => t.trim()).filter(Boolean) : [],
      isVpnRequired: values.vpn,
      clickCount: item.clickCount ?? 0,
    };
    const group = links.icons[currentGroup];
    if (index === null) group.children.push(card);
    else group.children[index] = card;
    renderCards();
    renderGroups();
    toast('已修改，记得点「保存」');
  });
}

async function runScript(script, btn, successMsg) {
  btn.disabled = true;
  const original = btn.textContent;
  btn.textContent = '运行中…';
  try {
    const res = await api('/api/run-script', { method: 'POST', body: JSON.stringify({ script }) });
    appendLog(res.output);
    if (res.ok) {
      toast(successMsg, 'success');
      if (script === 'download_icons' || script === 'enhance_links') await loadLinks();
    } else {
      toast('脚本运行出错，请查看日志', 'error');
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
  list.innerHTML = (myfiles.folders || []).map((f, i) => {
    const count = (f.files || []).length + (f.folders || []).length;
    return `<div class="folder-item ${f.slug === currentFolder ? 'active' : ''}" data-folder="${i}">
      <span class="fname">${escapeHtml(f.name)}</span>
      ${f.protected ? '<span class="badge badge-vpn">保护</span>' : ''}
      <span class="fcount">${count}</span>
      <button class="icon-btn danger fdel" data-fdel="${i}" title="删除">${ICON_DEL}</button>
    </div>`;
  }).join('');
  renderFiles();
}

function renderFiles() {
  const folder = (myfiles.folders || []).find((f) => f.slug === currentFolder);
  const list = $('#fileList');
  const empty = $('#fileEmpty');
  $('#fileColTitle').textContent = folder ? `文件 · ${folder.name}` : '文件';
  if (!folder || !(folder.files || []).length) {
    list.innerHTML = '';
    empty.hidden = false;
    return;
  }
  empty.hidden = true;
  list.innerHTML = folder.files.map((f, i) => {
    const kindLabel = f.kind === 'source' ? '源码' : '报告';
    return `<div class="file-item">
      <span class="fname">${escapeHtml(f.name)}</span>
      <span class="fmeta">${kindLabel}${f.size ? ' · ' + escapeHtml(f.size) : ''}</span>
      <div class="factions">
        <button class="icon-btn" data-fedit="${i}" title="编辑">${ICON_EDIT}</button>
        <button class="icon-btn danger" data-fdel="${i}" title="删除">${ICON_DEL}</button>
      </div>
    </div>`;
  }).join('');
}

function openFileModal(file, index) {
  openModal(index === null ? '新建文件' : '编辑文件', [
    { key: 'name', label: '文件名', value: file.name, required: true },
    { key: 'kind', label: '类型', type: 'select', options: [{ value: 'report', label: '报告 report' }, { value: 'source', label: '源码 source' }], value: file.kind || 'report' },
    { key: 'size', label: '大小', value: file.size || '', placeholder: '如 25 KB' },
    { key: 'url', label: 'URL（相对路径，留空用文件名）', value: file.url || '' },
    { key: 'qmd', label: '源码 QMD 文件名（可选）', value: (file.source && file.source.qmd) || '' },
    { key: 'figures', label: '图表 ZIP 文件名（可选）', value: (file.source && file.source.figures) || '' },
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
    toast('已修改，记得点「保存」');
  });
}

/* ---------- 部署 ---------- */
async function loadGitStatus() {
  try {
    const res = await api('/api/git-status');
    $('#gitBranch').textContent = res.branch || '—';
    $('#gitLast').textContent = res.last || '—';
    const changes = res.status ? res.status.split('\n').filter(Boolean) : [];
    const el = $('#gitChanges');
    if (!changes.length) {
      el.innerHTML = '<div class="muted">工作区干净，无未提交改动</div>';
    } else {
      el.innerHTML = changes.map((line) => {
        const st = line.slice(0, 2).trim() || '??';
        const file = line.slice(3);
        return `<div class="change-line"><span class="st ${st}">${st}</span><span>${escapeHtml(file)}</span></div>`;
      }).join('');
    }
  } catch (e) {
    $('#gitChanges').innerHTML = `<div class="muted">无法读取 git 状态：${escapeHtml(e.message)}</div>`;
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

$('#groupSelect').onchange = () => {
  currentGroup = Number($('#groupSelect').value);
  renderCards();
};

$('#btnNewGroup').onclick = () => {
  openModal('新建分组', [{ key: 'title', label: '分组名称', value: '', required: true }], (v) => {
    links.icons.push({ title: v.title, sort: 0, children: [] });
    currentGroup = links.icons.length - 1;
    renderGroups();
    toast('已新建分组，记得点「保存」');
  });
};

$('#btnRenameGroup').onclick = () => {
  const g = links.icons[currentGroup];
  openModal('重命名分组', [{ key: 'title', label: '分组名称', value: g.title, required: true }], (v) => {
    g.title = v.title;
    renderGroups();
    toast('已重命名，记得点「保存」');
  });
};

$('#btnDeleteGroup').onclick = () => {
  const g = links.icons[currentGroup];
  if (!confirm(`确定删除分组「${g.title}」及其全部卡片？`)) return;
  links.icons.splice(currentGroup, 1);
  currentGroup = Math.max(0, currentGroup - 1);
  renderGroups();
  toast('已删除分组，记得点「保存」');
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
    if (confirm(`确定删除卡片「${item.title}」？`)) {
      links.icons[currentGroup].children.splice(i, 1);
      renderCards();
      renderGroups();
      toast('已删除，记得点「保存」');
    }
  }
});

$('#btnSaveLinks').onclick = async () => {
  const btn = $('#btnSaveLinks');
  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    await api('/api/links', { method: 'POST', body: JSON.stringify({ data: links }) });
    toast('links.json 已保存', 'success');
  } catch (e) {
    toast(e.message, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = '保存';
  }
};

$('#btnDownloadIcons').onclick = () => runScript('download_icons', $('#btnDownloadIcons'), '图标已下载并更新');
$('#btnEnhance').onclick = () => runScript('enhance_links', $('#btnEnhance'), '标签 / VPN 已重新计算');

$('#folderList').addEventListener('click', (e) => {
  const delBtn = e.target.closest('[data-fdel]');
  if (delBtn) {
    e.stopPropagation();
    const i = Number(delBtn.dataset.fdel);
    const f = myfiles.folders[i];
    if (confirm(`确定从 My Files 中移除「${f.name}」？\n（仅从 data.json 移除，磁盘上的文件保留）`)) {
      myfiles.folders.splice(i, 1);
      if (currentFolder === f.slug) currentFolder = (myfiles.folders[0] && myfiles.folders[0].slug) || null;
      renderFolders();
      toast('已移除，记得点「保存」');
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
  openModal('新建文件夹', [
    { key: 'slug', label: 'slug（URL 标识，小写字母/数字/连字符）', value: '', required: true, placeholder: 'my-folder' },
    { key: 'name', label: '显示名称', value: '', required: true },
    { key: 'protected', label: '需要密码保护', type: 'checkbox', value: false },
  ], async (v) => {
    try {
      await api('/api/create-folder', { method: 'POST', body: JSON.stringify({ slug: v.slug, name: v.name, protected: v.protected }) });
      myfiles.folders.push({ slug: v.slug, name: v.name, protected: v.protected, dateAdded: today(), folders: [], files: [] });
      currentFolder = v.slug;
      renderFolders();
      toast('文件夹已创建（含页面样板文件）', 'success');
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
    if (confirm(`确定删除文件「${f.name}」？`)) {
      folder.files.splice(i, 1);
      renderFiles();
      renderFolders();
      toast('已删除，记得点「保存」');
    }
  }
});

$('#btnNewFile').onclick = () => openFileModal({ name: '', kind: 'report', size: '', url: '', source: {} }, null);
$('#btnNewFileEmpty').onclick = () => openFileModal({ name: '', kind: 'report', size: '', url: '', source: {} }, null);

$('#btnSaveMyfiles').onclick = async () => {
  const btn = $('#btnSaveMyfiles');
  btn.disabled = true;
  btn.textContent = '保存中…';
  try {
    await api('/api/myfiles', { method: 'POST', body: JSON.stringify({ data: myfiles }) });
    toast('data.json 已保存', 'success');
  } catch (e) {
    toast(e.message, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = '保存';
  }
};

$('#btnPush').onclick = async () => {
  const btn = $('#btnPush');
  const msg = $('#commitMsg').value.trim();
  if (!msg) { toast('请填写提交信息', 'error'); return; }
  btn.disabled = true;
  btn.textContent = '推送中…';
  appendLog('$ git add -A && git commit && git push');
  try {
    const res = await api('/api/git-push', { method: 'POST', body: JSON.stringify({ message: msg }) });
    if (res.add) appendLog(res.add);
    if (res.commit) appendLog(res.commit);
    if (res.push) appendLog(res.push);
    if (res.commitCode === 0 && res.pushCode === 0) {
      toast('已推送到 GitHub', 'success');
      $('#commitMsg').value = '';
    } else {
      toast('推送未完全成功，请查看日志', 'error');
    }
    loadGitStatus();
  } catch (e) {
    toast(e.message, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = '保存并推送';
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
  try {
    await Promise.all([loadLinks(), loadMyfiles()]);
  } catch (e) {
    toast('加载数据失败：' + e.message, 'error');
  }
}

init();
