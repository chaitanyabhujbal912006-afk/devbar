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

// ─── Boot ────────────────────────────────────────────────────────────────────
loadWatchDirs();
initTheme();
initAutostart();
refresh();
setInterval(refresh, 60_000);
setInterval(updateTimestampDisplay, 1000);
