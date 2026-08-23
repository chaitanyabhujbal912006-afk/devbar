use serde::Serialize;
use std::process::Command;

#[derive(Serialize, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub state: String,  // "running" | "exited" | "restarting" | "paused" | etc.
    pub status: String, // human-readable, e.g. "Up 2 hours" or "Exited (0) 3 minutes ago"
}

#[derive(Serialize, Clone)]
pub struct DockerStatus {
    pub available: bool,
    pub containers: Vec<ContainerInfo>,
}

pub fn get_docker_status() -> DockerStatus {
    // Attempt to call `docker ps -a --format json`.
    // `--format json` outputs one JSON object per line (NDJSON) since Docker 20.10.
    let output = Command::new("docker")
        .args(["ps", "-a", "--format", "json"])
        .output();

    let output = match output {
        Ok(out) => out,
        // docker binary not found in PATH
        Err(_) => return DockerStatus { available: false, containers: vec![] },
    };

    if !output.status.success() {
        // Daemon not running, permission denied, or other runtime error.
        return DockerStatus { available: false, containers: vec![] };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut containers = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            // Names may be comma-separated; take the first one and strip any leading "/".
            let name = val["Names"]
                .as_str()
                .unwrap_or("")
                .split(',')
                .next()
                .unwrap_or("")
                .trim_start_matches('/')
                .to_string();

            let image = val["Image"].as_str().unwrap_or("").to_string();

            // "State" is lowercase in the JSON output: running/exited/paused/restarting/dead/created
            let state = val["State"].as_str().unwrap_or("unknown").to_string();

            // "Status" is the human-readable description shown in `docker ps`
            let status = val["Status"].as_str().unwrap_or("").to_string();

            if !name.is_empty() {
                containers.push(ContainerInfo { name, image, state, status });
            }
        }
    }

    DockerStatus { available: true, containers }
}
