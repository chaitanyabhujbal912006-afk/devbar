use serde::Serialize;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;
use walkdir::WalkDir;

#[derive(Serialize, Clone)]
pub struct RepoStatus {
    pub name: String,
    pub path: String,
    pub dirty: bool,
    pub changed_files: usize,
    pub unpushed_commits: usize,
    pub branch: String,
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

/// Scans the given root directories (up to 4 levels deep) for git repositories.
/// Uses filter_entry to log visited directories and avoid descending into existing git repos,
/// hidden folders, or heavy dependency trees.
pub fn scan_git_repos(root_dirs: &[String]) -> Vec<RepoStatus> {
    let mut repos = Vec::new();

    for root in root_dirs {
        let root_path = Path::new(root);
        if !root_path.exists() {
            continue;
        }

        let walker = WalkDir::new(root_path)
            .max_depth(4)
            .into_iter()
            .filter_entry(|entry| {
                let path = entry.path();
                if !path.is_dir() {
                    return false;
                }

                // Check directory name for ignored pattern (hidden dirs or build output)
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
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

                // Check if this directory is a git repository
                if path.join(".git").exists() {
                    if let Some(status) = inspect_repo(path) {
                        repos.push(status);
                    }
                    // Do not descend into this repository's subdirectories
                    return false;
                }

                true
            });

        // Consume the iterator to execute filter_entry walking
        for _ in walker {}
    }

    repos
}


fn inspect_repo(path: &Path) -> Option<RepoStatus> {
    let name = path.file_name()?.to_string_lossy().to_string();
    let path_str = path.to_string_lossy().to_string();

    let timeout = Duration::from_secs(3);

    // git status --porcelain -> count of changed files
    let mut cmd_status = Command::new("git");
    cmd_status.args(["status", "--porcelain"]).current_dir(path);
    let changed_files = match run_cmd_with_timeout(cmd_status, timeout) {
        Some(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count(),
        None => 0,
    };

    // current branch name
    let mut cmd_branch = Command::new("git");
    cmd_branch.args(["rev-parse", "--abbrev-ref", "HEAD"]).current_dir(path);
    let branch = match run_cmd_with_timeout(cmd_branch, timeout) {
        Some(out) if out.status.success() => {
            let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if b.is_empty() {
                "unknown".to_string()
            } else {
                b
            }
        }
        _ => "unknown".to_string(),
    };

    // unpushed commits: commits in HEAD not in the upstream branch
    let mut cmd_unpushed = Command::new("git");
    cmd_unpushed.args(["log", "@{u}..HEAD", "--oneline"]).current_dir(path);
    let unpushed_commits = match run_cmd_with_timeout(cmd_unpushed, timeout) {
        Some(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count(),
        _ => 0,
    };

    Some(RepoStatus {
        name,
        path: path_str,
        dirty: changed_files > 0,
        changed_files,
        unpushed_commits,
        branch,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_projects() {
        let repos = scan_git_repos(&["C:\\projects".to_string()]);
        println!("FOUND REPOS ({})", repos.len());
        for r in &repos {
            println!("  - {} (branch: {}, dirty: {}, changed: {})", r.name, r.branch, r.dirty, r.changed_files);
        }
    }
}


