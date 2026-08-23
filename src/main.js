import { invoke } from "@tauri-apps/api/core";

const diskListEl = document.getElementById("disk-list");
const repoListEl = document.getElementById("repo-list");
const containerListEl = document.getElementById("container-list");
const lastUpdatedEl = document.getElementById("last-updated");
const refreshBtn = document.getElementById("refresh-btn");

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
  if (!repos.length) {
    repoListEl.innerHTML = `<div class="empty">No git repos found in watched folders</div>`;
    return;
  }
  for (const r of repos) {
    const status = r.dirty ? "warn" : "ok";
    const row = document.createElement("div");
    row.className = "item";
    row.title = r.path;
    row.innerHTML = `
      <span><span class="dot ${status}"></span>${r.name} <span class="meta">(${r.branch})</span></span>
      <span class="meta">${r.changed_files} changed · ${r.unpushed_commits} unpushed</span>
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

async function refresh() {
  try {
    const data = await invoke("get_status");
    renderDisks(data.disks);
    renderRepos(data.repos);
    renderContainers(data.docker);
    lastUpdatedEl.textContent = `Updated ${new Date().toLocaleTimeString()}`;
  } catch (err) {
    lastUpdatedEl.textContent = `Error: ${err}`;
    console.error(err);
  }
}

refreshBtn.addEventListener("click", refresh);

// Initial load + auto-refresh every 60s
refresh();
setInterval(refresh, 60_000);
