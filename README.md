# DevBar — Menu-bar Dev Environment Health Monitor

A tiny cross-platform (macOS/Windows/Linux) tray app built with **Tauri (Rust + HTML/JS)**.

It quietly checks, every 60 seconds:
- 💾 Disk space on all mounted drives (green/yellow/red thresholds)
- 🌿 Git status across your project folders (dirty files, unpushed commits)

Click the tray icon to see a popup with live status. No login, no server — everything runs locally.

## What's included (v1 MVP)
- Tray icon + toggleable popup window (`src-tauri/src/main.rs`)
- Disk monitor (`src-tauri/src/monitors/disk.rs`)
- Git repo scanner (`src-tauri/src/monitors/git.rs`)
- Simple dark-themed popup UI (`src/index.html`, `main.js`, `styles.css`)

## Prerequisites
- Node.js 18+
- Rust (via https://rustup.rs)
- Platform build tools:
  - **macOS**: Xcode Command Line Tools
  - **Windows**: Microsoft Visual Studio C++ Build Tools, WebView2 (usually preinstalled on Win 10/11)
  - **Linux**: `libwebkit2gtk-4.1-dev`, `build-essential`, `libssl-dev`, `libayatana-appindicator3-dev`

## Run it
```bash
npm install
npm run dev
```

## Build a distributable
```bash
npm run build
```

## Project structure
```
devbar/
├── src-tauri/            # Rust backend
│   ├── src/
│   │   ├── main.rs       # tray icon, window, commands
│   │   └── monitors/
│   │       ├── disk.rs
│   │       └── git.rs
│   ├── icons/icon.png
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/                  # Frontend (popup UI)
│   ├── index.html
│   ├── main.js
│   └── styles.css
└── package.json
```

## Next steps
See `PROMPTS.md` for a copy-paste sequence of prompts to feed into Antigravity
to extend this into the full product (Docker monitor, settings screen, auto-launch,
notifications, packaging, etc).
