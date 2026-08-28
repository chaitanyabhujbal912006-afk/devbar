const diskListEl       = document.getElementById('disk-list');
const repoListEl       = document.getElementById('repo-list');
const containerListEl  = document.getElementById('container-list');
const portListEl       = document.getElementById('port-list');
const lastUpdatedEl    = document.getElementById('last-updated');
const refreshBtn       = document.getElementById('refresh-btn');
const settingsBtn      = document.getElementById('settings-btn');
const settingsPanel    = document.getElementById('settings-panel');
const watchDirsListEl  = document.getElementById('watch-dirs-list');
const addDirForm       = document.getElementById('add-dir-form');
const dirInput         = document.getElementById('dir-input');

const { invoke } = window.__TAURI__.core;

let currentWatchDirs = [];

// ─── macOS Segmented Tab Navigation ──────────────────────────────────────
const macTabBtns = document.querySelectorAll('.mac-tab-btn');
const blockTabMap = {
  'resume-block': ['all', 'repos'],
  'git-block': ['all', 'repos'],
  'env-block': ['all', 'security'],
  'ports-block': ['all', 'services'],
  'containers-block': ['all', 'services'],
  'disk-block': ['all', 'services'],
  'deps-block': ['all', 'deps'],
};

macTabBtns.forEach(btn => {
  btn.addEventListener('click', () => {
    macTabBtns.forEach(b => b.classList.remove('active'));
    btn.classList.add('active');

    const tab = btn.dataset.tab;
    Object.entries(blockTabMap).forEach(([blockId, allowedTabs]) => {
      const el = document.getElementById(blockId);
      if (el) {
        if (allowedTabs.includes(tab)) {
          el.classList.remove('hidden');
        } else {
          el.classList.add('hidden');
        }
      }
    });
  });
});


function renderDisks(disks) {
  diskListEl.innerHTML = "";
  if (!disks.length) {
    diskListEl.innerHTML = `<div class="empty">No disks found</div>`;
    return;
  }
  for (const d of disks) {
    const row = document.createElement("div");
    row.className = "item";
    row.innerHTML = `
      <span><span class="dot ${d.status}"></span>${d.name || d.mount_point}</span>
      <span class="meta">${d.free_gb} GB free / ${d.total_gb} GB (${d.percent_used}%)</span>
    `;
    diskListEl.appendChild(row);
  }
}

function renderRepos(repos) {
  repoListEl.innerHTML = "";
  if (!repos || !Array.isArray(repos) || !repos.length) {
    repoListEl.innerHTML = `<div class="empty">No git repos found in watched folders</div>`;
    return;
  }
  for (const r of repos) {
    const status = r.dirty ? "warn" : "ok";
    const card = document.createElement("div");
    card.className = "repo-card-item";
    card.title = r.path || "";

    const hasChanges = (r.changed_files ?? 0) > 0 || (r.unpushed_commits ?? 0) > 0;
    const metaText = `${r.changed_files ?? 0} changed · ${r.unpushed_commits ?? 0} unpushed`;
    const metaClass = hasChanges ? "repo-badge badge-warn" : "repo-badge badge-ok";

    card.innerHTML = `
      <div class="repo-row-top">
        <div class="repo-name-box">
          <span class="dot ${status}"></span>
          <span class="repo-title-text">${r.name || "Unknown"}</span>
          <span class="repo-branch-tag">${r.branch || "main"}</span>
        </div>
        <div class="btn-group">
          <button class="btn-script-runner" data-path="${r.path || ''}" data-name="${r.name || ''}" title="Quick Actions / Scripts">⚡ Actions</button>
          <div class="split-btn-group">
            <button class="open-vscode-btn" data-path="${r.path || ''}" title="Open in default editor">Open</button>
            <button class="open-with-trigger" data-path="${r.path || ''}" title="Open with…">▾</button>
          </div>
        </div>
      </div>
      <div class="repo-row-bottom">
        <span class="${metaClass}">${metaText}</span>
        <span class="repo-path-sub">${r.path || ''}</span>
      </div>
    `;

    repoListEl.appendChild(card);
  }
}


function renderContainers(docker) {
  containerListEl.innerHTML = "";
  if (!docker.available) {
    containerListEl.innerHTML = `<div class="empty">Docker not available</div>`;
    return;
  }
  if (!docker.containers.length) {
    containerListEl.innerHTML = `<div class="empty">No containers found</div>`;
    return;
  }
  for (const c of docker.containers) {
    const dot = c.state === "running" ? "ok"
               : c.state === "restarting" ? "warn"
               : "critical";
    const row = document.createElement("div");
    row.className = "item";
    row.title = c.status;
    row.innerHTML = `
      <span><span class="dot ${dot}"></span>${c.name} <span class="meta">(${c.image})</span></span>
      <span class="meta">${c.status}</span>
    `;
    containerListEl.appendChild(row);
  }
}

function renderWatchDirs(dirs) {
  currentWatchDirs = dirs;
  watchDirsListEl.innerHTML = "";
  if (!dirs.length) {
    watchDirsListEl.innerHTML = `<div class="empty">No folders watched</div>`;
    return;
  }
  dirs.forEach((dir, index) => {
    const item = document.createElement("div");
    item.className = "dir-item";
    item.innerHTML = `
      <span class="dir-path" title="${dir}">${dir}</span>
      <button class="remove-dir-btn" data-index="${index}" title="Remove folder">×</button>
    `;
    watchDirsListEl.appendChild(item);
  });
}

async function loadWatchDirs() {
  try {
    const dirs = await invoke("get_watch_dirs");
    renderWatchDirs(dirs);
  } catch (err) {
    console.error("Failed to load watch dirs:", err);
  }
}

async function updateWatchDirs(newDirs) {
  try {
    const updated = await invoke("set_watch_dirs", { dirs: newDirs });
    renderWatchDirs(updated);
    await refresh();
  } catch (err) {
    console.error("Failed to update watch dirs:", err);
  }
}

settingsBtn.addEventListener('click', () => {
  const isHidden = settingsPanel.classList.contains('hidden');
  if (isHidden) {
    settingsPanel.classList.remove('hidden');
    settingsBtn.classList.add('active');
  } else {
    settingsPanel.classList.add('hidden');
    settingsBtn.classList.remove('active');
  }
});

watchDirsListEl.addEventListener("click", (e) => {
  if (e.target.classList.contains("remove-dir-btn")) {
    const index = parseInt(e.target.dataset.index, 10);
    if (!isNaN(index)) {
      const nextDirs = [...currentWatchDirs];
      nextDirs.splice(index, 1);
      updateWatchDirs(nextDirs);
    }
  }
});

addDirForm.addEventListener("submit", (e) => {
  e.preventDefault();
  const newDir = dirInput.value.trim();
  if (newDir && !currentWatchDirs.includes(newDir)) {
    const nextDirs = [...currentWatchDirs, newDir];
    updateWatchDirs(nextDirs);
    dirInput.value = "";
  }
});

let lastRefreshTime = null;

function updateTimestampDisplay() {
  if (!lastRefreshTime) return;
  const elapsedSec = Math.floor((Date.now() - lastRefreshTime) / 1000);
  if (elapsedSec < 5) {
    lastUpdatedEl.textContent = "Updated just now";
  } else if (elapsedSec < 60) {
    lastUpdatedEl.textContent = `Updated ${elapsedSec}s ago`;
  } else {
    const min = Math.floor(elapsedSec / 60);
    lastUpdatedEl.textContent = `Updated ${min}m ago`;
  }
}

function renderPorts(ports) {
  if (!portListEl) return;
  portListEl.innerHTML = "";
  if (!ports || !Array.isArray(ports) || !ports.length) {
    portListEl.innerHTML = `<div class="empty">No port data</div>`;
    return;
  }
  for (const p of ports) {
    const dotClass = p.in_use ? "critical" : "ok";
    const row = document.createElement("div");
    row.className = "item";
    if (p.in_use) {
      row.title = `Click port to open http://localhost:${p.port} (PID ${p.pid ?? "?"} — ${p.process_name ?? "Unknown"})`;
    }
    const procLabel = p.in_use
      ? ` <span class="meta">(${p.process_name ?? "Unknown"})</span>`
      : "";
    const rightLabel = p.in_use
      ? `<span class="meta">PID ${p.pid}</span>`
      : `<span class="meta port-free">free</span>`;

    // Open in browser button & Kill button
    const openBtn = p.in_use
      ? `<button class="open-url-btn" data-url="http://localhost:${p.port}" title="Open http://localhost:${p.port} in web browser">🌐 Open</button>`
      : '';

    const killBtn = p.in_use && p.pid
      ? `<button class="kill-port-btn" data-pid="${p.pid}" data-port="${p.port}" title="Kill process on :${p.port}">✕</button>`
      : '';

    const portNumHtml = p.in_use
      ? `<strong class="port-num port-link" data-url="http://localhost:${p.port}" title="Click to open http://localhost:${p.port}">:${p.port}</strong>`
      : `<strong class="port-num">:${p.port}</strong>`;

    row.innerHTML = `
      <span><span class="dot ${dotClass}"></span>${portNumHtml}${procLabel}</span>
      <span style="display:flex;align-items:center;gap:6px;">${rightLabel}${openBtn}${killBtn}</span>
    `;
    portListEl.appendChild(row);
  }
}


if (refreshBtn) {
  refreshBtn.addEventListener("click", refresh);
}

// Kill-port button — delegated on portListEl
if (portListEl) {
  portListEl.addEventListener('click', async (e) => {
    const btn = e.target.closest('.kill-port-btn');
    if (!btn) return;
    const pid = parseInt(btn.dataset.pid, 10);
    const port = btn.dataset.port;
    if (isNaN(pid)) return;

    btn.disabled = true;
    btn.textContent = '…';
    try {
      await invoke('kill_process_on_port', { pid });
      btn.textContent = '✓';
      setTimeout(() => refresh(), 800);
    } catch (err) {
      console.warn('[devbar] kill_process_on_port failed:', err);
      btn.textContent = '!';
      btn.title = `Kill failed: ${err}`;
      btn.classList.add('kill-port-btn--error');
      setTimeout(() => {
        btn.textContent = '✕';
        btn.disabled = false;
        btn.classList.remove('kill-port-btn--error');
      }, 3000);
    }
  });
}

// VS Code open button — delegated listener on the repo list
repoListEl.addEventListener("click", async (e) => {
  const btn = e.target.closest(".open-vscode-btn");
  if (!btn) return;
  const path = btn.dataset.path;
  if (!path) return;

  btn.disabled = true;
  btn.textContent = "…";

  try {
    await invoke("open_in_vscode", { path });
    btn.textContent = "✓";
    setTimeout(() => { btn.textContent = "Open"; btn.disabled = false; }, 1500);
  } catch (err) {
    console.warn("[devbar] open_in_vscode failed:", err);
    btn.textContent = "!";
    btn.title = `VS Code CLI error: ${err}`;
    btn.classList.add("open-vscode-btn--error");
    setTimeout(() => {
      btn.textContent = "Open";
      btn.disabled = false;
      btn.classList.remove("open-vscode-btn--error");
    }, 3000);
  }
});

// ─── Resume Work ────────────────────────────────────────────────────────────
const resumeListEl = document.getElementById('resume-list');

async function loadRecentFiles() {
  if (!resumeListEl) return;
  try {
    const files = await invoke('cmd_get_recent_files');
    renderRecentFiles(files);
  } catch (err) {
    console.error('[devbar] loadRecentFiles error:', err);
    resumeListEl.innerHTML = `<div class="empty">Could not load recent files</div>`;
  }
}

function renderRecentFiles(files) {
  resumeListEl.innerHTML = '';
  if (!files || !files.length) {
    resumeListEl.innerHTML = `<div class="empty">No recent edits found</div>`;
    return;
  }
  for (const f of files) {
    const item = document.createElement('div');
    item.className = 'resume-item';
    item.innerHTML = `
      <div class="resume-left">
        <span class="resume-repo">${f.repo}</span>
        <span class="resume-file" title="${f.absolute_path}">${f.relative_path}</span>
      </div>
      <div class="resume-right">
        <span class="resume-age">${f.age}</span>
        <div class="btn-group">
          <button class="open-vscode-btn" data-path="${f.absolute_path}" title="Open ${f.relative_path}">Open</button>
          <button class="open-with-trigger" data-path="${f.absolute_path}" title="Open with…">▾</button>
        </div>
      </div>
    `;
    resumeListEl.appendChild(item);
  }
}

// ─── Dependencies Dashboard ──────────────────────────────────────────────────
const depsListEl = document.getElementById('deps-list');

async function loadDeps() {
  if (!depsListEl) return;
  try {
    const data = await invoke('cmd_get_dep_versions');
    renderDeps(data);
  } catch (err) {
    console.error('[devbar] loadDeps error:', err);
    depsListEl.innerHTML = `<div class="empty">Could not load dependency matrix</div>`;
  }
}

const LANG_ICONS = { js: '🟨', rust: '🦀', python: '🐍', go: '🐹' };

function renderDeps(repoDeps) {
  depsListEl.innerHTML = '';
  if (!repoDeps || !repoDeps.length) {
    depsListEl.innerHTML = `<div class="empty">No package.json, Cargo.toml, pyproject.toml, or go.mod found in watched folders</div>`;
    return;
  }

  // 1. Collect all package names
  const pkgCounts = {};
  const repos = repoDeps.map(rd => rd.repo);

  repoDeps.forEach(rd => {
    Object.keys(rd.deps || {}).forEach(pkg => {
      pkgCounts[pkg] = (pkgCounts[pkg] || 0) + 1;
    });
  });

  const pkgs = Object.keys(pkgCounts).sort();

  if (!pkgs.length) {
    depsListEl.innerHTML = `<div class="empty">No tracked dependencies found</div>`;
    return;
  }

  // 2. Build Pivot Table HTML
  let tableHtml = `<table class="deps-table"><thead><tr><th>Package</th>`;
  repoDeps.forEach(rd => {
    const icon = LANG_ICONS[rd.lang] || '📦';
    tableHtml += `<th title="${rd.repo} (${rd.lang})">${icon} ${rd.repo.length > 7 ? rd.repo.slice(0, 6) + '…' : rd.repo}</th>`;
  });
  tableHtml += `</tr></thead><tbody>`;

  pkgs.forEach(pkg => {
    const versions = repoDeps.map(rd => rd.deps[pkg]).filter(Boolean);
    const isUniform = new Set(versions).size <= 1;
    const badgeClass = isUniform ? 'ver-match' : 'ver-diff';

    tableHtml += `<tr><td class="deps-pkg-name">${pkg}</td>`;
    repoDeps.forEach(rd => {
      const ver = rd.deps[pkg];
      if (ver) {
        tableHtml += `<td><span class="ver-badge ${badgeClass}">${ver}</span></td>`;
      } else {
        tableHtml += `<td><span style="color: var(--text-dim);">-</span></td>`;
      }
    });
    tableHtml += `</tr>`;
  });

  tableHtml += `</tbody></table>`;
  depsListEl.innerHTML = tableHtml;
}


// ─── Global Search (Ctrl+K Overlay) ─────────────────────────────────────────
const searchModal       = document.getElementById('search-modal');
const searchInput       = document.getElementById('search-input');
const searchResultsEl   = document.getElementById('search-results');
const searchTriggerBtn = document.getElementById('search-trigger-btn');

let searchDebounceTimer = null;
let searchHits          = [];
let selectedIndex       = 0;

function openSearch() {
  searchModal.classList.remove('hidden');
  searchInput.value = '';
  searchInput.focus();
  searchResultsEl.innerHTML = `<div class="empty">Type at least 2 characters to search...</div>`;
}

function closeSearch() {
  searchModal.classList.add('hidden');
}

if (searchTriggerBtn) {
  searchTriggerBtn.addEventListener('click', openSearch);
}

document.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
    e.preventDefault();
    if (searchModal.classList.contains('hidden')) {
      openSearch();
    } else {
      closeSearch();
    }
  } else if (e.key === 'Escape' && !searchModal.classList.contains('hidden')) {
    closeSearch();
  }
});

if (searchInput) {
  searchInput.addEventListener('input', () => {
    clearTimeout(searchDebounceTimer);
    const q = searchInput.value.trim();
    if (q.length < 2) {
      searchResultsEl.innerHTML = `<div class="empty">Type at least 2 characters to search...</div>`;
      return;
    }
    searchDebounceTimer = setTimeout(async () => {
      try {
        searchHits = await invoke('cmd_search_repos', { query: q });
        renderSearchResults(searchHits);
      } catch (err) {
        console.error('[devbar] search error:', err);
      }
    }, 150);
  });

  searchInput.addEventListener('keydown', (e) => {
    if (!searchHits.length) return;
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      selectedIndex = (selectedIndex + 1) % searchHits.length;
      updateSearchSelection();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      selectedIndex = (selectedIndex - 1 + searchHits.length) % searchHits.length;
      updateSearchSelection();
    } else if (e.key === 'Enter') {
      e.preventDefault();
      const hit = searchHits[selectedIndex];
      if (hit) {
        invoke('open_in_vscode', { path: hit.open_path });
        closeSearch();
      }
    }
  });
}

function renderSearchResults(hits) {
  searchResultsEl.innerHTML = '';
  selectedIndex = 0;
  if (!hits || !hits.length) {
    searchResultsEl.innerHTML = `<div class="empty">No matching files, commits, branches, or repos found</div>`;
    return;
  }

  hits.forEach((hit, idx) => {
    const row = document.createElement('div');
    row.className = `search-item ${idx === 0 ? 'selected' : ''}`;
    row.dataset.index = idx;
    row.innerHTML = `
      <div class="search-item-left">
        <span class="search-kind-badge kind-${hit.kind}">${hit.kind}</span>
        <span class="search-item-label" title="${hit.open_path}">${hit.label}</span>
      </div>
      <span class="search-item-repo">${hit.repo}</span>
    `;
    row.addEventListener('click', () => {
      invoke('open_in_vscode', { path: hit.open_path });
      closeSearch();
    });
    searchResultsEl.appendChild(row);
  });
}

function updateSearchSelection() {
  const items = searchResultsEl.querySelectorAll('.search-item');
  items.forEach((item, idx) => {
    item.classList.toggle('selected', idx === selectedIndex);
    if (idx === selectedIndex) {
      item.scrollIntoView({ block: 'nearest' });
    }
  });
}

// Global click listener for VS Code open buttons anywhere in the document
document.addEventListener('click', async (e) => {
  const btn = e.target.closest('.open-vscode-btn');
  if (!btn || btn.dataset.handled) return;
  const path = btn.dataset.path;
  if (!path) return;

  btn.dataset.handled = 'true';
  btn.disabled = true;
  btn.textContent = '…';

  try {
    await invoke('open_in_vscode', { path });
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = 'Open'; btn.disabled = false; delete btn.dataset.handled; }, 1500);
  } catch (err) {
    console.warn('[devbar] open_in_vscode failed:', err);
    btn.textContent = '!';
    btn.title = `VS Code CLI error: ${err}`;
    btn.classList.add('open-vscode-btn--error');
    setTimeout(() => {
      btn.textContent = 'Open';
      btn.disabled = false;
      btn.classList.remove('open-vscode-btn--error');
      delete btn.dataset.handled;
    }, 3000);
  }
});

// Global click listener for opening URLs in browser
document.addEventListener('click', async (e) => {
  const target = e.target.closest('.open-url-btn, .port-link');
  if (!target) return;
  const url = target.dataset.url;
  if (!url) return;

  try {
    await invoke('cmd_open_url', { url });
  } catch (err) {
    console.warn('[devbar] cmd_open_url failed:', err);
  }
});

async function refresh() {
  try {
    const data = await invoke("get_status");
    console.log("[devbar] get_status response data:", data);
    renderDisks(data.disks);
    renderRepos(data.repos);
    renderContainers(data.docker);
    renderPorts(data.ports);
    lastRefreshTime = Date.now();
    updateTimestampDisplay();
    loadRecentFiles();
    loadDeps();
    loadEnvHealth();
  } catch (err) {
    const errorMsg = `<div class="empty" style="color: #f43f5e;">Error: ${err}</div>`;
    diskListEl.innerHTML = errorMsg;
    repoListEl.innerHTML = errorMsg;
    containerListEl.innerHTML = errorMsg;
    if (portListEl) portListEl.innerHTML = errorMsg;
    lastUpdatedEl.textContent = `Error: ${err}`;
    console.error("get_status failed:", err);
  }
}

// ─── Theme System ───────────────────────────────────────────────────────────
function applyTheme(theme) {
  document.body.setAttribute('data-theme', theme);
  document.querySelectorAll('.theme-btn').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.theme === theme);
  });
}

async function initTheme() {
  try {
    const theme = await invoke('cmd_get_theme');
    applyTheme(theme);
  } catch (err) {
    console.warn('[devbar] Could not load theme:', err);
    applyTheme('dark');
  }

  document.querySelectorAll('.theme-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const theme = btn.dataset.theme;
      applyTheme(theme);
      try {
        await invoke('cmd_set_theme', { theme });
      } catch (err) {
        console.warn('[devbar] Could not save theme:', err);
      }
    });
  });
}

// ─── Autostart ──────────────────────────────────────────────────────────────
const autostartToggle = document.getElementById('autostart-toggle');

async function initAutostart() {
  if (!autostartToggle) return;
  try {
    const enabled = await invoke('plugin:autostart|is_enabled');
    autostartToggle.checked = !!enabled;
  } catch (err) {
    console.warn('Autostart status check failed:', err);
  }

  autostartToggle.addEventListener('change', async () => {
    try {
      if (autostartToggle.checked) {
        await invoke('plugin:autostart|enable');
      } else {
        await invoke('plugin:autostart|disable');
      }
    } catch (err) {
      console.error('Failed to toggle autostart:', err);
      autostartToggle.checked = !autostartToggle.checked;
    }
  });
}

// ─── Collapsible Sections ────────────────────────────────────────────────────
const COLLAPSE_KEY = 'devbar_collapsed_sections';

function initCollapsibleSections() {
  const saved = JSON.parse(localStorage.getItem(COLLAPSE_KEY) || '[]');
  document.querySelectorAll('.section-header').forEach(header => {
    const sectionId = header.dataset.section;
    const block = document.getElementById(sectionId);
    if (!block) return;

    // Restore persisted collapsed state
    if (saved.includes(sectionId)) block.classList.add('collapsed');

    header.addEventListener('click', () => {
      block.classList.toggle('collapsed');
      const nowCollapsed = [...document.querySelectorAll('.block.collapsed')]
        .map(b => b.id)
        .filter(Boolean);
      localStorage.setItem(COLLAPSE_KEY, JSON.stringify(nowCollapsed));
    });
  });
}

// ─── Editor Picker ──────────────────────────────────────────────────────────
const editorCurrentEl   = document.getElementById('editor-current');
const editorCustomForm  = document.getElementById('editor-custom-form');
const editorCliInput    = document.getElementById('editor-cli-input');
const editorPresetBtns  = document.querySelectorAll('.editor-preset-btn');

async function initEditorPicker() {
  if (!editorCurrentEl) return;
  try {
    const current = await invoke('cmd_get_editor');
    updateEditorUI(current);
  } catch (err) {
    console.error('Failed to load editor setting:', err);
  }

  editorPresetBtns.forEach(btn => {
    btn.addEventListener('click', async () => {
      const cli = btn.dataset.cli;
      if (!cli) return;
      await setEditor(cli);
    });
  });

  if (editorCustomForm) {
    editorCustomForm.addEventListener('submit', async (e) => {
      e.preventDefault();
      const val = editorCliInput.value.trim();
      if (val) {
        await setEditor(val);
        editorCliInput.value = '';
      }
    });
  }
}

async function setEditor(cli) {
  try {
    const updated = await invoke('cmd_set_editor', { editor: cli });
    updateEditorUI(updated);
  } catch (err) {
    console.error('Failed to set editor:', err);
  }
}

function updateEditorUI(cli) {
  if (editorCurrentEl) {
    editorCurrentEl.textContent = `Current: ${cli}`;
  }
  editorPresetBtns.forEach(btn => {
    if (btn.dataset.cli === cli) {
      btn.classList.add('active');
    } else {
      btn.classList.remove('active');
    }
  });
}

// ─── Boot ────────────────────────────────────────────────────────────────────
loadWatchDirs();
loadWatchedPorts();
initTheme();
initAutostart();
initCollapsibleSections();
initEditorPicker();
refresh();
setInterval(refresh, 60_000);
setInterval(updateTimestampDisplay, 1000);



// ─── Watched Ports ──────────────────────────────────────────────────────────
const watchPortsListEl = document.getElementById('watch-ports-list');
const addPortForm      = document.getElementById('add-port-form');
const portInput        = document.getElementById('port-input');

let currentWatchedPorts = [];

function renderWatchedPorts(ports) {
  currentWatchedPorts = ports;
  watchPortsListEl.innerHTML = '';
  if (!ports.length) {
    watchPortsListEl.innerHTML = `<div class="empty">No ports watched</div>`;
    return;
  }
  ports.forEach((port, index) => {
    const item = document.createElement('div');
    item.className = 'dir-item';
    item.innerHTML = `
      <span class="dir-path">:${port}</span>
      <button class="remove-dir-btn" data-port-index="${index}" title="Remove port">×</button>
    `;
    watchPortsListEl.appendChild(item);
  });
}

async function loadWatchedPorts() {
  try {
    const ports = await invoke('get_watched_ports');
    renderWatchedPorts(ports);
  } catch (err) {
    console.error('Failed to load watched ports:', err);
  }
}

async function updateWatchedPorts(newPorts) {
  try {
    const updated = await invoke('set_watched_ports', { ports: newPorts });
    renderWatchedPorts(updated);
    await refresh();
  } catch (err) {
    console.error('Failed to update watched ports:', err);
  }
}

if (watchPortsListEl) {
  watchPortsListEl.addEventListener('click', (e) => {
    if (e.target.classList.contains('remove-dir-btn')) {
      const idx = parseInt(e.target.dataset.portIndex, 10);
      if (!isNaN(idx)) {
        const next = [...currentWatchedPorts];
        next.splice(idx, 1);
        updateWatchedPorts(next);
      }
    }
  });
}

if (addPortForm) {
  addPortForm.addEventListener('submit', (e) => {
    e.preventDefault();
    const val = parseInt(portInput.value.trim(), 10);
    if (!isNaN(val) && val > 0 && val <= 65535 && !currentWatchedPorts.includes(val)) {
      updateWatchedPorts([...currentWatchedPorts, val]);
      portInput.value = '';
    }
  });
}

// ─── Window Controls ────────────────────────────────────────────────────────
const hideBtn = document.getElementById('hide-btn');
const quitBtn = document.getElementById('quit-btn');

if (hideBtn) {
  hideBtn.addEventListener('click', () => {
    invoke('cmd_hide_window');
  });
}

if (quitBtn) {
  quitBtn.addEventListener('click', () => {
    invoke('cmd_quit_app');
  });
}

const titlebarEl = document.querySelector('.titlebar');
if (titlebarEl) {
  titlebarEl.addEventListener('mousedown', (e) => {
    if (e.target.closest('button') || e.target.closest('input')) return;
    invoke('cmd_start_drag').catch(() => {});
  });
}

// ─── Open With Popover Manager ─────────────────────────────────────────────
const openMenuPopover = document.getElementById('open-menu-popover');
const popoverCloseBtn = document.getElementById('popover-close-btn');

let activePopoverPath = null;

function showOpenMenu(path, anchorBtn) {
  if (!openMenuPopover) return;
  activePopoverPath = path;
  openMenuPopover.classList.remove('hidden');

  const rect = anchorBtn.getBoundingClientRect();
  const popoverWidth = 220;
  const popoverHeight = 250;

  let left = rect.right - popoverWidth;
  if (left < 10) left = 10;

  let top = rect.bottom + 4;
  if (top + popoverHeight > window.innerHeight) {
    top = rect.top - popoverHeight - 4;
  }
  if (top < 10) top = 10;

  openMenuPopover.style.left = `${left}px`;
  openMenuPopover.style.top = `${top}px`;
}

function hideOpenMenu() {
  if (!openMenuPopover) return;
  openMenuPopover.classList.add('hidden');
  activePopoverPath = null;
}

if (popoverCloseBtn) {
  popoverCloseBtn.addEventListener('click', hideOpenMenu);
}

document.addEventListener('click', (e) => {
  const trigger = e.target.closest('.open-with-trigger');
  if (trigger) {
    const path = trigger.dataset.path;
    if (path) {
      e.stopPropagation();
      showOpenMenu(path, trigger);
    }
    return;
  }

  if (openMenuPopover && !openMenuPopover.classList.contains('hidden')) {
    if (!e.target.closest('#open-menu-popover')) {
      hideOpenMenu();
    }
  }
});

if (openMenuPopover) {
  openMenuPopover.addEventListener('click', async (e) => {
    const item = e.target.closest('.popover-item');
    if (!item || !activePopoverPath) return;

    const action = item.dataset.action;
    const cli = item.dataset.cli;
    const path = activePopoverPath;
    hideOpenMenu();

    try {
      if (action === 'editor') {
        if (cli === 'default') {
          await invoke('open_in_vscode', { path });
        } else {
          await invoke('open_with', { editor: cli, path });
        }
      } else if (action === 'explorer') {
        await invoke('open_in_explorer', { path });
      } else if (action === 'terminal') {
        await invoke('open_in_terminal', { path });
      }
    } catch (err) {
      console.warn('[devbar] popover open action failed:', err);
    }
  });
}

// ─── Security & .env Health ──────────────────────────────────────────────────
const envListEl = document.getElementById('env-list');

async function loadEnvHealth() {
  if (!envListEl) return;
  try {
    const issues = await invoke('cmd_get_env_health');
    renderEnvHealth(issues);
  } catch (err) {
    console.error('[devbar] loadEnvHealth error:', err);
    envListEl.innerHTML = `<div class="empty">Could not scan .env health</div>`;
  }
}

function renderEnvHealth(issues) {
  if (!envListEl) return;
  envListEl.innerHTML = '';

  if (!issues || !issues.length) {
    envListEl.innerHTML = `<div class="empty" style="color: #34d399;">🟢 All repos have healthy .env setup & no unignored secrets</div>`;
    return;
  }

  issues.forEach(issue => {
    const item = document.createElement('div');
    item.className = 'env-issue-item';

    const isMissing = issue.issue_type === 'missing_env';
    const badgeClass = isMissing ? 'badge-missing-env' : 'badge-unignored-secret';
    const badgeText = isMissing ? 'Missing .env' : 'Unignored Secret';
    const btnText = isMissing ? `Copy ${issue.example_file}` : `Ignore ${issue.file_name}`;

    item.innerHTML = `
      <div class="env-issue-left">
        <span class="env-issue-badge ${badgeClass}">${badgeText}</span>
        <div class="env-issue-info">
          <span class="env-issue-repo">${issue.repo_name}</span>
          <span class="env-issue-msg">${issue.message}</span>
        </div>
      </div>
      <button class="btn-quick-fix" data-type="${issue.issue_type}" data-repo="${issue.repo_path}" data-file="${issue.file_name || ''}" data-example="${issue.example_file || ''}">
        ${btnText}
      </button>
    `;

    const fixBtn = item.querySelector('.btn-quick-fix');
    fixBtn.addEventListener('click', async () => {
      fixBtn.disabled = true;
      fixBtn.textContent = 'Fixing…';
      try {
        if (isMissing) {
          await invoke('cmd_fix_missing_env', { repoPath: issue.repo_path, exampleFilename: issue.example_file });
        } else {
          await invoke('cmd_add_to_gitignore', { repoPath: issue.repo_path, fileToIgnore: issue.file_name });
        }
        await refresh();
      } catch (err) {
        console.error('[devbar] env fix error:', err);
        fixBtn.textContent = 'Failed';
        setTimeout(() => { fixBtn.disabled = false; fixBtn.textContent = btnText; }, 2000);
      }
    });

    envListEl.appendChild(item);
  });
}

// ─── Script Runner Modal Manager ─────────────────────────────────────────────
const scriptModal          = document.getElementById('script-modal');
const scriptModalTitle     = document.getElementById('script-modal-title');
const scriptLogBody        = document.getElementById('script-log-body');
const scriptModalClose     = document.getElementById('script-modal-close');
const scriptModalDone      = document.getElementById('script-modal-done');

const quickActionsPopover  = document.getElementById('quick-actions-popover');
const quickActionsTitle    = document.getElementById('quick-actions-title');
const quickActionsList     = document.getElementById('quick-actions-list');
const quickActionsCloseBtn = document.getElementById('quick-actions-close-btn');

function showScriptModal(title, logContent) {
  if (!scriptModal) return;
  scriptModalTitle.textContent = title;
  scriptLogBody.textContent = logContent;
  scriptModal.classList.remove('hidden');
}

function hideScriptModal() {
  if (!scriptModal) return;
  scriptModal.classList.add('hidden');
}

if (scriptModalClose) scriptModalClose.addEventListener('click', hideScriptModal);
if (scriptModalDone)  scriptModalDone.addEventListener('click', hideScriptModal);

function hideQuickActionsMenu() {
  if (quickActionsPopover) quickActionsPopover.classList.add('hidden');
}

if (quickActionsCloseBtn) {
  quickActionsCloseBtn.addEventListener('click', hideQuickActionsMenu);
}

document.addEventListener('click', async (e) => {
  const runnerBtn = e.target.closest('.btn-script-runner');
  if (!runnerBtn) {
    if (quickActionsPopover && !quickActionsPopover.classList.contains('hidden')) {
      if (!e.target.closest('#quick-actions-popover')) {
        hideQuickActionsMenu();
      }
    }
    return;
  }

  const repoPath = runnerBtn.dataset.path;
  const repoName = runnerBtn.dataset.name;
  if (!repoPath) return;

  e.stopPropagation();
  hideOpenMenu();

  try {
    const scripts = await invoke('cmd_get_repo_scripts', { repoPath });
    if (!scripts || !scripts.length) {
      showScriptModal(`⚡ Quick Actions (${repoName})`, `No scripts found for ${repoName}`);
      return;
    }

    showScriptSelectionMenu(repoPath, repoName, scripts, runnerBtn);
  } catch (err) {
    console.error('[devbar] get_repo_scripts error:', err);
  }
});

function showScriptSelectionMenu(repoPath, repoName, scripts, anchorBtn) {
  if (!quickActionsPopover || !quickActionsList) return;

  quickActionsPopover.classList.remove('hidden');
  if (quickActionsTitle) quickActionsTitle.textContent = `⚡ Actions (${repoName})`;

  quickActionsList.innerHTML = '';

  scripts.forEach(s => {
    const btn = document.createElement('button');
    btn.className = 'popover-item';
    btn.innerHTML = `
      <span class="popover-icon">${s.is_interactive ? '🚀' : '▶'}</span>
      <span style="font-weight: 500;">${s.name}</span>
      <span class="meta" style="margin-left:auto; font-size:10px;">${s.category}</span>
    `;

    btn.addEventListener('click', async () => {
      hideQuickActionsMenu();
      const actionMsg = s.is_interactive
        ? `Launching "${s.command} ${s.args.join(' ')}" in terminal window for ${repoName}...`
        : `Running "${s.command} ${s.args.join(' ')}" in ${repoName}...\n\nPlease wait up to 15s...`;

      showScriptModal(`⚡ ${s.name}`, actionMsg);

      try {
        const output = await invoke('cmd_run_repo_script', {
          repoPath,
          command: s.command,
          args: s.args
        });
        showScriptModal(`✅ Finished: ${s.name}`, output);
      } catch (err) {
        showScriptModal(`❌ Error: ${s.name}`, err);
      }
    });

    quickActionsList.appendChild(btn);
  });

  const rect = anchorBtn.getBoundingClientRect();
  const popoverWidth = 250;
  let left = rect.right - popoverWidth;
  if (left < 10) left = 10;
  if (left + popoverWidth > window.innerWidth - 10) {
    left = window.innerWidth - popoverWidth - 10;
  }

  let top = rect.bottom + 6;
  if (top + 220 > window.innerHeight) {
    top = rect.top - 220 - 6;
  }
  if (top < 10) top = 10;

  quickActionsPopover.style.left = `${left}px`;
  quickActionsPopover.style.top = `${top}px`;
}


