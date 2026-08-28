use crate::monitors::common::collect_repo_paths;
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
pub struct EnvHealthIssue {
    pub repo_path: String,
    pub repo_name: String,
    pub issue_type: String, // "missing_env" or "unignored_secret"
    pub message: String,
    pub file_name: Option<String>,
    pub example_file: Option<String>,
}

/// Scans root directories for git repositories and inspects .env & secret health.
pub fn scan_env_health(root_dirs: &[String]) -> Vec<EnvHealthIssue> {
    let mut issues = Vec::new();
    let repos = collect_repo_paths(root_dirs);

    for (_, repo_path_str) in repos {
        let repo_path = Path::new(&repo_path_str);
        if !repo_path.exists() {
            continue;
        }

        let repo_name = repo_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| repo_path_str.clone());

        // 1. Check for missing .env file when example exists
        let example_candidates = [
            ".env.example",
            ".env.template",
            ".env.dist",
            ".env.sample",
        ];
        let has_env = repo_path.join(".env").exists() || repo_path.join(".env.local").exists();

        if !has_env {
            for example in &example_candidates {
                if repo_path.join(example).exists() {
                    issues.push(EnvHealthIssue {
                        repo_path: repo_path_str.clone(),
                        repo_name: repo_name.clone(),
                        issue_type: "missing_env".to_string(),
                        message: format!("Found {} but missing local .env file", example),
                        file_name: Some(".env".to_string()),
                        example_file: Some(example.to_string()),
                    });
                    break;
                }
            }
        }

        // 2. Check for un-ignored secret files
        let secret_candidates = [
            ".env",
            ".env.local",
            "id_rsa",
            "credentials.json",
            "service-account.json",
        ];

        let gitignore_path = repo_path.join(".gitignore");
        let gitignore_content = if gitignore_path.exists() {
            fs::read_to_string(&gitignore_path).unwrap_or_default()
        } else {
            String::new()
        };

        for secret in &secret_candidates {
            let secret_path = repo_path.join(secret);
            if secret_path.exists() {
                let is_ignored = is_file_ignored_in_gitignore(&gitignore_content, secret);
                if !is_ignored {
                    issues.push(EnvHealthIssue {
                        repo_path: repo_path_str.clone(),
                        repo_name: repo_name.clone(),
                        issue_type: "unignored_secret".to_string(),
                        message: format!("File '{}' is present but not in .gitignore", secret),
                        file_name: Some(secret.to_string()),
                        example_file: None,
                    });
                }
            }
        }
    }

    issues
}

fn is_file_ignored_in_gitignore(gitignore_content: &str, file_name: &str) -> bool {
    for line in gitignore_content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == file_name || trimmed == format!("/{}", file_name) || trimmed == format!("*.{}", file_name) {
            return true;
        }
    }
    false
}

/// Copies an example file (e.g. .env.example) to .env in the repository.
pub fn fix_missing_env(repo_path: &str, example_filename: &str) -> Result<(), String> {
    let base = Path::new(repo_path);
    let src = base.join(example_filename);
    let dest = base.join(".env");

    if !src.exists() {
        return Err(format!("Example file '{}' does not exist", example_filename));
    }

    fs::copy(&src, &dest)
        .map(|_| ())
        .map_err(|e| format!("Failed to copy file: {}", e))
}

/// Appends a file pattern to .gitignore in the repository.
pub fn add_to_gitignore(repo_path: &str, file_to_ignore: &str) -> Result<(), String> {
    let base = Path::new(repo_path);
    let gitignore_path = base.join(".gitignore");

    let mut content = if gitignore_path.exists() {
        fs::read_to_string(&gitignore_path).unwrap_or_default()
    } else {
        String::new()
    };

    if is_file_ignored_in_gitignore(&content, file_to_ignore) {
        return Ok(());
    }

    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(file_to_ignore);
    content.push('\n');

    fs::write(&gitignore_path, content).map_err(|e| format!("Failed to update .gitignore: {}", e))
}
