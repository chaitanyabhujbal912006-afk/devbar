use serde::Serialize;
use std::collections::HashMap;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};
use wait_timeout::ChildExt;

/// The fixed set of dev ports DevBar watches.
const WATCHED_PORTS: &[u16] = &[3000, 3001, 5173, 5174, 8000, 8080, 4200, 9000];

#[derive(Serialize, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub in_use: bool,
    pub process_name: Option<String>,
    pub pid: Option<u32>,
}

fn run_cmd_with_timeout(mut cmd: Command, timeout: Duration) -> Option<Output> {
    let mut child = cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().ok()?;
    match child.wait_timeout(timeout) {
        Ok(Some(_)) => child.wait_with_output().ok(),
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            None
        }
    }
}

/// Returns one PortInfo per watched port.
/// in_use=true  → something is LISTENING on that port (red dot in UI)
/// in_use=false → port is free (green dot in UI)
pub fn get_port_status() -> Vec<PortInfo> {
    // Build sysinfo system with refreshed processes for PID→name lookups.
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    // Parse netstat to find which ports are LISTENING and who owns them.
    // HashMap<port, (pid, process_name)>
    let listening = parse_listening_ports(&sys);

    WATCHED_PORTS
        .iter()
        .map(|&port| {
            if let Some((pid, name)) = listening.get(&port) {
                PortInfo {
                    port,
                    in_use: true,
                    process_name: Some(name.clone()),
                    pid: Some(*pid),
                }
            } else {
                PortInfo {
                    port,
                    in_use: false,
                    process_name: None,
                    pid: None,
                }
            }
        })
        .collect()
}

/// Runs `netstat -ano -p tcp` and returns a map of port → (pid, process_name)
/// for every local address that is in LISTENING state.
fn parse_listening_ports(sys: &System) -> HashMap<u16, (u32, String)> {
    let mut map = HashMap::new();

    let mut cmd = Command::new("netstat");
    cmd.args(["-ano", "-p", "tcp"]);

    let output = match run_cmd_with_timeout(cmd, Duration::from_secs(4)) {
        Some(out) if out.status.success() => out,
        _ => return map,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let line = line.trim();
        if !line.contains("LISTENING") {
            continue;
        }

        // netstat -ano output format (Windows):
        //   TCP  0.0.0.0:3000   0.0.0.0:0   LISTENING   1234
        //   TCP  [::]:3000      [::]:0       LISTENING   1234
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let local_addr = parts[1];
        let pid_str = parts[parts.len() - 1];

        if let (Some(port), Ok(pid)) = (parse_port(local_addr), pid_str.parse::<u32>()) {
            // Only record ports in our watch list to keep the map small.
            if WATCHED_PORTS.contains(&port) {
                map.entry(port).or_insert_with(|| {
                    let name = get_process_name(sys, pid);
                    (pid, name)
                });
            }
        }
    }

    map
}

fn parse_port(addr: &str) -> Option<u16> {
    // Handles both "0.0.0.0:3000" and "[::]:3000" (IPv6 bracket notation)
    addr.rfind(':').and_then(|pos| addr[pos + 1..].parse::<u16>().ok())
}

fn get_process_name(sys: &System, pid: u32) -> String {
    if let Some(proc_) = sys.process(Pid::from(pid as usize)) {
        proc_.name().to_string_lossy().to_string()
    } else {
        // Fallback: try tasklist if sysinfo missed it (e.g. elevated processes)
        tasklist_name(pid).unwrap_or_else(|| "Unknown".to_string())
    }
}

fn tasklist_name(pid: u32) -> Option<String> {
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"]);
    let out = run_cmd_with_timeout(cmd, Duration::from_secs(2))?;
    let text = String::from_utf8_lossy(&out.stdout);
    // CSV line: "process.exe","1234","Console","1","12,345 K"
    for line in text.lines() {
        let trimmed = line.trim().trim_matches('"');
        if !trimmed.is_empty() && !trimmed.starts_with("INFO:") {
            let name = trimmed.split('"').next().unwrap_or("Unknown");
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_port_status_returns_all_watched() {
        let ports = get_port_status();
        // Should always return exactly WATCHED_PORTS.len() entries
        assert_eq!(ports.len(), WATCHED_PORTS.len());
        println!("PORT STATUS:");
        for p in &ports {
            println!(
                "  :{} — {} — {:?} (pid {:?})",
                p.port,
                if p.in_use { "IN USE" } else { "free" },
                p.process_name,
                p.pid,
            );
        }
    }
}
