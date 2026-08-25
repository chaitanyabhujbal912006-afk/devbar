use serde::Serialize;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

#[derive(Serialize, Clone)]
pub struct SearchHit {
    /// "repo" | "branch" | "commit" | "file"
    pub kind: String,
    /// Display label (repo name, branch, commit subject, or file path)
    pub label: String,
    /// Short name of the owning repo
    pub repo: String,
    /// Absolute path of the repo root
    pub repo_path: String,
    /// Absolute path to open — repo root for repo/branch/commit, full file path for file
    pub open_path: String,
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

/// Search across all repos in `dirs` for `query`.
/// Matches: repo names, branch names, recent commit subjects, tracked file paths.
/// Results are capped (files: 2000 per repo, commits: 20 per repo).
pub fn search_repos(dirs: &[String], query: &str) -> Vec<SearchHit> {
    if query.len() < 2 {
        return vec![];
    }
    let q = query.to_lowercase();
    let mut hits: Vec<SearchHit> = Vec::new();

    // Collect all repo roots the same way the git monitor does
    let repos = collect_repo_paths(dirs);

    for (repo_name, repo_path) in &repos {
        let path = Path::new(repo_path);

        // 1. Repo name match
        if repo_name.to_lowercase().contains(&q) {
            hits.push(SearchHit {
                kind: "repo".into(),
                label: repo_name.clone(),
                repo: repo_name.clone(),
                repo_path: repo_path.clone(),
                open_path: repo_path.clone(),
            });
        }

        // 2. Branch name match
        if let Some(branch) = get_branch(path) {
            if branch.to_lowercase().contains(&q) {
                hits.push(SearchHit {
                    kind: "branch".into(),
                    label: format!("{} ({})", branch, repo_name),
                    repo: repo_name.clone(),
                    repo_path: repo_path.clone(),
                    open_path: repo_path.clone(),
                });
            }
        }

        // 3. Recent commit subjects
        let mut cmd = Command::new("git");
        cmd.args(["log", "--oneline", "-20", "--no-decorate"]).current_dir(path);
        if let Some(out) = run_cmd(cmd, Duration::from_secs(3)) {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                // line format: "<hash> <subject>"
                let subject = line.splitn(2, ' ').nth(1).unwrap_or("").trim();
                if subject.to_lowercase().contains(&q) {
                    hits.push(SearchHit {
                        kind: "commit".into(),
                        label: subject.to_string(),
                        repo: repo_name.clone(),
                        repo_path: repo_path.clone(),
                        open_path: repo_path.clone(),
                    });
                }
            }
        }

        // 4. Tracked file paths (git ls-files, max 2000 per repo)
        let mut cmd = Command::new("git");
        cmd.args(["ls-files"]).current_dir(path);
        if let Some(out) = run_cmd(cmd, Duration::from_secs(5)) {
            let mut file_count = 0usize;
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if file_count >= 2000 {
                    break;
                }
                let rel = line.trim();
                if rel.is_empty() {
                    continue;
                }
                // Match against just the filename part first (faster feel)
                let filename = Path::new(rel)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(rel);
                if filename.to_lowercase().contains(&q) || rel.to_lowercase().contains(&q) {
                    // Build absolute open path
                    let abs = format!("{}\\{}", repo_path, rel.replace('/', "\\"));
                    hits.push(SearchHit {
                        kind: "file".into(),
                        label: rel.to_string(),
                        repo: repo_name.clone(),
                        repo_path: repo_path.clone(),
                        open_path: abs,
                    });
                    file_count += 1;
                }
            }
        }
    }

    // Cap total results to keep the overlay snappy
    hits.truncate(80);
    hits
}

/// Collect (name, path) pairs for all git repos under the given dirs.
/// Mirrors the logic in git.rs but without full inspection.
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

fn get_branch(path: &Path) -> Option<String> {
    let mut cmd = Command::new("git");
    cmd.args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path);
    let out = run_cmd(cmd, Duration::from_secs(2))?;
    if out.status.success() {
        let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !b.is_empty() {
            return Some(b);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_repos_query() {
        let dirs = vec!["C:\\projects".to_string()];
        let hits = search_repos(&dirs, "main");
        println!("SEARCH HITS FOR 'main': {}", hits.len());
        for h in &hits {
            println!("  - [{}] {} ({})", h.kind, h.label, h.repo);
        }
    }
}

