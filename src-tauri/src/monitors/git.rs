use crate::monitors::common::collect_repo_paths;
use serde::Serialize;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::Duration;
use wait_timeout::ChildExt;

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

/// Scans the given root directories for git repositories and returns their status.
pub fn scan_git_repos(root_dirs: &[String]) -> Vec<RepoStatus> {
    collect_repo_paths(root_dirs)
        .into_iter()
        .filter_map(|(_, path_str)| inspect_repo(Path::new(&path_str)))
        .collect()
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
            if b.is_empty() { "unknown".to_string() } else { b }
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
