use crate::db::Db;
use crate::models::RepoProfile;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Profile a single repository: detect languages, frameworks, package managers,
/// scripts, CI systems, README, and license.
pub fn profile_repo(repo_id: &str, worktree_path: &str, db: &Db) -> Result<RepoProfile, String> {
    let path = Path::new(worktree_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Repo path does not exist: {}", worktree_path));
    }

    // Collect all top-level and one-level-deep file names for fast detection
    let file_set = list_files(path, 2);

    let languages = detect_languages(&file_set, path);
    let package_managers = detect_package_managers(&file_set);
    let frameworks = detect_frameworks(&file_set, path, &languages);
    let scripts = detect_scripts(&file_set, path);
    let ci_systems = detect_ci(&file_set);
    let has_readme = detect_readme(&file_set);
    let (has_license, license_type) = detect_license(&file_set, path);

    let profile = RepoProfile {
        id: uuid::Uuid::new_v4().to_string(),
        repo_id: repo_id.to_string(),
        languages,
        frameworks,
        package_managers,
        scripts,
        ci_systems,
        has_readme,
        has_license,
        license_type,
        detected_at: chrono::Utc::now().to_rfc3339(),
    };

    upsert_profile(&profile, db)?;

    // Update project_asset.primary_language
    if let Some(primary) = profile.languages.first() {
        update_primary_language(repo_id, primary, db)?;
    }

    Ok(profile)
}

/// List file names in the directory up to `depth` levels, returning relative file names.
fn list_files(root: &Path, depth: u8) -> HashSet<String> {
    let mut files = HashSet::new();
    collect_files(root, root, depth, &mut files);
    files
}

fn collect_files(base: &Path, current: &Path, depth: u8, out: &mut HashSet<String>) {
    if depth == 0 {
        return;
    }
    if let Ok(entries) = fs::read_dir(current) {
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            if let Ok(relative) = entry.path().strip_prefix(base) {
                out.insert(relative.to_string_lossy().to_string());
            }
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                // Skip known noise directories
                if file_name == "node_modules"
                    || file_name == ".git"
                    || file_name == "target"
                    || file_name == "__pycache__"
                    || file_name == ".next"
                    || file_name == "dist"
                    || file_name == "build"
                    || file_name == ".cache"
                {
                    continue;
                }
                collect_files(base, &entry.path(), depth - 1, out);
            }
        }
    }
}

fn detect_languages(files: &HashSet<String>, path: &Path) -> Vec<String> {
    let mut languages = Vec::new();

    // TypeScript / JavaScript
    let has_ts = files.iter().any(|f| f == "tsconfig.json");
    let has_package_json = files.contains("package.json");
    if has_ts {
        languages.push("TypeScript".into());
    }
    if has_package_json && !has_ts {
        languages.push("JavaScript".into());
    }

    // Rust
    if files.contains("Cargo.toml") {
        languages.push("Rust".into());
    }

    // Python
    if files.contains("pyproject.toml")
        || files.contains("setup.py")
        || files.contains("setup.cfg")
        || files.contains("requirements.txt")
        || files.contains("Pipfile")
    {
        languages.push("Python".into());
    }

    // Go
    if files.contains("go.mod") {
        languages.push("Go".into());
    }

    // Java / Kotlin
    if files.contains("pom.xml")
        || files.contains("build.gradle")
        || files.contains("build.gradle.kts")
    {
        languages.push("Java".into());
    }
    if files
        .iter()
        .any(|f| f.ends_with(".kt") || f == "build.gradle.kts")
    {
        // Check for .kt files in src
        if path.join("src").join("main").join("kotlin").exists()
            || files.iter().any(|f| f.contains(".kt"))
        {
            if !languages.contains(&"Java".into()) {
                languages.push("Kotlin".into());
            }
        }
    }

    // C/C++
    if files.contains("CMakeLists.txt")
        || files.contains("Makefile")
        || files.contains("meson.build")
    {
        if files.contains("CMakeLists.txt") && has_cpp_sources(path) {
            languages.push("C++".into());
        } else {
            languages.push("C".into());
        }
    }

    // Ruby
    if files.contains("Gemfile") || files.contains(".ruby-version") {
        languages.push("Ruby".into());
    }

    // PHP
    if files.contains("composer.json") {
        languages.push("PHP".into());
    }

    // Swift / Objective-C
    if files.contains("Package.swift") {
        languages.push("Swift".into());
    }

    // .NET / C#
    if files
        .iter()
        .any(|f| f.ends_with(".csproj") || f.ends_with(".sln"))
    {
        languages.push("C#".into());
    }

    // Dart/Flutter
    if files.contains("pubspec.yaml") {
        languages.push("Dart".into());
    }

    // Shell
    if files.iter().any(|f| f.ends_with(".sh")) {
        languages.push("Shell".into());
    }

    // Fallback: if no language detected, look at file extensions in root
    if languages.is_empty() {
        if files.iter().any(|f| f.ends_with(".py")) {
            languages.push("Python".into());
        }
        if files.iter().any(|f| f.ends_with(".rs")) {
            languages.push("Rust".into());
        }
        if files.iter().any(|f| f.ends_with(".go")) {
            languages.push("Go".into());
        }
    }

    languages
}

fn has_cpp_sources(path: &Path) -> bool {
    // Quick check: look for .cpp/.cxx/.cc in src/
    let src = path.join("src");
    let check_dir = |dir: &Path| -> bool {
        fs::read_dir(dir)
            .map(|entries| {
                entries.filter_map(|e| e.ok()).any(|e| {
                    let name = e.file_name().to_string_lossy().to_string();
                    name.ends_with(".cpp")
                        || name.ends_with(".cxx")
                        || name.ends_with(".cc")
                        || name.ends_with(".hpp")
                })
            })
            .unwrap_or(false)
    };
    check_dir(path) || check_dir(&src)
}

fn detect_package_managers(files: &HashSet<String>) -> Vec<String> {
    let mut managers = Vec::new();

    if files.contains("package.json") {
        if files.contains("pnpm-lock.yaml") {
            managers.push("pnpm".into());
        } else if files.contains("yarn.lock") {
            managers.push("yarn".into());
        } else if files.contains("bun.lockb") || files.contains("bun.lock") {
            managers.push("bun".into());
        } else {
            managers.push("npm".into());
        }
    }

    if files.contains("Cargo.toml") {
        managers.push("cargo".into());
    }

    if files.contains("pyproject.toml") {
        // Could be poetry, pdm, etc.
        if files.contains("poetry.lock") {
            managers.push("poetry".into());
        } else if files.contains("pdm.lock") {
            managers.push("pdm".into());
        } else {
            managers.push("pip".into());
        }
    } else if files.contains("requirements.txt") {
        managers.push("pip".into());
    } else if files.contains("Pipfile") {
        managers.push("pipenv".into());
    }

    if files.contains("go.mod") {
        managers.push("go_modules".into());
    }

    if files.contains("pom.xml") {
        managers.push("maven".into());
    } else if files.contains("build.gradle") || files.contains("build.gradle.kts") {
        managers.push("gradle".into());
    }

    if files.contains("Gemfile") {
        managers.push("bundler".into());
    }

    if files.contains("composer.json") {
        managers.push("composer".into());
    }

    if files.contains("pubspec.yaml") {
        managers.push("pub".into());
    }

    managers
}

fn detect_frameworks(files: &HashSet<String>, path: &Path, languages: &[String]) -> Vec<String> {
    let mut frameworks = Vec::new();

    // Read package.json for JS frameworks
    if files.contains("package.json") {
        if let Ok(pkg) = read_package_json(path) {
            let deps = pkg.get("dependencies").and_then(|d| d.as_object());
            let dev_deps = pkg.get("devDependencies").and_then(|d| d.as_object());

            let all_deps: Vec<String> = deps
                .unwrap_or(&serde_json::Map::new())
                .keys()
                .chain(dev_deps.unwrap_or(&serde_json::Map::new()).keys())
                .cloned()
                .collect();

            if all_deps
                .iter()
                .any(|d| d == "react" || d.starts_with("@react"))
            {
                if all_deps.iter().any(|d| d == "next") {
                    frameworks.push("Next.js".into());
                } else if all_deps.iter().any(|d| d == "remix") {
                    frameworks.push("Remix".into());
                } else {
                    frameworks.push("React".into());
                }
            }
            if all_deps
                .iter()
                .any(|d| d == "vue" || d.starts_with("@vue/"))
            {
                if all_deps.iter().any(|d| d == "nuxt") {
                    frameworks.push("Nuxt".into());
                } else {
                    frameworks.push("Vue".into());
                }
            }
            if all_deps
                .iter()
                .any(|d| d == "svelte" || d.starts_with("@sveltejs/"))
            {
                frameworks.push("Svelte".into());
            }
            if all_deps
                .iter()
                .any(|d| d == "angular" || d.starts_with("@angular/"))
            {
                frameworks.push("Angular".into());
            }
            if all_deps.iter().any(|d| d == "express") {
                frameworks.push("Express".into());
            }
            if all_deps.iter().any(|d| d == "fastify") {
                frameworks.push("Fastify".into());
            }
            if all_deps.iter().any(|d| d == "@tauri-apps/api") {
                frameworks.push("Tauri".into());
            }
            if all_deps.iter().any(|d| d == "electron") {
                frameworks.push("Electron".into());
            }
            if all_deps.iter().any(|d| d == "vite")
                || all_deps.iter().any(|d| d.starts_with("@vitejs/"))
            {
                frameworks.push("Vite".into());
            }
            if all_deps.iter().any(|d| d == "tailwindcss") {
                frameworks.push("Tailwind CSS".into());
            }
        }
    }

    // Rust frameworks
    if languages.contains(&"Rust".into()) {
        if let Ok(cargo_content) = fs::read_to_string(path.join("Cargo.toml")) {
            if cargo_content.contains("actix-web") {
                frameworks.push("Actix Web".into());
            }
            if cargo_content.contains("axum") {
                frameworks.push("Axum".into());
            }
            if cargo_content.contains("rocket") {
                frameworks.push("Rocket".into());
            }
            if cargo_content.contains("tokio") {
                frameworks.push("Tokio".into());
            }
            if cargo_content.contains("tauri") {
                if !frameworks.contains(&"Tauri".into()) {
                    frameworks.push("Tauri".into());
                }
            }
            if cargo_content.contains("wasm-bindgen") || cargo_content.contains("yew") {
                frameworks.push("Yew/WASM".into());
            }
        }
    }

    // Python frameworks
    if languages.contains(&"Python".into()) {
        if let Ok(pyproject) = fs::read_to_string(path.join("pyproject.toml")) {
            let lower = pyproject.to_lowercase();
            if lower.contains("django") {
                frameworks.push("Django".into());
            }
            if lower.contains("flask") {
                frameworks.push("Flask".into());
            }
            if lower.contains("fastapi") {
                frameworks.push("FastAPI".into());
            }
            if lower.contains("scrapy") {
                frameworks.push("Scrapy".into());
            }
        }
        if let Ok(reqs) = fs::read_to_string(path.join("requirements.txt")) {
            let lower = reqs.to_lowercase();
            if lower.contains("django") && !frameworks.contains(&"Django".into()) {
                frameworks.push("Django".into());
            }
            if lower.contains("flask") && !frameworks.contains(&"Flask".into()) {
                frameworks.push("Flask".into());
            }
            if lower.contains("fastapi") && !frameworks.contains(&"FastAPI".into()) {
                frameworks.push("FastAPI".into());
            }
        }
    }

    // Go frameworks
    if languages.contains(&"Go".into()) {
        if let Ok(go_mod) = fs::read_to_string(path.join("go.mod")) {
            if go_mod.contains("gin-gonic/gin") {
                frameworks.push("Gin".into());
            }
            if go_mod.contains("labstack/echo") || go_mod.contains("echo") {
                frameworks.push("Echo".into());
            }
            if go_mod.contains("fiber") {
                frameworks.push("Fiber".into());
            }
        }
    }

    frameworks
}

fn detect_scripts(files: &HashSet<String>, path: &Path) -> serde_json::Value {
    let mut scripts = serde_json::Map::new();

    // package.json scripts
    if files.contains("package.json") {
        if let Ok(pkg) = read_package_json(path) {
            if let Some(pkg_scripts) = pkg.get("scripts").and_then(|s| s.as_object()) {
                for (key, value) in pkg_scripts {
                    if let Some(v) = value.as_str() {
                        scripts.insert(
                            format!("npm:{}", key),
                            serde_json::Value::String(v.to_string()),
                        );
                    }
                }
            }
        }
    }

    // Cargo.toml - no standard scripts, but we can detect just/Makefile
    if files.contains("justfile") {
        scripts.insert(
            "build:just".into(),
            serde_json::Value::String("just".into()),
        );
    }
    if files.contains("Makefile") {
        scripts.insert(
            "build:make".into(),
            serde_json::Value::String("make".into()),
        );
    }

    // Tox for Python
    if files.contains("tox.ini") {
        scripts.insert("python:tox".into(), serde_json::Value::String("tox".into()));
    }

    serde_json::Value::Object(scripts)
}

fn detect_ci(files: &HashSet<String>) -> Vec<String> {
    let mut ci = Vec::new();

    if files.contains(".github/workflows")
        || files.iter().any(|f| f.starts_with(".github/workflows/"))
    {
        ci.push("GitHub Actions".into());
    }
    if files.contains(".gitlab-ci.yml") {
        ci.push("GitLab CI".into());
    }
    if files.contains(".circleci") || files.iter().any(|f| f.starts_with(".circleci/")) {
        ci.push("CircleCI".into());
    }
    if files.contains("Jenkinsfile") {
        ci.push("Jenkins".into());
    }
    if files.contains(".travis.yml") {
        ci.push("Travis CI".into());
    }
    if files.contains("azure-pipelines.yml") || files.contains(".azure-pipelines.yml") {
        ci.push("Azure Pipelines".into());
    }
    if files.contains("bitbucket-pipelines.yml") {
        ci.push("Bitbucket Pipelines".into());
    }

    ci
}

fn detect_readme(files: &HashSet<String>) -> bool {
    files.iter().any(|f| {
        let lower = f.to_lowercase();
        lower == "readme.md" || lower == "readme" || lower == "readme.txt" || lower == "readme.rst"
    })
}

fn detect_license(files: &HashSet<String>, path: &Path) -> (bool, Option<String>) {
    let has_license = files.iter().any(|f| {
        let lower = f.to_lowercase();
        lower == "license"
            || lower == "license.md"
            || lower == "license.txt"
            || lower == "licence"
            || lower == "licence.md"
            || lower == "licence.txt"
            || lower == "copying"
            || lower == "copying.md"
            || lower == "copying.txt"
    });

    let license_type = if has_license {
        // Try to detect license type from content
        detect_license_type(path)
    } else {
        None
    };

    (has_license, license_type)
}

fn detect_license_type(path: &Path) -> Option<String> {
    // Check for license files
    let candidates = [
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "LICENCE",
        "LICENCE.md",
        "COPYING",
        "COPYING.txt",
    ];
    for name in &candidates {
        let license_path = path.join(name);
        if let Ok(content) = fs::read_to_string(&license_path) {
            let lower = content.to_lowercase();
            if lower.contains("mit license") || lower.contains("mit license") {
                return Some("MIT".into());
            }
            if lower.contains("apache license") || lower.contains("apache-2.0") {
                return Some("Apache-2.0".into());
            }
            if lower.contains("gnu general public license") {
                if lower.contains("version 3") || lower.contains("gplv3") {
                    return Some("GPL-3.0".into());
                }
                if lower.contains("version 2") || lower.contains("gplv2") {
                    return Some("GPL-2.0".into());
                }
                return Some("GPL".into());
            }
            if lower.contains("bsd license")
                || lower.contains("bsd 3-clause")
                || lower.contains("bsd 2-clause")
            {
                if lower.contains("3-clause") {
                    return Some("BSD-3-Clause".into());
                }
                return Some("BSD-2-Clause".into());
            }
            if lower.contains("isc license") {
                return Some("ISC".into());
            }
            if lower.contains("mozilla public license") {
                return Some("MPL-2.0".into());
            }
            if lower.contains("unlicense") {
                return Some("Unlicense".into());
            }
        }
    }

    // Also check package.json for license field
    if let Ok(pkg) = read_package_json(path) {
        if let Some(lic) = pkg.get("license").and_then(|l| l.as_str()) {
            return Some(lic.to_string());
        }
    }

    None
}

/// Read and parse package.json
fn read_package_json(path: &Path) -> Result<serde_json::Value, String> {
    let content = fs::read_to_string(path.join("package.json"))
        .map_err(|e| format!("Cannot read package.json: {}", e))?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid package.json: {}", e))
}

/// Update the primary_language field on the project_asset table
fn update_primary_language(repo_id: &str, language: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE project_asset SET primary_language = ?1 WHERE id = (SELECT asset_id FROM repository WHERE id = ?2)",
        rusqlite::params![language, repo_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Upsert a repo profile into the database
fn upsert_profile(profile: &RepoProfile, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let languages_json = serde_json::to_string(&profile.languages).unwrap_or_else(|_| "[]".into());
    let frameworks_json =
        serde_json::to_string(&profile.frameworks).unwrap_or_else(|_| "[]".into());
    let package_managers_json =
        serde_json::to_string(&profile.package_managers).unwrap_or_else(|_| "[]".into());
    let scripts_json = profile.scripts.to_string();
    let ci_systems_json =
        serde_json::to_string(&profile.ci_systems).unwrap_or_else(|_| "[]".into());

    conn.execute(
        "INSERT INTO repo_profile (id, repo_id, languages, frameworks, package_managers, scripts, ci_systems, has_readme, has_license, license_type, detected_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(repo_id) DO UPDATE SET
            languages = excluded.languages,
            frameworks = excluded.frameworks,
            package_managers = excluded.package_managers,
            scripts = excluded.scripts,
            ci_systems = excluded.ci_systems,
            has_readme = excluded.has_readme,
            has_license = excluded.has_license,
            license_type = excluded.license_type,
            detected_at = excluded.detected_at",
        rusqlite::params![
            profile.id,
            profile.repo_id,
            languages_json,
            frameworks_json,
            package_managers_json,
            scripts_json,
            ci_systems_json,
            profile.has_readme as i32,
            profile.has_license as i32,
            profile.license_type,
            profile.detected_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Load a repo profile from the database
pub fn load_profile(repo_id: &str, db: &Db) -> Result<Option<RepoProfile>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let result = conn.query_row(
        "SELECT id, repo_id, languages, frameworks, package_managers, scripts, ci_systems, has_readme, has_license, license_type, detected_at
         FROM repo_profile WHERE repo_id = ?1",
        rusqlite::params![repo_id],
        |row| {
            let languages_str: String = row.get(2)?;
            let frameworks_str: String = row.get(3)?;
            let pm_str: String = row.get(4)?;
            let scripts_str: String = row.get(5)?;
            let ci_str: String = row.get(6)?;

            Ok(RepoProfile {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                languages: serde_json::from_str(&languages_str).unwrap_or_default(),
                frameworks: serde_json::from_str(&frameworks_str).unwrap_or_default(),
                package_managers: serde_json::from_str(&pm_str).unwrap_or_default(),
                scripts: serde_json::from_str(&scripts_str).unwrap_or_default(),
                ci_systems: serde_json::from_str(&ci_str).unwrap_or_default(),
                has_readme: row.get::<_, i32>(7)? != 0,
                has_license: row.get::<_, i32>(8)? != 0,
                license_type: row.get(9)?,
                detected_at: row.get(10)?,
            })
        },
    );

    match result {
        Ok(profile) => Ok(Some(profile)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Load all repo profiles
pub fn load_all_profiles(db: &Db) -> Result<Vec<RepoProfile>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, repo_id, languages, frameworks, package_managers, scripts, ci_systems, has_readme, has_license, license_type, detected_at FROM repo_profile")
        .map_err(|e| e.to_string())?;

    let profiles = stmt
        .query_map([], |row| {
            let languages_str: String = row.get(2)?;
            let frameworks_str: String = row.get(3)?;
            let pm_str: String = row.get(4)?;
            let scripts_str: String = row.get(5)?;
            let ci_str: String = row.get(6)?;

            Ok(RepoProfile {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                languages: serde_json::from_str(&languages_str).unwrap_or_default(),
                frameworks: serde_json::from_str(&frameworks_str).unwrap_or_default(),
                package_managers: serde_json::from_str(&pm_str).unwrap_or_default(),
                scripts: serde_json::from_str(&scripts_str).unwrap_or_default(),
                ci_systems: serde_json::from_str(&ci_str).unwrap_or_default(),
                has_readme: row.get::<_, i32>(7)? != 0,
                has_license: row.get::<_, i32>(8)? != 0,
                license_type: row.get(9)?,
                detected_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_ts_project() {
        let mut files = HashSet::new();
        files.insert("package.json".into());
        files.insert("tsconfig.json".into());
        let langs = detect_languages(&files, Path::new("/nonexistent"));
        assert!(langs.contains(&"TypeScript".to_string()));
    }

    #[test]
    fn test_detect_rust_project() {
        let mut files = HashSet::new();
        files.insert("Cargo.toml".into());
        let langs = detect_languages(&files, Path::new("/nonexistent"));
        assert!(langs.contains(&"Rust".to_string()));
    }

    #[test]
    fn test_detect_python_project() {
        let mut files = HashSet::new();
        files.insert("pyproject.toml".into());
        let langs = detect_languages(&files, Path::new("/nonexistent"));
        assert!(langs.contains(&"Python".to_string()));
    }

    #[test]
    fn test_detect_pnpm() {
        let mut files = HashSet::new();
        files.insert("package.json".into());
        files.insert("pnpm-lock.yaml".into());
        let pms = detect_package_managers(&files);
        assert!(pms.contains(&"pnpm".to_string()));
    }

    #[test]
    fn test_detect_github_actions() {
        let mut files = HashSet::new();
        files.insert(".github/workflows/ci.yml".into());
        let ci = detect_ci(&files);
        assert!(ci.contains(&"GitHub Actions".to_string()));
    }

    #[test]
    fn test_detect_readme() {
        let mut files = HashSet::new();
        files.insert("README.md".into());
        assert!(detect_readme(&files));
    }

    #[test]
    fn test_detect_license_file() {
        let mut files = HashSet::new();
        files.insert("LICENSE".into());
        let (has, _) = detect_license(&files, Path::new("/nonexistent"));
        assert!(has);
    }
}
