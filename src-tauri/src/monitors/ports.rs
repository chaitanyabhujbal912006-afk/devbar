use serde::Serialize;
use std::collections::HashSet;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use sysinfo::{Pid, ProcessesToUpdate, System};
use wait_timeout::ChildExt;

#[derive(Serialize, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub process_name: String,
    pub pid: u32,
    pub address: String,
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

pub fn get_port_status() -> Vec<PortInfo> {
    let mut ports = Vec::new();
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);

    let mut cmd = Command::new("netstat");
    cmd.args(["-ano", "-p", "tcp"]);

    let output = match run_cmd_with_timeout(cmd, Duration::from_secs(3)) {
        Some(out) if out.status.success() => out,
        _ => return ports,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut seen_ports = HashSet::new();

    for line in stdout.lines() {
        let line = line.trim();
        if !line.contains("LISTENING") {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        // Format: Protocol LocalAddress ForeignAddress State PID
        // Example: TCP 127.0.0.1:3000 0.0.0.0:0 LISTENING 1234
        if parts.len() >= 5 {
            let local_addr = parts[1];
            let pid_str = parts[parts.len() - 1];

            if let Ok(pid) = pid_str.parse::<u32>() {
                if let Some(port) = parse_port(local_addr) {
                    // Filter out system ports if needed, but include common dev & app ports
                    if seen_ports.insert(port) {
                        let proc_name = get_process_name(&sys, pid);
                        ports.push(PortInfo {
                            port,
                            process_name: proc_name,
                            pid,
                            address: local_addr.to_string(),
                        });
                    }
                }
            }
        }
    }

    ports.sort_by_key(|p| p.port);
    ports
}

fn parse_port(addr: &str) -> Option<u16> {
    if let Some(pos) = addr.rfind(':') {
        addr[pos + 1..].parse::<u16>().ok()
    } else {
        None
    }
}

fn get_process_name(sys: &System, pid: u32) -> String {
    if let Some(proc_) = sys.process(Pid::from(pid as usize)) {
        proc_.name().to_string_lossy().to_string()
    } else {
        "Unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_port_status() {
        let ports = get_port_status();
        println!("FOUND LISTENING PORTS ({}):", ports.len());
        for p in &ports {
            println!("  - Port :{} -> Process: {} (PID {})", p.port, p.process_name, p.pid);
        }
    }
}

