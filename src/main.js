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
  console.log("[devbar] renderRepos called with:", repos);
  repoListEl.innerHTML = "";
  if (!repos || !Array.isArray(repos) || !repos.length) {
    repoListEl.innerHTML = `<div class="empty">No git repos found in watched folders</div>`;
    return;
  }
  for (const r of repos) {
    const status = r.dirty ? "warn" : "ok";
    const row = document.createElement("div");
    row.className = "item";
    row.title = r.path || "";

    const infoSpan = document.createElement("span");
    infoSpan.innerHTML = `<span class="dot ${status}"></span>${r.name || "Unknown"} <span class="meta">(${r.branch || "main"})</span>`;

    const rightSpan = document.createElement("span");
    rightSpan.style.display = "flex";
    rightSpan.style.alignItems = "center";
    rightSpan.style.gap = "8px";

    const metaSpan = document.createElement("span");
    metaSpan.className = "meta";
    metaSpan.textContent = `${r.changed_files ?? 0} changed · ${r.unpushed_commits ?? 0} unpushed`;

    const openBtn = document.createElement("button");
    openBtn.className = "open-vscode-btn";
    openBtn.title = `Open in VS Code: ${r.path}`;
    openBtn.textContent = "Open";
    openBtn.dataset.path = r.path || "";

    rightSpan.appendChild(metaSpan);
    rightSpan.appendChild(openBtn);
    row.appendChild(infoSpan);
    row.appendChild(rightSpan);
    repoListEl.appendChild(row);
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
      row.title = `PID: ${p.pid ?? "?"} — ${p.process_name ?? "Unknown"}`;
    }
    const procLabel = p.in_use
      ? ` <span class="meta">(${p.process_name ?? "Unknown"})</span>`
      : "";
    const rightLabel = p.in_use
      ? `<span class="meta">PID ${p.pid}</span>`
      : `<span class="meta port-free">free</span>`;
    row.innerHTML = `
      <span><span class="dot ${dotClass}"></span><strong class="port-num">:${p.port}</strong>${procLabel}</span>
      ${rightLabel}
    `;
    portListEl.appendChild(row);
  }
}

if (refreshBtn) {
  refreshBtn.addEventListener("click", refresh);
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
        <button class="open-vscode-btn" data-path="${f.absolute_path}" title="Open ${f.relative_path} in VS Code">Open</button>
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

// ─── Boot ────────────────────────────────────────────────────────────────────
loadWatchDirs();
loadWatchedPorts();
initTheme();
initAutostart();
initCollapsibleSections();
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
