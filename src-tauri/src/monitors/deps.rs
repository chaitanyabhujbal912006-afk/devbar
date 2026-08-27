use crate::monitors::common::collect_repo_paths;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Curated list of packages we care about for the dashboard.
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
    /// key = package name, value = version string (without ^ ~ = prefix)
    pub deps: HashMap<String, String>,
}

/// Read package.json from every git repo in `dirs` and return per-repo
/// dependency maps, limited to the curated INTERESTING_DEPS list.
pub fn get_dep_versions(dirs: &[String]) -> Vec<RepoDeps> {
    let mut result = Vec::new();

    for (repo_name, repo_path) in collect_repo_paths(dirs) {
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
            result.push(RepoDeps { repo: repo_name, path: repo_path, deps });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_dep_versions() {
        let dirs = vec!["C:\\projects".to_string()];
        let deps = get_dep_versions(&dirs);
        println!("FOUND DEPS FOR REPOS: {}", deps.len());
        for d in &deps {
            println!("  - Repo: {} with {} packages", d.repo, d.deps.len());
        }
    }
}
