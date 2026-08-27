use serde::Serialize;
use std::collections::HashMap;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};
use wait_timeout::ChildExt;

/// Default dev ports DevBar watches when user hasn't customised.
pub const DEFAULT_PORTS: &[u16] = &[3000, 3001, 5173, 5174, 8000, 8080, 4200, 9000];

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
/// in_use=true  → something is LISTENING on that port
/// in_use=false → port is free
pub fn get_port_status_for(ports: &[u16]) -> Vec<PortInfo> {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let listening = parse_listening_ports(&sys, ports);

    ports
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
                PortInfo { port, in_use: false, process_name: None, pid: None }
            }
        })
        .collect()
}

/// Convenience wrapper using the default port list.
pub fn get_port_status() -> Vec<PortInfo> {
    get_port_status_for(DEFAULT_PORTS)
}

/// Runs `netstat -ano -p tcp` and returns a map of port → (pid, process_name)
/// for every local address that is in LISTENING state.
fn parse_listening_ports(sys: &System, watch: &[u16]) -> HashMap<u16, (u32, String)> {
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

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 5 {
            continue;
        }

        let local_addr = parts[1];
        let pid_str = parts[parts.len() - 1];

        if let (Some(port), Ok(pid)) = (parse_port(local_addr), pid_str.parse::<u32>()) {
            if watch.contains(&port) {
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
    addr.rfind(':').and_then(|pos| addr[pos + 1..].parse::<u16>().ok())
}

fn get_process_name(sys: &System, pid: u32) -> String {
    if let Some(proc_) = sys.process(Pid::from(pid as usize)) {
        proc_.name().to_string_lossy().to_string()
    } else {
        tasklist_name(pid).unwrap_or_else(|| "Unknown".to_string())
    }
}

fn tasklist_name(pid: u32) -> Option<String> {
    let mut cmd = Command::new("tasklist");
    cmd.args(["/FI", &format!("PID eq {}", pid), "/NH", "/FO", "CSV"]);
    let out = run_cmd_with_timeout(cmd, Duration::from_secs(2))?;
    let text = String::from_utf8_lossy(&out.stdout);
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
        let ports = get_port_status_for(DEFAULT_PORTS);
        assert_eq!(ports.len(), DEFAULT_PORTS.len());
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

    #[test]
    fn test_custom_ports() {
        let ports = get_port_status_for(&[80, 443, 22]);
        assert_eq!(ports.len(), 3);
    }
}
