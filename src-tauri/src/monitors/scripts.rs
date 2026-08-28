use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ScriptAction {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub category: String, // "package", "rust", "docker", "git"
    pub description: String,
}

#[derive(Deserialize)]
struct PackageJson {
    scripts: Option<std::collections::BTreeMap<String, String>>,
}

/// Detects available scripts and quick actions for a given repository path.
pub fn get_repo_scripts(repo_path_str: &str) -> Vec<ScriptAction> {
    let mut actions = Vec::new();
    let repo_path = Path::new(repo_path_str);

    if !repo_path.exists() {
        return actions;
    }

    // 1. Detect package.json scripts (Node.js)
    let pkg_path = repo_path.join("package.json");
    if pkg_path.exists() {
        if let Ok(content) = fs::read_to_string(&pkg_path) {
            if let Ok(pkg) = serde_json::from_str::<PackageJson>(&content) {
                if let Some(scripts) = pkg.scripts {
                    for (script_name, script_cmd) in scripts {
                        actions.push(ScriptAction {
                            name: format!("npm run {}", script_name),
                            command: if cfg!(windows) { "npm.cmd".to_string() } else { "npm".to_string() },
                            args: vec!["run".to_string(), script_name.clone()],
                            category: "package".to_string(),
                            description: script_cmd,
                        });
                    }
                }
            }
        }
    }

    // 2. Detect Cargo.toml (Rust)
    if repo_path.join("Cargo.toml").exists() {
        actions.push(ScriptAction {
            name: "cargo run".to_string(),
            command: "cargo".to_string(),
            args: vec!["run".to_string()],
            category: "rust".to_string(),
            description: "Build and run Rust project".to_string(),
        });
        actions.push(ScriptAction {
            name: "cargo test".to_string(),
            command: "cargo".to_string(),
            args: vec!["test".to_string()],
            category: "rust".to_string(),
            description: "Run Rust test suite".to_string(),
        });
        actions.push(ScriptAction {
            name: "cargo check".to_string(),
            command: "cargo".to_string(),
            args: vec!["check".to_string()],
            category: "rust".to_string(),
            description: "Fast compilation check".to_string(),
        });
    }

    // 3. Detect Docker Compose
    if repo_path.join("docker-compose.yml").exists()
        || repo_path.join("docker-compose.yaml").exists()
        || repo_path.join("compose.yaml").exists()
        || repo_path.join("compose.yml").exists()
    {
        actions.push(ScriptAction {
            name: "docker compose up -d".to_string(),
            command: "docker".to_string(),
            args: vec!["compose".to_string(), "up".to_string(), "-d".to_string()],
            category: "docker".to_string(),
            description: "Start containers in background".to_string(),
        });
        actions.push(ScriptAction {
            name: "docker compose down".to_string(),
            command: "docker".to_string(),
            args: vec!["compose".to_string(), "down".to_string()],
            category: "docker".to_string(),
            description: "Stop and remove containers".to_string(),
        });
    }

    // 4. Git Quick Actions
    if repo_path.join(".git").exists() {
        actions.push(ScriptAction {
            name: "git pull".to_string(),
            command: "git".to_string(),
            args: vec!["pull".to_string()],
            category: "git".to_string(),
            description: "Fetch and merge remote changes".to_string(),
        });
        actions.push(ScriptAction {
            name: "git fetch".to_string(),
            command: "git".to_string(),
            args: vec!["fetch".to_string(), "--all".to_string()],
            category: "git".to_string(),
            description: "Fetch remote refs and branches".to_string(),
        });
    }

    actions
}

/// Executes a script/command in the given repository path with a 15-second execution timeout.
pub fn run_repo_script(repo_path_str: &str, command: &str, args: &[String]) -> Result<String, String> {
    let repo_path = Path::new(repo_path_str);
    if !repo_path.exists() {
        return Err(format!("Repository path '{}' does not exist", repo_path_str));
    }

    let mut cmd = Command::new(command);
    cmd.args(args).current_dir(repo_path);

    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn command '{}': {}", command, e))?;

    let timeout = Duration::from_secs(15);
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => {
            let output = child
                .wait_with_output()
                .map_err(|e| format!("Failed to read output: {}", e))?;

            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            let combined = format!(
                "Exit Status: {}\n\n--- STDOUT ---\n{}\n--- STDERR ---\n{}",
                status, stdout, stderr
            );

            if status.success() {
                Ok(combined)
            } else {
                Err(combined)
            }
        }
        Ok(None) => {
            let _ = child.kill();
            let _ = child.wait();
            Err(format!(
                "Command '{} {:?}' timed out after 15 seconds.",
                command, args
            ))
        }
        Err(e) => Err(format!("Error waiting for command: {}", e)),
    }
}
