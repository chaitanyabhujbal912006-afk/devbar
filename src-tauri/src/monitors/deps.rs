use crate::monitors::common::collect_repo_paths;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

/// Curated list of JS/TS packages we care about for the dashboard.
const INTERESTING_JS_DEPS: &[&str] = &[
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
    /// "js" | "rust" | "python" | "go"
    pub lang: String,
    /// key = package name, value = version string (without ^ ~ = prefix)
    pub deps: HashMap<String, String>,
}

/// Read package manifests from every git repo in `dirs` and return per-repo
/// dependency maps. Supports package.json, Cargo.toml, pyproject.toml/requirements.txt, go.mod.
pub fn get_dep_versions(dirs: &[String]) -> Vec<RepoDeps> {
    let mut result = Vec::new();

    for (repo_name, repo_path) in collect_repo_paths(dirs) {
        let repo_p = Path::new(&repo_path);

        // ── JavaScript / TypeScript ──────────────────────────────────────────
        let pkg_path = repo_p.join("package.json");
        if pkg_path.exists() {
            if let Some(rd) = parse_package_json(&pkg_path, &repo_name, &repo_path) {
                result.push(rd);
            }
        }

        // ── Rust / Cargo ─────────────────────────────────────────────────────
        let cargo_path = repo_p.join("Cargo.toml");
        if cargo_path.exists() {
            if let Some(rd) = parse_cargo_toml(&cargo_path, &repo_name, &repo_path) {
                result.push(rd);
            }
        }

        // ── Python ───────────────────────────────────────────────────────────
        let pyproject_path = repo_p.join("pyproject.toml");
        let requirements_path = repo_p.join("requirements.txt");
        if pyproject_path.exists() {
            if let Some(rd) = parse_pyproject_toml(&pyproject_path, &repo_name, &repo_path) {
                result.push(rd);
            }
        } else if requirements_path.exists() {
            if let Some(rd) = parse_requirements_txt(&requirements_path, &repo_name, &repo_path) {
                result.push(rd);
            }
        }

        // ── Go ───────────────────────────────────────────────────────────────
        let go_mod_path = repo_p.join("go.mod");
        if go_mod_path.exists() {
            if let Some(rd) = parse_go_mod(&go_mod_path, &repo_name, &repo_path) {
                result.push(rd);
            }
        }
    }

    result
}

// ─── Parsers ─────────────────────────────────────────────────────────────────

fn parse_package_json(path: &Path, repo: &str, repo_path: &str) -> Option<RepoDeps> {
    let content = std::fs::read_to_string(path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;

    let mut deps: HashMap<String, String> = HashMap::new();
    for key in &["dependencies", "devDependencies"] {
        if let Some(obj) = json.get(key).and_then(|v| v.as_object()) {
            for (pkg, ver) in obj {
                if INTERESTING_JS_DEPS.contains(&pkg.as_str()) {
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

    if deps.is_empty() { return None; }
    Some(RepoDeps { repo: repo.into(), path: repo_path.into(), lang: "js".into(), deps })
}

fn parse_cargo_toml(path: &Path, repo: &str, repo_path: &str) -> Option<RepoDeps> {
    let content = std::fs::read_to_string(path).ok()?;
    let toml: toml::Value = toml::from_str(&content).ok()?;

    let mut deps: HashMap<String, String> = HashMap::new();

    let gather = |table: &toml::Value, out: &mut HashMap<String, String>| {
        if let Some(t) = table.as_table() {
            for (pkg, ver) in t {
                let ver_str = match ver {
                    toml::Value::String(s) => s
                        .trim_start_matches('^')
                        .trim_start_matches('~')
                        .trim_start_matches('=')
                        .to_string(),
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("?")
                        .trim_start_matches('^')
                        .trim_start_matches('~')
                        .trim_start_matches('=')
                        .to_string(),
                    _ => return,
                };
                out.insert(pkg.clone(), ver_str);
            }
        }
    };

    if let Some(d) = toml.get("dependencies") { gather(d, &mut deps); }
    if let Some(d) = toml.get("dev-dependencies") { gather(d, &mut deps); }
    if let Some(d) = toml.get("build-dependencies") { gather(d, &mut deps); }

    if deps.is_empty() { return None; }
    Some(RepoDeps { repo: repo.into(), path: repo_path.into(), lang: "rust".into(), deps })
}

fn parse_pyproject_toml(path: &Path, repo: &str, repo_path: &str) -> Option<RepoDeps> {
    let content = std::fs::read_to_string(path).ok()?;
    let toml: toml::Value = toml::from_str(&content).ok()?;

    let mut deps: HashMap<String, String> = HashMap::new();

    // PEP 621: [project].dependencies = ["requests>=2.28", ...]
    if let Some(arr) = toml
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        for dep in arr {
            if let Some(s) = dep.as_str() {
                // e.g. "requests>=2.28.0" or "flask"
                let pkg = s
                    .split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_' && c != '.')
                    .next()
                    .unwrap_or(s)
                    .to_lowercase();
                let ver = s.trim_start_matches(&pkg).trim_start_matches(|c: char| !c.is_alphanumeric() && c != '.').to_string();
                deps.insert(pkg, if ver.is_empty() { "*".into() } else { ver });
            }
        }
    }

    // Poetry: [tool.poetry.dependencies]
    if let Some(table) = toml
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        for (pkg, ver) in table {
            if pkg == "python" { continue; }
            let ver_str = match ver {
                toml::Value::String(s) => s
                    .trim_start_matches('^')
                    .trim_start_matches('~')
                    .trim_start_matches('=')
                    .trim_start_matches('>')
                    .to_string(),
                _ => "*".into(),
            };
            deps.insert(pkg.clone(), ver_str);
        }
    }

    if deps.is_empty() { return None; }
    Some(RepoDeps { repo: repo.into(), path: repo_path.into(), lang: "python".into(), deps })
}

fn parse_requirements_txt(path: &Path, repo: &str, repo_path: &str) -> Option<RepoDeps> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut deps: HashMap<String, String> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }
        // e.g. "requests==2.28.0" or "flask>=2.0" or just "numpy"
        let (pkg, ver) = if let Some(pos) = line.find(|c: char| c == '=' || c == '>' || c == '<' || c == '~' || c == '!') {
            let ver_raw = line[pos..].trim_start_matches(|c: char| !c.is_alphanumeric() && c != '.');
            (line[..pos].to_lowercase(), ver_raw.to_string())
        } else {
            (line.to_lowercase(), "*".to_string())
        };
        deps.insert(pkg, ver);
    }

    if deps.is_empty() { return None; }
    Some(RepoDeps { repo: repo.into(), path: repo_path.into(), lang: "python".into(), deps })
}

fn parse_go_mod(path: &Path, repo: &str, repo_path: &str) -> Option<RepoDeps> {
    let content = std::fs::read_to_string(path).ok()?;
    let mut deps: HashMap<String, String> = HashMap::new();
    let mut in_require = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("require (") || line == "require (" {
            in_require = true;
            continue;
        }
        if in_require && line == ")" {
            in_require = false;
            continue;
        }
        if in_require {
            // e.g. "github.com/gin-gonic/gin v1.9.1"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let pkg = parts[0].to_string();
                let ver = parts[1].trim_start_matches('v').to_string();
                deps.insert(pkg, ver);
            }
        } else if line.starts_with("require ") {
            // single-line: "require github.com/foo/bar v1.0.0"
            let rest = &line["require ".len()..];
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                deps.insert(parts[0].to_string(), parts[1].trim_start_matches('v').to_string());
            }
        }
    }

    if deps.is_empty() { return None; }
    Some(RepoDeps { repo: repo.into(), path: repo_path.into(), lang: "go".into(), deps })
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
            println!("  - Repo: {} [{}] with {} packages", d.repo, d.lang, d.deps.len());
        }
    }
}
