(function () {
  const DATA_URL = '/myfiles/data.json';

  const SVG = {
    folder: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z"/></svg>',
    report: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M7 3h7l5 5v13a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1z"/><path d="M14 3v5h5"/></svg>',
    code: '<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>',
  };

  function escapeHtml(s) {
    return String(s).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]));
  }

  async function init() {
    let data;
    try {
      data = await (await fetch(DATA_URL)).json();
    } catch (e) {
      document.getElementById('explorer').innerHTML = '<div class="notice">Failed to load data.json — please refresh.</div>';
      return;
    }

    const segments = location.pathname.replace(/^\/+|\/+$/g, '').split('/').filter(Boolean);
    const folderPath = segments.slice(1); // first segment is 'myfiles'

    const root = { name: data.name || 'My Files', folders: data.folders || [], files: data.files || [], protected: false };

    let node = root;
    let nodePath = '/myfiles';
    const crumbs = [];
    let ok = true;
    for (const slug of folderPath) {
      const child = (node.folders || []).find((f) => f.slug === slug);
      if (!child) { ok = false; break; }
      nodePath += '/' + slug;
      crumbs.push({ name: child.name || slug, path: nodePath });
      node = child;
    }
    if (!ok) {
      document.getElementById('explorer').innerHTML = '<div class="notice">Folder not found.</div>';
      return;
    }

    // breadcrumb
    const bc = document.getElementById('breadcrumb');
    bc.innerHTML = [
      '<a href="/">Home</a>',
      '<span class="sep">›</span>',
      '<a href="/myfiles/">My Files</a>',
      crumbs.map((c, i) => {
        const isLast = i === crumbs.length - 1;
        return isLast
          ? `<span class="sep">›</span><span class="current">${escapeHtml(c.name)}</span>`
          : `<span class="sep">›</span><a href="${c.path}/">${escapeHtml(c.name)}</a>`;
      }).join(''),
    ].join('');

    // hero with sort/filter toolbar
    const heroEl = document.getElementById('hero');
    heroEl.innerHTML = [
      `<h1>${escapeHtml(node.name)}</h1>`,
      node.protected ? '<span class="badge lock"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg><span>Password protected</span></span>' : '',
      `<div class="toolbar">
        <input type="text" id="filterInput" class="toolbar-search" placeholder="Search folders and files...">
        <select id="sortSelect" class="toolbar-sort">
          <option value="name-asc">Name A-Z</option>
          <option value="name-desc">Name Z-A</option>
          <option value="date-desc">Date Added (Newest)</option>
          <option value="date-asc">Date Added (Oldest)</option>
        </select>
      </div>`
    ].join('');

    const filterInput = document.getElementById('filterInput');
    const sortSelect = document.getElementById('sortSelect');

    function getSortFn() {
      const mode = sortSelect ? sortSelect.value : 'name-asc';
      switch (mode) {
        case 'name-asc': return (a, b) => (a.name || a.slug || '').localeCompare(b.name || b.slug || '');
        case 'name-desc': return (a, b) => (b.name || b.slug || '').localeCompare(a.name || a.slug || '');
        case 'date-desc': return (a, b) => ((b.dateAdded || '1970-01-01') > (a.dateAdded || '1970-01-01') ? 1 : -1);
        case 'date-asc': return (a, b) => ((a.dateAdded || '1970-01-01') > (b.dateAdded || '1970-01-01') ? 1 : -1);
        default: return (a, b) => (a.name || a.slug || '').localeCompare(b.name || b.slug || '');
      }
    }

    function getFilterTerm() {
      return filterInput ? filterInput.value.trim().toLowerCase() : '';
    }

    function render() {
      const term = getFilterTerm();
      const sortFn = getSortFn();

      // subfolders as a list
      const subFolders = (node.folders || []).slice().sort(sortFn).filter((f) => {
        if (!term) return true;
        return (f.name || f.slug || '').toLowerCase().includes(term);
      });
      const foldersEl = document.getElementById('folders');
      foldersEl.innerHTML = subFolders.length
        ? `<div class="section-title">Folders</div><div class="list">` + subFolders.map((f) => {
            const count = (f.files ? f.files.length : 0) + (f.folders ? f.folders.length : 0);
            return `<a href="${nodePath}/${encodeURIComponent(f.slug)}/" class="glass row">
              <div class="icon folder">${SVG.folder}</div>
              <div class="name">${escapeHtml(f.name || f.slug)}</div>
              <div class="meta">
                <span class="size-badge">${count} items</span>
                ${f.protected ? '<span class="lock-icon" title="Protected"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="11" width="18" height="11" rx="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/></svg></span>' : ''}
                <span class="action">Open ›</span>
              </div>
            </a>`;
          }).join('') + `</div>`
        : '';

      // files as a list
      const files = (node.files || []).slice().sort(sortFn).filter((f) => {
        if (!term) return true;
        return (f.name || '').toLowerCase().includes(term);
      });
      const filesEl = document.getElementById('files');
      filesEl.innerHTML = files.length
        ? `<div class="section-title">Files</div><div class="list">` + files.map((f, index) => {
            const isSource = f.kind === 'source';
            const href = `${nodePath}/${encodeURIComponent(f.name)}`;
            const source = f.source || null;
            const menuId = `source-menu-${index}`;
            const rowMain = isSource
              ? `<a href="${href}" class="row-main" download>`
              : `<a href="${href}" class="row-main">`;
            const sourceButton = source
              ? `<button type="button" class="source-btn" data-source-menu="${escapeHtml(menuId)}" aria-expanded="false" aria-controls="${escapeHtml(menuId)}">Source</button>`
              : '';
            const sourceMenu = source
              ? `<div class="source-menu" id="${escapeHtml(menuId)}" hidden>
                   <a href="${nodePath}/${encodeURIComponent(source.qmd || f.name.replace(/\.html$/, '.qmd'))}" download>Download QMD</a>
                   <a href="${nodePath}/${encodeURIComponent(source.figures)}" download>Download Figures ZIP</a>
                 </div>`
              : '';
            return `<div class="glass row file-row">
              ${rowMain}
                <div class="icon ${isSource ? 'source' : 'report'}">${isSource ? SVG.code : SVG.report}</div>
                <div class="name">${escapeHtml(f.name)}</div>
                <div class="meta">
                  <span class="size-badge">${escapeHtml(f.size || '')}</span>
                  <span class="action">${isSource ? 'Download' : 'Open >'}</span>
                </div>
              </a>
              ${sourceButton}
              ${sourceMenu}
            </div>`;
          }).join('') + `</div>`
        : '';
    }

    if (filterInput) filterInput.addEventListener('input', render);
    if (sortSelect) sortSelect.addEventListener('change', render);

    const filesEl = document.getElementById('files');
    filesEl.addEventListener('click', (event) => {
      const toggle = event.target.closest('[data-source-menu]');
      if (toggle) {
        event.preventDefault();
        const menu = document.getElementById(toggle.getAttribute('data-source-menu'));
        const open = toggle.getAttribute('aria-expanded') === 'true';
        filesEl.querySelectorAll('.source-menu').forEach((m) => { m.hidden = true; });
        filesEl.querySelectorAll('.source-btn').forEach((b) => b.setAttribute('aria-expanded', 'false'));
        if (!open && menu) {
          menu.hidden = false;
          toggle.setAttribute('aria-expanded', 'true');
        }
        return;
      }
      if (!event.target.closest('.source-menu')) {
        filesEl.querySelectorAll('.source-menu').forEach((m) => { m.hidden = true; });
        filesEl.querySelectorAll('.source-btn').forEach((b) => b.setAttribute('aria-expanded', 'false'));
      }
    });
    document.addEventListener('click', (event) => {
      if (!event.target.closest('#files .file-row')) {
        filesEl.querySelectorAll('.source-menu').forEach((m) => { m.hidden = true; });
        filesEl.querySelectorAll('.source-btn').forEach((b) => b.setAttribute('aria-expanded', 'false'));
      }
    });

    render();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();
