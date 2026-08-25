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
    let start = std::time::Instant::now();
    let (repos, walk_count) = collect_repo_paths(dirs);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    // Concurrently scan all repos in parallel threads
    let candidates = std::thread::scope(|s| {
        let mut handles = Vec::new();

        for (repo_name, repo_path) in &repos {
            handles.push(s.spawn(move || {
                let mut repo_candidates = Vec::new();
                let repo_p = Path::new(repo_path);

                // 1. Check uncommitted / modified working tree files via `git status --porcelain`
                let mut cmd_status = Command::new("git");
                cmd_status.args(["status", "--porcelain"]).current_dir(repo_p);
                if let Some(out) = run_cmd(cmd_status, Duration::from_secs(2)) {
                    for line in String::from_utf8_lossy(&out.stdout).lines() {
                        let line = line.trim();
                        if line.len() > 3 {
                            let rel = line[3..].trim().to_string();
                            let abs = format!("{}\\{}", repo_path, rel.replace('/', "\\"));
                            let abs_p = Path::new(&abs);
                            if abs_p.exists() && abs_p.is_file() {
                                let mtime = abs_p
                                    .metadata()
                                    .and_then(|m| m.modified())
                                    .ok()
                                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                    .map(|d| d.as_secs() as i64)
                                    .unwrap_or(now);

                                let filename = abs_p
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or(&rel)
                                    .to_string();
                                let age = human_age(now - mtime);

                                repo_candidates.push((
                                    mtime,
                                    RecentFile {
                                        filename,
                                        relative_path: rel,
                                        absolute_path: abs,
                                        repo: repo_name.clone(),
                                        timestamp: mtime.to_string(),
                                        age,
                                    },
                                ));
                            }
                        }
                    }
                }

                // 2. Check committed files via `git log`
                let mut cmd_log = Command::new("git");
                cmd_log
                    .args([
                        "log",
                        "--diff-filter=AM",
                        "--name-only",
                        "--format=%ct",
                        "-30",
                    ])
                    .current_dir(repo_p);

                if let Some(out) = run_cmd(cmd_log, Duration::from_secs(3)) {
                    let text = String::from_utf8_lossy(&out.stdout);
                    let mut current_ts: Option<i64> = None;
                    let mut seen: HashMap<String, i64> = HashMap::new();

                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }
                        if line.chars().all(|c| c.is_ascii_digit()) {
                            current_ts = line.parse::<i64>().ok();
                            continue;
                        }
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
                                repo_candidates.push((
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

                repo_candidates
            }));
        }

        let mut all = Vec::new();
        for h in handles {
            if let Ok(res) = h.join() {
                all.extend(res);
            }
        }
        all
    });

    // Sort by timestamp descending, deduplicate by absolute_path, take limit
    let mut candidates = candidates;
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

    let elapsed = start.elapsed();
    println!(
        "[devbar] get_recent_files: scanned {} dirs across {} repos in {:?}, returning {} recent files",
        walk_count,
        repos.len(),
        elapsed,
        result.len()
    );

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

/// Returns (repos, visited_dir_count)
pub fn collect_repo_paths(dirs: &[String]) -> (Vec<(String, String)>, usize) {
    use walkdir::WalkDir;
    let mut repos = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut visited_count = 0usize;

    for root in dirs {
        let root_path = Path::new(root);
        if !root_path.exists() {
            continue;
        }
        let mut local_repos = Vec::new();

        let walker = WalkDir::new(root_path)
            .max_depth(4)
            .into_iter()
            .filter_entry(|e| {
                visited_count += 1;
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

                // If this folder contains a .git directory, it is a git repo root.
                // Collect it and return FALSE to prevent descending into subdirectories!
                if p.join(".git").exists() {
                    let path_str = p.to_string_lossy().to_string();
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    local_repos.push((name, path_str));
                    return false;
                }
                true
            });

        for _ in walker.flatten() {}

        for (name, path_str) in local_repos {
            if seen.insert(path_str.clone()) {
                repos.push((name, path_str));
            }
        }
    }
    (repos, visited_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recent_files_performance() {
        let dirs = vec!["C:\\projects".to_string()];
        let start = std::time::Instant::now();
        let files = get_recent_files(&dirs, 5);
        let elapsed = start.elapsed();

        println!("\n=== RECENT FILES TEST ===");
        println!("Elapsed: {:?}", elapsed);
        println!("Found files ({}):", files.len());
        for f in &files {
            println!("  - [{}] {} ({}) -> {}", f.repo, f.filename, f.age, f.absolute_path);
        }
        assert!(!files.is_empty());
    }
}

