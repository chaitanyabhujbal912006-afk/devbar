# DevBar 🖥️

> A lightweight system tray app that keeps your developer environment healthy — without opening a browser, logging in, or running complicated commands.

Built with **Rust + Tauri v2** — native speed, tiny memory footprint, runs on Windows, macOS, and Linux.

---

## What problem does it solve?

As a developer you constantly juggle multiple things:

- Is my disk about to fill up?
- Did I forget to commit changes in one of my projects?
- Are my Docker containers actually running?

Normally you'd open a terminal and run `df -h`, `git status`, and `docker ps` in each folder one by one. DevBar does all of that automatically and puts the results one click away in your taskbar.

---

## What it shows you

Click the tray icon and a compact popup appears with live, collapsible monitor panels:

| Section | What it tracks |
|---------|---------------|
| 📝 **Resume Work** | Recent file edits and uncommitted files across all repos for instant context recovery |
| 💾 **Disk Space** | Used/free space on every drive. 🟢 fine · 🟡 getting full (>75%) · 🔴 urgent (>90%) |
| 🌿 **Git Repos** | Scans your project folders — shows uncommitted files and unpushed commits |
| 🛡️ **Security & .env** | Detects missing `.env` files and un-ignored secret files (`.env`, `.pem`, `id_rsa`, `credentials.json`) with 1-click quick-fix buttons |
| ⚡ **Quick Actions** | Auto-detects and runs repo scripts (`npm run dev`, `cargo run`, `docker compose up`, `git pull`) directly from the tray with live output drawer |
| 🔌 **Active Ports** | Monitors configured dev ports (3000, 5173, 8000, etc.) with process names, PIDs, and one-click process termination (`taskkill`/`kill`) |
| 🐳 **Containers** | Lists Docker containers and state (running, stopped, restarting). Graceful fallback if Docker is stopped |
| 📦 **Dependencies** | Cross-repo package version matrix supporting JS/TS (`package.json`), Rust (`Cargo.toml`), Python (`pyproject.toml`, `requirements.txt`), and Go (`go.mod`) |
| 🔍 **Global Search (Ctrl+K)** | Instant fuzzy search across repositories for files, commits, branches, and repo names |

Click the icon again to hide the popup. All monitor tasks execute concurrently in parallel threads every **60 seconds**.

---

## Screenshots

> _Coming soon_

---

## Prerequisites

You need two tools installed before you can build or run DevBar:

**Node.js 18+** — download from [nodejs.org](https://nodejs.org)

**Rust** — install via one command from [rustup.rs](https://rustup.rs), then restart your terminal

**Platform build tools** (one-time setup):

| OS | What to install |
|----|-----------------|
| Windows | [Visual Studio C++ Build Tools](https://aka.ms/vs/17/release/vs_BuildTools.exe) — tick "Desktop development with C++". WebView2 is pre-installed on Win 10/11. |
| macOS | Run `xcode-select --install` in a terminal |
| Linux | `sudo apt install build-essential libwebkit2gtk-4.1-dev libssl-dev libayatana-appindicator3-dev` |

> **Windows tip:** After installing Rust, add `C:\Users\<you>\.cargo\bin` to your PATH environment variable. Also add the project's `target\` folder to Windows Defender exclusions — this stops Defender from locking `.exe` files mid-build.

---

## Getting started

```bash
# 1. Install JS dependencies
npm install

# 2. Start the app in development mode
npm run dev
```

The first build downloads ~400 Rust crates and takes **5–15 minutes**. Subsequent builds are fast (under 30 seconds).

---

## Project structure

```
devbar/
├── src/                        # Frontend (plain HTML/JS/CSS — no framework)
│   ├── index.html              # Popup UI
│   ├── main.js                 # Calls Rust commands, renders status
│   └── styles.css              # Dark theme styles
│
└── src-tauri/                  # Rust backend
    ├── src/
    │   ├── main.rs             # App setup: tray icon, window, Tauri commands
    │   └── monitors/
    │       ├── disk.rs         # Disk space monitor (via sysinfo)
    │       ├── git.rs          # Git repo scanner (shells out to git)
    │       └── docker.rs       # Docker container monitor (shells out to docker)
    ├── icons/                  # App icons
    ├── Cargo.toml              # Rust dependencies
    └── tauri.conf.json         # Window size, tray config, security settings
```

---

## How it works (in plain English)

1. **Rust backend** — the "engine". Runs your disk, git, and Docker checks and returns the results.
2. **Tauri bridge** — connects the Rust engine to the UI using a secure, typed interface (no HTTP server, no open ports).
3. **Frontend (HTML/JS/CSS)** — the popup window you see. It asks the engine for fresh data every 60 seconds and displays it with coloured status dots:
   - 🟢 **OK** — everything is fine
   - 🟡 **Warn** — worth keeping an eye on (disk >75%, dirty repo, restarting container)
   - 🔴 **Critical** — needs attention (disk >90%, exited container)

No external servers. No accounts. Everything runs locally on your machine.

---

## Customising watched folders

By default, DevBar scans `~/dev`, `~/projects`, and `~/code` for Git repos. You can change this by editing the default directories in [`src-tauri/src/main.rs`](src-tauri/src/main.rs):

```rust
watch_dirs: Mutex::new(vec![
    shellexpand::tilde("~/dev").to_string(),
    shellexpand::tilde("~/projects").to_string(),
    shellexpand::tilde("~/code").to_string(),
]),
```

---

## Tech stack

| Layer | Technology | Why |
|-------|-----------|-----|
| Desktop shell | [Tauri v2](https://tauri.app) | Rust-powered, ~5 MB bundle size vs ~200 MB for Electron |
| Backend language | Rust 1.98 | Memory-safe, fast, no garbage-collector pauses |
| Disk info | `sysinfo` crate | Cross-platform disk stats with one API call |
| Repo scanning | `walkdir` + `git` CLI | Reads real git state without a heavy library |
| Container info | `docker` CLI | No Docker SDK needed — just parses its JSON output |
| Frontend | Vanilla HTML/CSS/JS | Zero build step, tiny footprint, easy to modify |

---

## Building for production

Run the production build command:

```bash
npm run build
```

This compiles optimized binaries and bundles installers in `src-tauri/target/release/bundle/` (or `~/.cargo/devbar-target/release/bundle/`).

### Platform-specific Build Artifacts & Prerequisites

#### 🪟 Windows (`.msi`, `.exe` NSIS installer)
- **Output artifacts:**
  - `bundle/msi/DevBar_0.1.0_x64_en-US.msi`
  - `bundle/nsis/DevBar_0.1.0_x64-setup.exe`
- **Prerequisites:**
  - [Visual Studio C++ Build Tools](https://aka.ms/vs/17/release/vs_BuildTools.exe) (Desktop development with C++)
  - WebView2 (built-in on Windows 10/11)
  - *Note:* WiX 3.14 and NSIS 3.11 are automatically fetched by Tauri bundler during `npm run build`.

#### 🍎 macOS (`.dmg`, `.app`)
- **Output artifacts:**
  - `bundle/dmg/DevBar_0.1.0_x64.dmg` (or `aarch64.dmg` for Apple Silicon)
  - `bundle/macos/DevBar.app`
- **Prerequisites:**
  - Xcode Command Line Tools: `xcode-select --install`
  - macOS 10.15 or later

#### 🐧 Linux (`.AppImage`, `.deb`)
- **Output artifacts:**
  - `bundle/appimage/devbar_0.1.0_amd64.AppImage`
  - `bundle/deb/devbar_0.1.0_amd64.deb`
- **Prerequisites:**
  ```bash
  sudo apt update
  sudo apt install -y build-essential curl wget libssl-dev libgtk-3-dev \
    libayatana-appindicator3-dev librsvg2-dev libwebkit2gtk-4.1-dev
  ```

---

## License

MIT — do whatever you want with it.
