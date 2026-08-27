# Changelog

All notable changes to DevBar will be documented in this file.

## [0.3.0] - 2026-08-27

### Added
- **Multi-Language Dependency Matrix** (`src-tauri/src/monitors/deps.rs`): Extended cross-repository dependency tracking to parse Rust (`Cargo.toml`), Python (`pyproject.toml`, `requirements.txt`), and Go (`go.mod`) manifests with language badge indicators.
- **Configurable Watched Ports** (`src-tauri/src/monitors/ports.rs`): Added customizable ports list persisted to `watched_ports.json` and editable from the Settings panel.
- **Interactive Process Kill Action**: One-click kill button on in-use ports panel using platform-native `taskkill /F /PID` (Windows) and `kill -9` (Unix) with UI status feedback.
- **Editor Launcher Picker**: Added setting to choose default project launcher (VS Code, Cursor, Zed, or custom CLI) persisted to `editor.json`.
- **Collapsible Monitor Panels**: Interactive toggle headers on all monitor sections with smooth CSS transition animations and `localStorage` section state persistence.
- **Cross-Platform Port Engine**: Implemented `lsof` (macOS) and `ss` (Linux) command parsers alongside Windows `netstat`.

### Performance & Refactoring
- **Concurrent Monitor Pipeline**: Converted `get_status` Tauri handler to execute Disk, Git, Docker, and Ports checks concurrently using `std::thread::scope`, overlapping execution and eliminating blocking monitor serialization.
- **Consolidated Repository Walker**: Created `monitors/common.rs` to host a unified `collect_repo_paths` function, eliminating 4 duplicate recursive directory walking implementations and reducing disk traversal overhead.
- **Debug Log Cleanup**: Removed redundant stdout logging during repository walks for faster execution and cleaner console logs.

## [0.2.0] - 2026-08-25

### Added
- **Global Search Engine (Ctrl+K)** (`src-tauri/src/monitors/search.rs`): Cross-repository full-text search engine matching files, git commit messages, branch names, and repo names with fuzzy overlay UI and keyboard navigation.
- **Resume Work Monitor** (`src-tauri/src/monitors/recent.rs`): Quick-resume panel listing recently committed and working-tree modified files across all watched repositories, sorted by modification timestamp.
- **Cross-Repo Dependencies Matrix** (`src-tauri/src/monitors/deps.rs`): Automatic parsing of `package.json` across repositories to monitor core package versions (React, Next.js, Vite, TypeScript, Tailwind, etc.) and flag version mismatches.
- **Native VS Code Integration** (`src-tauri/src/main.rs`): One-click launch of project directories and files in VS Code via native CLI and platform fallbacks.
- **Multi-Theme Engine**: Persisted theme switcher supporting Dark, Light, and Midnight color palettes via CSS variables and `theme.json` configuration.
- **Active Ports Monitor** (`src-tauri/src/monitors/ports.rs`): Live tracking of common development TCP ports (3000, 5173, 8000, 8080, etc.) and their owning processes.
- **Thread Safety & Panic Resilience**: Background monitoring loop protected by `panic::catch_unwind` and explicit atomic shutdown handles on application exit.
- **Comprehensive Unit Tests**: Full Rust unit test suite covering git, disk, docker, ports, search, deps, and recent monitors.

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
