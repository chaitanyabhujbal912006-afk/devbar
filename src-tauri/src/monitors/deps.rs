use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Curated list of packages we care about for the dashboard.
/// Only packages present in ≥2 repos will be shown in the UI.
const INTERESTING_DEPS: &[&str] = &[
    "next",
    "react",
    "react-dom",
    "vue",
    "vite",
    "typescript",
    "tailwindcss",
    "framer-motion",
    "@supabase/supabase-js",
    "@supabase/ssr",
    "prisma",
    "@prisma/client",
    "express",
    "fastify",
    "astro",
    "nuxt",
    "svelte",
    "remix",
    "trpc",
    "@trpc/server",
    "zod",
    "axios",
    "drizzle-orm",
];

/// Dependency versions for one repo.
#[derive(Serialize, Clone)]
pub struct RepoDeps {
    pub repo: String,
    pub path: String,
    /// key = package name, value = version string (may include ^ ~ etc.)
    pub deps: HashMap<String, String>,
}

/// Read package.json from every git repo in `dirs` and return per-repo
/// dependency maps, limited to the curated INTERESTING_DEPS list.
pub fn get_dep_versions(dirs: &[String]) -> Vec<RepoDeps> {
    let repos = collect_repo_paths(dirs);
    let mut result = Vec::new();

    for (repo_name, repo_path) in repos {
        let pkg_path = Path::new(&repo_path).join("package.json");
        if !pkg_path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&pkg_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let mut deps: HashMap<String, String> = HashMap::new();

        // Merge dependencies + devDependencies
        for key in &["dependencies", "devDependencies"] {
            if let Some(obj) = json.get(key).and_then(|v| v.as_object()) {
                for (pkg, ver) in obj {
                    if INTERESTING_DEPS.contains(&pkg.as_str()) {
                        let ver_str = ver
                            .as_str()
                            .unwrap_or("?")
                            // Strip semver prefix chars for cleaner display
                            .trim_start_matches('^')
                            .trim_start_matches('~')
                            .trim_start_matches('=')
                            .to_string();
                        deps.insert(pkg.clone(), ver_str);
                    }
                }
            }
        }

        if !deps.is_empty() {
            result.push(RepoDeps {
                repo: repo_name,
                path: repo_path,
                deps,
            });
        }
    }

    result
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
