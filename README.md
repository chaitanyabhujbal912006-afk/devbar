# DevBar 🖥️

> A lightweight system tray app that keeps your dev environment health visible at a glance — no browser, no login, no fuss.

Built with **Rust + Tauri v2** (native performance, tiny footprint).

---

## What it does

DevBar sits quietly in your system tray and checks three things every 60 seconds:

| Monitor | What it tracks |
|---------|---------------|
| 💾 **Disk Space** | Free/used space on every mounted drive with green/yellow/red thresholds |
| 🌿 **Git Repos** | Dirty files and unpushed commits across your project folders |
| 🐳 **Docker Containers** | Running, exited, and restarting containers (gracefully skipped if Docker isn't installed) |

Click the tray icon → a compact dark popup appears with live status. Click again to hide it.

---

## Screenshots

> _Coming soon_

---

## Prerequisites

Before you can run DevBar, make sure you have:

- **Node.js** 18 or later — [nodejs.org](https://nodejs.org)
- **Rust** (via rustup) — [rustup.rs](https://rustup.rs)
- **Platform build tools:**

| OS | Required |
|----|----------|
| Windows | [Visual Studio C++ Build Tools](https://aka.ms/vs/17/release/vs_BuildTools.exe) + WebView2 (pre-installed on Win 10/11) |
| macOS | Xcode Command Line Tools (`xcode-select --install`) |
| Linux | `libwebkit2gtk-4.1-dev build-essential libssl-dev libayatana-appindicator3-dev` |

> **Windows tip:** After installing Rust, add `C:\Users\<you>\.cargo\bin` to your PATH and add the Rust `target\` folder to Windows Defender exclusions to avoid file-locking during builds.

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

## How it works

1. **Rust backend** runs the monitors and exposes a single `get_status` command
2. **Frontend JS** calls `invoke("get_status")` via Tauri's IPC bridge every 60 seconds
3. Results are rendered as coloured dots in the popup:
   - 🟢 **OK** — everything is fine
   - 🟡 **Warn** — worth keeping an eye on (disk > 75%, dirty repo, restarting container)
   - 🔴 **Critical** — needs attention (disk > 90%, exited container)

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

| Layer | Technology |
|-------|-----------|
| Backend | Rust 1.98, Tauri v2 |
| System info | `sysinfo` crate |
| Repo scanning | `walkdir` + `git` CLI |
| Container info | `docker` CLI |
| Frontend | Vanilla HTML, CSS, JS |
| IPC | Tauri `invoke` API |

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
