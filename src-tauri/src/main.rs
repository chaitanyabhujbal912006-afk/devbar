// Prevents an extra console window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod monitors;

use monitors::deps::get_dep_versions;
use monitors::disk::get_disk_status;
use monitors::docker::get_docker_status;
use monitors::git::scan_git_repos;
use monitors::ports::get_port_status;
use monitors::recent::get_recent_files;
use monitors::search::search_repos;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};
use tauri_plugin_notification::NotificationExt;

fn get_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("devbar");
    let _ = fs::create_dir_all(&path);
    path.push("watch_dirs.json");
    Some(path)
}

fn get_theme_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("devbar");
    let _ = fs::create_dir_all(&path);
    path.push("theme.json");
    Some(path)
}

fn load_theme() -> String {
    get_theme_path()
        .and_then(|p| fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str::<String>(&s).ok())
        .unwrap_or_else(|| "dark".to_string())
}

fn save_theme(theme: &str) {
    if let Some(path) = get_theme_path() {
        if let Ok(json) = serde_json::to_string(theme) {
            let _ = fs::write(path, json);
        }
    }
}

fn resolve_path(p: &str) -> String {
    let expanded = if p.starts_with("~/") || p == "~" {
        if let Some(home) = dirs::home_dir() {
            let relative = p.trim_start_matches("~/").trim_start_matches('~');
            home.join(relative)
        } else {
            PathBuf::from(p)
        }
    } else {
        PathBuf::from(p)
    };

    let abs_path = if expanded.is_relative() {
        if let Ok(cwd) = std::env::current_dir() {
            cwd.join(expanded)
        } else {
            expanded
        }
    } else {
        expanded
    };

    abs_path.to_string_lossy().to_string()
}


fn load_watch_dirs() -> Vec<String> {
    let mut default_dirs = Vec::new();

    // Always include parent directory of current working directory if it exists (e.g. C:\projects)
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(parent) = cwd.parent() {
            let parent_str = parent.to_string_lossy().to_string();
            if !parent_str.is_empty() && Path::new(&parent_str).exists() {
                default_dirs.push(parent_str);
            }
        }
    }

    // Add home directory defaults (~/dev, ~/projects, ~/code) using dirs::home_dir()
    if let Some(home) = dirs::home_dir() {
        for sub in &["dev", "projects", "code"] {
            let p = home.join(sub).to_string_lossy().to_string();
            if Path::new(&p).exists() {
                default_dirs.push(p);
            }
        }
    }

    let mut loaded_dirs = Vec::new();
    if let Some(path) = get_config_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(dirs) = serde_json::from_str::<Vec<String>>(&content) {
                    for d in dirs {
                        let resolved = resolve_path(&d);
                        if Path::new(&resolved).exists() {
                            loaded_dirs.push(resolved);
                        }
                    }
                }
            }
        }
    }

    let mut final_dirs = Vec::new();
    for def in default_dirs {
        if !final_dirs.contains(&def) {
            final_dirs.push(def);
        }
    }
    for loaded in loaded_dirs {
        if !final_dirs.contains(&loaded) {
            final_dirs.push(loaded);
        }
    }

    let mut seen = HashSet::new();
    final_dirs.retain(|d| seen.insert(d.clone()));

    save_watch_dirs(&final_dirs);
    final_dirs
}


fn save_watch_dirs(dirs: &[String]) {
    if let Some(path) = get_config_path() {
        if let Ok(json) = serde_json::to_string_pretty(dirs) {
            let _ = fs::write(path, json);
        }
    }
}


// Shared app state: watch dirs, notification tracking, and shutdown signal.
pub struct AppState {
    pub watch_dirs:    Mutex<Vec<String>>,
    pub notified_keys: Mutex<HashSet<String>>,
    /// Set to Some(flag) during setup(); the background thread reads it.
    pub shutdown:      Mutex<Option<Arc<AtomicBool>>>,
}

impl AppState {
    pub fn get_watch_dirs(&self) -> Vec<String> {
        self.watch_dirs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> serde_json::Value {
    println!("[devbar] get_status called");
    let dirs = state.get_watch_dirs();

    println!("[devbar] get_status: checking disks...");
    let disks = get_disk_status();
    println!("[devbar] get_status: disk check done ({})", disks.len());

    println!("[devbar] get_status: scanning git repos...");
    let repos = scan_git_repos(&dirs);
    println!("[devbar] get_status: git scan done ({})", repos.len());

    println!("[devbar] get_status: checking docker status...");
    let docker = get_docker_status();
    println!("[devbar] get_status: docker check done (available: {})", docker.available);

    println!("[devbar] get_status: checking listening ports...");
    let ports = get_port_status();
    println!("[devbar] get_status: port check done ({})", ports.len());

    println!("[devbar] get_status: returning JSON status");
    serde_json::json!({
        "disks": disks,
        "repos": repos,
        "docker": docker,
        "ports": ports,
    })
}



#[tauri::command]
fn get_watch_dirs(state: tauri::State<AppState>) -> Vec<String> {
    state.get_watch_dirs()
}

#[tauri::command]
fn set_watch_dirs(state: tauri::State<AppState>, dirs: Vec<String>) -> Vec<String> {
    let mut watch = state.watch_dirs.lock().unwrap_or_else(|e| e.into_inner());
    *watch = dirs.clone();
    save_watch_dirs(&dirs);
    dirs
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
        }
    }
}

fn check_and_notify(app_handle: &tauri::AppHandle, state: &AppState) {
    let dirs = state.get_watch_dirs();
    let disks = get_disk_status();
    let repos = scan_git_repos(&dirs);

    let mut notified = state
        .notified_keys
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let mut active_keys = HashSet::new();

    // Check critical disks
    for disk in &disks {
        if disk.status == "critical" {
            let key = format!("disk_critical_{}", disk.name);
            active_keys.insert(key.clone());
            if !notified.contains(&key) {
                let _ = app_handle
                    .notification()
                    .builder()
                    .title("DevBar Alert — Critical Disk")
                    .body(format!(
                        "Drive {} is critical ({:.1}% used, {:.1} GB free)",
                        disk.name, disk.percent_used, disk.free_gb
                    ))
                    .show();
                notified.insert(key);
            }
        }
    }

    // Check git repo thresholds (>20 changed files or >5 unpushed commits)
    for repo in &repos {
        if repo.changed_files > 20 {
            let key = format!("repo_changed_{}", repo.name);
            active_keys.insert(key.clone());
            if !notified.contains(&key) {
                let _ = app_handle
                    .notification()
                    .builder()
                    .title("DevBar Alert — Git Repo")
                    .body(format!(
                        "Repo '{}' has {} changed files",
                        repo.name, repo.changed_files
                    ))
                    .show();
                notified.insert(key);
            }
        }

        if repo.unpushed_commits > 5 {
            let key = format!("repo_unpushed_{}", repo.name);
            active_keys.insert(key.clone());
            if !notified.contains(&key) {
                let _ = app_handle
                    .notification()
                    .builder()
                    .title("DevBar Alert — Unpushed Commits")
                    .body(format!(
                        "Repo '{}' has {} unpushed commits",
                        repo.name, repo.unpushed_commits
                    ))
                    .show();
                notified.insert(key);
            }
        }
    }

    // Clean up keys that no longer cross threshold
    notified.retain(|k| active_keys.contains(k));
}

fn update_tray_icon(
    app_handle: &tauri::AppHandle,
    warning_icon: &Option<tauri::image::Image>,
    normal_icon: &Option<tauri::image::Image>,
    state: &AppState,
) {
    let dirs = state.get_watch_dirs();
    let disks = get_disk_status();
    let repos = scan_git_repos(&dirs);

    let is_warning = disks.iter().any(|d| d.status == "critical")
        || repos.iter().any(|r| r.changed_files > 20);

    if let Some(tray) = app_handle.tray_by_id("main") {
        if is_warning {
            if let Some(icon) = warning_icon {
                let _ = tray.set_icon(Some(icon.clone()));
            }
        } else {
            if let Some(icon) = normal_icon {
                let _ = tray.set_icon(Some(icon.clone()));
            }
        }
    }
}

/// Opens the given path in VS Code using the `code` CLI or fallback paths.
#[tauri::command]
fn open_in_vscode(_app: tauri::AppHandle, path: String) -> Result<(), String> {
    // 1. Try standard `code` command directly
    if std::process::Command::new("code").arg(&path).spawn().is_ok() {
        return Ok(());
    }

    // 2. On Windows, try via cmd.exe because `code` is a batch script (code.cmd)
    #[cfg(target_os = "windows")]
    {
        if std::process::Command::new("cmd")
            .args(["/C", "code", &path])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }

        // 3. Try standard installation paths on Windows
        let mut candidates = Vec::new();
        if let Some(local) = dirs::data_local_dir() {
            candidates.push(local.join("Programs\\Microsoft VS Code\\Code.exe"));
            candidates.push(local.join("Programs\\Microsoft VS Code\\bin\\code.cmd"));
        }
        if let Ok(program_files) = std::env::var("ProgramFiles") {
            let base = PathBuf::from(program_files);
            candidates.push(base.join("Microsoft VS Code\\Code.exe"));
            candidates.push(base.join("Microsoft VS Code\\bin\\code.cmd"));
        }

        for candidate in candidates {
            if candidate.exists() && std::process::Command::new(&candidate).arg(&path).spawn().is_ok() {
                return Ok(());
            }
        }
    }

    Err("VS Code (`code`) not found on PATH or standard installation locations".to_string())
}

/// Full-text search across repos: file names, commits, branch names.
#[tauri::command]
fn cmd_search_repos(state: tauri::State<AppState>, query: String) -> Vec<monitors::search::SearchHit> {
    let dirs = state.get_watch_dirs();
    search_repos(&dirs, &query)
}

/// Recently touched files across all repos, ranked by commit timestamp.
#[tauri::command]
fn cmd_get_recent_files(state: tauri::State<AppState>) -> Vec<monitors::recent::RecentFile> {
    let dirs = state.get_watch_dirs();
    get_recent_files(&dirs, 5)
}

/// Cross-repo package.json dependency versions.
#[tauri::command]
fn cmd_get_dep_versions(state: tauri::State<AppState>) -> Vec<monitors::deps::RepoDeps> {
    let dirs = state.get_watch_dirs();
    get_dep_versions(&dirs)
}

/// Returns the persisted theme name ("dark" | "light" | "midnight").
#[tauri::command]
fn cmd_get_theme() -> String {
    load_theme()
}

/// Saves the chosen theme to disk and returns it.
#[tauri::command]
fn cmd_set_theme(theme: String) -> String {
    save_theme(&theme);
    theme
}

fn main() {
    let watch_dirs = load_watch_dirs();
    println!("[devbar] watching folders: {:?}", watch_dirs);

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_shell::init())

        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::AppleScript, None))
        .manage(AppState {
            watch_dirs:    Mutex::new(watch_dirs),
            notified_keys: Mutex::new(HashSet::new()),
            shutdown:      Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_watch_dirs,
            set_watch_dirs,
            open_in_vscode,
            cmd_search_repos,
            cmd_get_recent_files,
            cmd_get_dep_versions,
            cmd_get_theme,
            cmd_set_theme,
        ])
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "Quit DevBar", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let normal_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png")).ok();

            let tray_icon = app
                .default_window_icon()
                .cloned()
                .or_else(|| normal_icon.clone());


            let mut builder = TrayIconBuilder::with_id("main")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0);
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_window(tray.app_handle());
                    }
                });

            if let Some(icon) = tray_icon {
                builder = builder.icon(icon);
            }

            let _tray = builder.build(app)?;


            let handle        = app.handle().clone();
            let normal_icon  = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png")).ok();
            let warning_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon-warning.png")).ok();

            // Shutdown flag: set to true when the app is exiting so the
            // background thread can exit its loop instead of orphaning the process.
            let shutdown = Arc::new(AtomicBool::new(false));
            let shutdown_bg = Arc::clone(&shutdown);

            // Store the shutdown flag in AppState so RunEvent can reach it.
            {
                let state = handle.state::<AppState>();
                *state.shutdown.lock().unwrap() = Some(Arc::clone(&shutdown));
            }

            std::thread::Builder::new()
                .name("devbar-bg".into())
                .spawn(move || {
                    loop {
                        if shutdown_bg.load(Ordering::Relaxed) {
                            break;
                        }
                        // Catch any panic inside git/disk helpers so the thread
                        // doesn't die silently. AppHandle is not RefUnwindSafe,
                        // so we must assert that ourselves.
                        let handle_ref  = &handle;
                        let wi_ref      = &warning_icon;
                        let ni_ref      = &normal_icon;
                        let panic_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            let state = handle_ref.state::<AppState>();
                            update_tray_icon(handle_ref, wi_ref, ni_ref, &state);
                            check_and_notify(handle_ref, &state);
                        }));
                        if let Err(err) = panic_res {
                            eprintln!("[devbar] background monitor panic caught safely: {:?}", err);
                        }
                        // Sleep in 1-second increments so we can respond to
                        // the shutdown flag within ~1 second.
                        for _ in 0..10 {
                            if shutdown_bg.load(Ordering::Relaxed) {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_secs(1));
                        }
                    }
                    println!("[devbar] background thread exiting cleanly");
                })
                .expect("failed to spawn background thread");

            // Hide the window when the user clicks away (blur) or requests close.
            if let Some(window) = app.get_webview_window("main") {
                let win_clone = window.clone();
                window.on_window_event(move |event| {
                    match event {
                        WindowEvent::CloseRequested { api, .. } => {
                            api.prevent_close();
                            let _ = win_clone.hide();
                        }
                        WindowEvent::Focused(false) => {
                            let _ = win_clone.hide();
                        }
                        _ => {}
                    }
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("[devbar] fatal during build: {e}");
            std::process::exit(1);
        })
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // Signal the background thread to stop so the process exits cleanly.
                // Clone the Arc out of the guard so the borrow on `state` ends here.
                let flag: Option<Arc<AtomicBool>> = app
                    .state::<AppState>()
                    .shutdown
                    .lock()
                    .ok()
                    .and_then(|g| g.clone());
                if let Some(f) = flag {
                    f.store(true, Ordering::Relaxed);
                    println!("[devbar] shutdown signal sent to background thread");
                }
            }
        });
}
