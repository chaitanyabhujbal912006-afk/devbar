# Prompts for Antigravity — build DevBar end-to-end

Paste this whole project folder into Antigravity first (or open it as the workspace).
Then run these prompts **one at a time, in order**. Don't paste them all at once —
each one builds on the last, and Antigravity works best when it can verify each
step (compile/run) before moving to the next.

---

## Prompt 0 — Orient & verify the starter compiles
```
This is a Tauri v2 (Rust + JS) menu-bar app called DevBar. It has a tray icon,
a popup window, and two monitors: disk space (src-tauri/src/monitors/disk.rs)
and git repo status (src-tauri/src/monitors/git.rs).

First, install dependencies and try to run it with `npm install` and `npm run dev`.
Fix any compile errors or missing dependency versions in Cargo.toml or package.json
you encounter along the way — Tauri v2's exact API surface may have moved since
this was written, so check against the currently installed tauri crate's docs/
generated types if something doesn't match. Don't change the architecture, just
get it running. Tell me what you had to fix.
```

## Prompt 1 — Add a Docker monitor
```
Add a third monitor: Docker container health. Create
src-tauri/src/monitors/docker.rs that shells out to `docker ps -a --format json`
(handle the case where Docker isn't installed or the daemon isn't running —
return an empty list with a "docker_available: false" flag instead of crashing).
Report each container's name, status (running/exited/restarting), and image.
Wire it into the `get_status` command in main.rs alongside disks and repos, and
render it in the popup UI (src/index.html + main.js + styles.css) as a new
"Containers" section, following the same visual style as the existing sections.
```

## Prompt 2 — Settings screen for watched folders
```
Right now the folders scanned for git repos are hardcoded in main.rs
(~/dev, ~/projects, ~/code). Add a simple settings view:
1. A gear icon in the popup header that toggles a settings panel
2. A text list where the user can add/remove folder paths to watch
3. Persist this list to disk (use tauri-plugin-store, or a simple JSON file in
   the app's config dir via Rust's `dirs` crate) so it survives restarts
4. Call the existing `set_watch_dirs` command when the list changes, and load
   the saved list on startup instead of the hardcoded default
Keep the UI minimal and consistent with the existing dark theme.
```

## Prompt 3 — Menu bar icon reflects status at a glance
```
Right now the tray icon is static. Make it dynamic: when any disk is "critical"
or any git repo has more than 20 changed files (configurable threshold later),
switch the tray icon to a red/warning variant so the user notices without
opening the popup. Use a second icon asset (generate a simple colored dot PNG
similar to icons/icon.png, just in red) and swap it via the tray icon's
`set_icon` method whenever the background refresh loop runs. Also add a
background refresh loop in Rust itself (not just the frontend's setInterval)
using tauri's async runtime, so the icon updates even while the popup is closed.
```

## Prompt 4 — Native notifications on threshold cross
```
Add tauri-plugin-notification. When a disk crosses into "critical" status, or
a git repo's changed_files count crosses above 20, or unpushed_commits crosses
above 5 — fire a native OS notification once (not every refresh cycle — track
previous state to avoid spamming the same notification repeatedly). Let the
user click the notification to open the popup window.
```

## Prompt 5 — Auto-launch at login
```
Add tauri-plugin-autostart so DevBar can optionally launch when the user logs in.
Add a toggle for this in the settings panel from Prompt 2, defaulting to off.
```

## Prompt 6 — Polish pass
```
Do a polish pass across the whole app:
1. Add empty/loading states everywhere data might not have loaded yet
2. Make sure the popup window closes when it loses focus (blur), not just via
   the tray click toggle
3. Add a small "last refreshed X seconds ago" live-updating timestamp instead
   of a static one
4. Review all error handling in the Rust monitors — nothing should ever crash
   the app if a folder doesn't exist, git isn't installed, or docker isn't running
5. Add a CHANGELOG.md summarizing what's been built so far
```

## Prompt 7 — Package for distribution
```
Set up `npm run build` to produce a signed-or-unsigned installable for the
current platform (.dmg for Mac, .msi/.exe for Windows, .AppImage/.deb for Linux
depending on what we're building on). Document the exact steps for all three
platforms in README.md, including any platform-specific system dependencies
someone would need before running the build.
```

---

### Tips while using these prompts
- If Antigravity gets stuck on a Tauri API mismatch, tell it to check the
  installed `@tauri-apps/api` and `tauri` crate versions and adjust syntax —
  Tauri v2's plugin APIs changed a few times during its betas.
- Test after every single prompt (`npm run dev`) before moving to the next one.
- Feel free to reorder Prompts 2–5 based on what you care about most; only
  Prompt 0 and Prompt 1 have a strict dependency on being done first.
