// Prevents an extra console window from appearing on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod monitors;

use monitors::disk::get_disk_status;
use monitors::docker::get_docker_status;
use monitors::git::scan_git_repos;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WindowEvent,
};

fn get_config_path() -> Option<PathBuf> {
    let mut path = dirs::config_dir()?;
    path.push("devbar");
    let _ = fs::create_dir_all(&path);
    path.push("watch_dirs.json");
    Some(path)
}

fn load_watch_dirs() -> Vec<String> {
    if let Some(path) = get_config_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(dirs) = serde_json::from_str::<Vec<String>>(&content) {
                    return dirs;
                }
            }
        }
    }
    vec![
        shellexpand::tilde("~/dev").to_string(),
        shellexpand::tilde("~/projects").to_string(),
        shellexpand::tilde("~/code").to_string(),
    ]
}

fn save_watch_dirs(dirs: &[String]) {
    if let Some(path) = get_config_path() {
        if let Ok(json) = serde_json::to_string_pretty(dirs) {
            let _ = fs::write(path, json);
        }
    }
}

// Shared app state: the folder(s) we scan for git repos.
pub struct AppState {
    pub watch_dirs: Mutex<Vec<String>>,
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> serde_json::Value {
    let dirs = state.watch_dirs.lock().unwrap().clone();
    let disks = get_disk_status();
    let repos = scan_git_repos(&dirs);
    let docker = get_docker_status();

    serde_json::json!({
        "disks": disks,
        "repos": repos,
        "docker": docker,
    })
}

#[tauri::command]
fn get_watch_dirs(state: tauri::State<AppState>) -> Vec<String> {
    state.watch_dirs.lock().unwrap().clone()
}

#[tauri::command]
fn set_watch_dirs(state: tauri::State<AppState>, dirs: Vec<String>) -> Vec<String> {
    let mut watch = state.watch_dirs.lock().unwrap();
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

fn main() {
    let watch_dirs = load_watch_dirs();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            watch_dirs: Mutex::new(watch_dirs),
        })
        .invoke_handler(tauri::generate_handler![get_status, get_watch_dirs, set_watch_dirs])
        .setup(|app| {
            let quit = MenuItem::with_id(app, "quit", "Quit DevBar", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&quit])?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
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
                })
                .build(app)?;

            // Hide the window instead of closing it when the user clicks away/closes it.
            if let Some(window) = app.get_webview_window("main") {
                let win_clone = window.clone();
                window.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = win_clone.hide();
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running DevBar");
}
