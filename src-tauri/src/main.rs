// Prevents an extra console window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod monitors;

use monitors::disk::get_disk_status;
use monitors::docker::get_docker_status;
use monitors::git::scan_git_repos;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use std::sync::Mutex;
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


// Shared app state: watch dirs and notification state tracking.
pub struct AppState {
    pub watch_dirs: Mutex<Vec<String>>,
    pub notified_keys: Mutex<HashSet<String>>,
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

    println!("[devbar] get_status: returning JSON status");
    serde_json::json!({
        "disks": disks,
        "repos": repos,
        "docker": docker,
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

fn main() {
    let watch_dirs = load_watch_dirs();
    println!("[devbar] watching folders: {:?}", watch_dirs);

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::AppleScript, None))
        .manage(AppState {
            watch_dirs: Mutex::new(watch_dirs),
            notified_keys: Mutex::new(HashSet::new()),
        })
        .invoke_handler(tauri::generate_handler![get_status, get_watch_dirs, set_watch_dirs])
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


            let handle = app.handle().clone();
            let normal_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon.png")).ok();
            let warning_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/icon-warning.png")).ok();

            std::thread::spawn(move || {
                loop {
                    let state = handle.state::<AppState>();
                    update_tray_icon(&handle, &warning_icon, &normal_icon, &state);
                    check_and_notify(&handle, &state);
                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
            });

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
        .run(tauri::generate_context!())
        .expect("error while running DevBar");
}
