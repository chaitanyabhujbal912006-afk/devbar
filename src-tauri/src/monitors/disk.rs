use serde::Serialize;
use sysinfo::Disks;

#[derive(Serialize, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_gb: f64,
    pub free_gb: f64,
    pub percent_used: f64,
    pub status: String, // "ok" | "warn" | "critical"
}

pub fn get_disk_status() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();

    disks
        .iter()
        .map(|d| {
            let total = d.total_space() as f64 / 1_073_741_824.0; // bytes -> GB
            let free = d.available_space() as f64 / 1_073_741_824.0;
            let used_pct = if total > 0.0 {
                ((total - free) / total) * 100.0
            } else {
                0.0
            };

            let status = if used_pct >= 90.0 {
                "critical"
            } else if used_pct >= 75.0 {
                "warn"
            } else {
                "ok"
            };

            DiskInfo {
                name: d.name().to_string_lossy().to_string(),
                mount_point: d.mount_point().to_string_lossy().to_string(),
                total_gb: (total * 10.0).round() / 10.0,
                free_gb: (free * 10.0).round() / 10.0,
                percent_used: (used_pct * 10.0).round() / 10.0,
                status: status.to_string(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_status_returns_mounted_drives() {
        let disks = get_disk_status();
        println!("DISKS FOUND: {}", disks.len());
        for d in &disks {
            println!(
                "  - Drive [{}] mounted at {}: {:.1} GB free / {:.1} GB total ({:.1}%) -> {}",
                d.name, d.mount_point, d.free_gb, d.total_gb, d.percent_used, d.status
            );
            assert!(d.total_gb >= 0.0);
            assert!(d.free_gb >= 0.0);
            assert!(["ok", "warn", "critical"].contains(&d.status.as_str()));
        }
    }
}

