use serde::Serialize;
use std::path::Path;
use std::process::Command;
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

    // git status --porcelain -> count of changed files
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .ok()?;
    let changed_files = String::from_utf8_lossy(&status_output.stdout)
        .lines()
        .filter(|l| !l.trim().is_empty())
        .count();

    // current branch name
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .ok()?;
    let branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // unpushed commits: commits in HEAD not in the upstream branch
    let unpushed_output = Command::new("git")
        .args(["log", "@{u}..HEAD", "--oneline"])
        .current_dir(path)
        .output();
    let unpushed_commits = match unpushed_output {
        Ok(out) => String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter(|l| !l.trim().is_empty())
            .count(),
        Err(_) => 0, // no upstream configured
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
