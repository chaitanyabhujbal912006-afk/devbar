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

/// Scans the given root directories (non-recursive into node_modules/.git internals)
/// for folders containing a `.git` directory, and reports their status.
pub fn scan_git_repos(root_dirs: &[String]) -> Vec<RepoStatus> {
    let mut repos = Vec::new();

    for root in root_dirs {
        if !Path::new(root).exists() {
            continue;
        }

        // Only walk 2 levels deep (root/project/.git) to keep this fast.
        for entry in WalkDir::new(root)
            .max_depth(2)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_dir() && path.join(".git").exists() {
                if let Some(status) = inspect_repo(path) {
                    repos.push(status);
                }
            }
        }
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
    let status_output = run_cmd_with_timeout(cmd_status, timeout)?;
    let changed_files = String::from_utf8_lossy(&status_output.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();

    // current branch name
    let mut cmd_branch = Command::new("git");
    cmd_branch.args(["rev-parse", "--abbrev-ref", "HEAD"]).current_dir(path);
    let branch_output = run_cmd_with_timeout(cmd_branch, timeout)?;
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // unpushed commits: commits in HEAD not in the upstream branch
    let mut cmd_unpushed = Command::new("git");
    cmd_unpushed.args(["log", "@{u}..HEAD", "--oneline"]).current_dir(path);
    let unpushed_output = run_cmd_with_timeout(cmd_unpushed, timeout);
    let unpushed_commits = match unpushed_output {
        Some(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count(),
        None => 0, // no upstream configured or timed out
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
