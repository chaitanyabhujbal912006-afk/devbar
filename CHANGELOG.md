# Changelog

All notable changes to DevBar will be documented in this file.

## [0.1.0] - 2026-08-24

### Added
- **Disk Space Monitor** (`src-tauri/src/monitors/disk.rs`): Real-time disk usage metrics for all mounted drives with color-coded status thresholds (OK <75%, Warning >=75%, Critical >=90%).
- **Git Repository Monitor** (`src-tauri/src/monitors/git.rs`): Automatic scanning of watched directories for Git projects, reporting dirty state, changed files count, and unpushed commits.
- **Docker Container Health Monitor** (`src-tauri/src/monitors/docker.rs`): Live container status tracking via `docker ps` with fallback handling when Docker is uninstalled or daemon is stopped.
- **Settings Screen** (`src/index.html`, `src/main.js`): Gear button toggles settings panel to add/remove custom watched folder paths with persistent disk storage in `watch_dirs.json`.
- **Dynamic System Tray Icon** (`src-tauri/src/main.rs`): Automatic tray icon color swapping (green normal / red warning) whenever a disk is critical or git repo changed files > 20.
- **Native OS Notifications** (`tauri-plugin-notification`): Real-time native desktop notifications when threshold limits cross for disk space or git status, with deduplication to prevent notification spam.
- **Launch at Login Toggle** (`tauri-plugin-autostart`): Autostart toggle in settings panel to configure DevBar to start on OS boot.
- **UI & UX Polish**:
  - Popup window auto-hides when losing focus (blur).
  - Live-updating relative timestamp ("Updated 5s ago") refreshed every second.
  - Loading states for all status blocks before data is fetched.
