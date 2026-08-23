const diskListEl = document.getElementById("disk-list");
const repoListEl = document.getElementById("repo-list");
const containerListEl = document.getElementById("container-list");
const portListEl = document.getElementById("port-list");
const lastUpdatedEl = document.getElementById("last-updated");

const refreshBtn = document.getElementById("refresh-btn");

const settingsBtn = document.getElementById("settings-btn");
const settingsPanel = document.getElementById("settings-panel");
const watchDirsListEl = document.getElementById("watch-dirs-list");
const addDirForm = document.getElementById("add-dir-form");
const dirInput = document.getElementById("dir-input");

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
    row.innerHTML = `
      <span><span class="dot ${status}"></span>${r.name || "Unknown"} <span class="meta">(${r.branch || "main"})</span></span>
      <span class="meta">${r.changed_files ?? 0} changed · ${r.unpushed_commits ?? 0} unpushed</span>
    `;
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

settingsBtn.addEventListener("click", () => {
  const isHidden = settingsPanel.classList.contains("hidden");
  if (isHidden) {
    settingsPanel.classList.remove("hidden");
    settingsBtn.classList.add("active");
  } else {
    settingsPanel.classList.add("hidden");
    settingsBtn.classList.remove("active");
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
    portListEl.innerHTML = `<div class="empty">No active listening ports</div>`;
    return;
  }
  for (const p of ports) {
    const row = document.createElement("div");
    row.className = "item";
    row.title = `PID: ${p.pid} — ${p.address}`;
    row.innerHTML = `
      <span><span class="dot ok"></span><strong style="color: #38bdf8;">:${p.port}</strong> <span class="meta">(${p.process_name})</span></span>
      <span class="meta">PID ${p.pid}</span>
    `;
    portListEl.appendChild(row);
  }
}

if (refreshBtn) {
  refreshBtn.addEventListener("click", refresh);
}

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




const autostartToggle = document.getElementById("autostart-toggle");

async function initAutostart() {
  if (!autostartToggle) return;
  try {
    const enabled = await invoke("plugin:autostart|is_enabled");
    autostartToggle.checked = !!enabled;
  } catch (err) {
    console.warn("Autostart status check failed:", err);
  }

  autostartToggle.addEventListener("change", async () => {
    try {
      if (autostartToggle.checked) {
        await invoke("plugin:autostart|enable");
      } else {
        await invoke("plugin:autostart|disable");
      }
    } catch (err) {
      console.error("Failed to toggle autostart:", err);
      autostartToggle.checked = !autostartToggle.checked;
    }
  });
}

// Initial load + auto-refresh every 60s + 1s relative timestamp update
loadWatchDirs();
initAutostart();
refresh();
setInterval(refresh, 60_000);
setInterval(updateTimestampDisplay, 1000);
