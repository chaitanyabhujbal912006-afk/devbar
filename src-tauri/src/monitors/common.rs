use std::collections::HashSet;
use std::path::Path;
use walkdir::WalkDir;

/// Returns all git repository root paths found under the given `dirs`,
/// up to 4 directory levels deep.
/// Skips hidden dirs, `node_modules`, `target`, `dist`, `build`, `vendor`.
/// Each repo path is returned at most once (deduplicated).
pub fn collect_repo_paths(dirs: &[String]) -> Vec<(String, String)> {
    let mut repos = Vec::new();
    let mut seen = HashSet::new();

    for root in dirs {
        let root_path = Path::new(root);
        if !root_path.exists() {
            continue;
        }

        let mut local_repos: Vec<(String, String)> = Vec::new();

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
                // If this folder is a git repo root, collect it and stop descending.
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

        // Consume the iterator to drive filter_entry walking.
        for _ in walker.flatten() {}

        for (name, path_str) in local_repos {
            if seen.insert(path_str.clone()) {
                repos.push((name, path_str));
            }
        }
    }

    repos
}
