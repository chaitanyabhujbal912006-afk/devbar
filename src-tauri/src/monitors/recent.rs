use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

/// A recently-touched file across any watched repo.
#[derive(Serialize, Clone)]
pub struct RecentFile {
    /// Display name of the file (basename)
    pub filename: String,
    /// Repo-relative path, e.g. "src/main.rs"
    pub relative_path: String,
    /// Absolute path for opening in VS Code
    pub absolute_path: String,
    /// Short name of the owning repo
    pub repo: String,
    /// ISO-8601-ish timestamp of the last commit touching this file
    pub timestamp: String,
    /// Human-readable age string, e.g. "2h ago"
    pub age: String,
}

fn run_cmd(mut cmd: Command, timeout: Duration) -> Option<Output> {
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

/// Returns up to `limit` recently-touched files across all git repos in `dirs`,
/// ranked by commit timestamp (most recent first).
pub fn get_recent_files(dirs: &[String], limit: usize) -> Vec<RecentFile> {
    let repos = collect_repo_paths(dirs);

    // For each repo, get the last-touched file per commit (up to 50 commits back).
    // We'll collect (unix_ts, RecentFile) pairs, then sort and take the top N.
    let mut candidates: Vec<(i64, RecentFile)> = Vec::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    for (repo_name, repo_path) in &repos {
        // git log with unix timestamps and file names.
        // Format: each commit prints its unix timestamp on one line, then
        // a blank line, then the list of changed files.
        let mut cmd = Command::new("git");
        cmd.args([
            "log",
            "--diff-filter=AM",   // Added or Modified only
            "--name-only",
            "--format=%ct",       // unix commit timestamp
            "-50",
        ])
        .current_dir(Path::new(repo_path));

        let out = match run_cmd(cmd, Duration::from_secs(5)) {
            Some(o) => o,
            None => continue,
        };

        let text = String::from_utf8_lossy(&out.stdout);
        let mut current_ts: Option<i64> = None;

        // Track the most recent file seen per (repo, rel_path) to deduplicate
        let mut seen: HashMap<String, i64> = HashMap::new();

        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Lines that are pure digits are timestamps
            if line.chars().all(|c| c.is_ascii_digit()) {
                current_ts = line.parse::<i64>().ok();
                continue;
            }
            // Otherwise it's a file path
            if let Some(ts) = current_ts {
                let rel = line.to_string();
                let entry = seen.entry(rel.clone()).or_insert(i64::MIN);
                if ts > *entry {
                    *entry = ts;
                    let abs = format!("{}\\{}", repo_path, rel.replace('/', "\\"));
                    let filename = Path::new(&rel)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&rel)
                        .to_string();
                    let age = human_age(now - ts);
                    candidates.push((
                        ts,
                        RecentFile {
                            filename,
                            relative_path: rel,
                            absolute_path: abs,
                            repo: repo_name.clone(),
                            timestamp: ts.to_string(),
                            age,
                        },
                    ));
                }
            }
        }
    }

    // Sort by timestamp descending, then deduplicate by absolute_path, take limit
    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    let mut seen_paths = std::collections::HashSet::new();
    let mut result = Vec::new();
    for (_, rf) in candidates {
        if seen_paths.insert(rf.absolute_path.clone()) {
            result.push(rf);
            if result.len() >= limit {
                break;
            }
        }
    }
    result
}

fn human_age(seconds: i64) -> String {
    if seconds < 0 {
        return "just now".into();
    }
    match seconds {
        0..=119 => "just now".into(),
        120..=3599 => format!("{}m ago", seconds / 60),
        3600..=86399 => format!("{}h ago", seconds / 3600),
        86400..=604799 => format!("{}d ago", seconds / 86400),
        _ => format!("{}w ago", seconds / 604800),
    }
}

fn collect_repo_paths(dirs: &[String]) -> Vec<(String, String)> {
    use walkdir::WalkDir;
    let mut repos = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for root in dirs {
        let root_path = Path::new(root);
        if !root_path.exists() {
            continue;
        }
        let walker = WalkDir::new(root_path)
            .max_depth(4)
            .into_iter()
            .filter_entry(|e| {
                let p = e.path();
                if !p.is_dir() {
                    return false;
                }
                if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                    if (name.starts_with('.') && name != ".")
                        || name == "node_modules"
                        || name == "target"
                        || name == "dist"
                        || name == "build"
                        || name == "vendor"
                    {
                        return false;
                    }
                }
                true
            });

        for entry in walker.flatten() {
            let p = entry.path();
            if p.join(".git").exists() {
                let path_str = p.to_string_lossy().to_string();
                if seen.insert(path_str.clone()) {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    repos.push((name, path_str));
                }
            }
        }
    }
    repos
}
