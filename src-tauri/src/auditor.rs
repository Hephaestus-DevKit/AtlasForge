use crate::db::Db;
use crate::models::*;
use crate::profiler::load_profile;
use std::path::Path;

/// Check whether a repo has no remote origin (local-only).
fn is_local_repo(path: &Path) -> bool {
    std::process::Command::new("git")
        .args(&["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(true) // If git fails entirely, treat as local
        == false
}

/// Run a health audit on a repository.
pub fn audit_repo(
    repo_id: &str,
    worktree_path: &str,
    scan_id: Option<&str>,
    db: &Db,
) -> Result<HealthSnapshot, String> {
    let path = Path::new(worktree_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Repo path does not exist: {}", worktree_path));
    }

    let profile = load_profile(repo_id, db).unwrap_or(None);
    let local_only = is_local_repo(path);

    // Run all checkers
    let mut all_findings: Vec<Finding> = Vec::new();
    let mut category_scores: std::collections::HashMap<String, CategoryScore> =
        std::collections::HashMap::new();

    // Runnable
    let runnable = check_runnable(path, &profile);
    all_findings.extend(runnable.findings.clone());
    category_scores.insert("runnable".into(), runnable);

    // Tests
    let tests = check_tests(path, &profile);
    all_findings.extend(tests.findings.clone());
    category_scores.insert("tests".into(), tests);

    // CI
    let ci = check_ci(path, &profile, local_only);
    all_findings.extend(ci.findings.clone());
    category_scores.insert("ci".into(), ci);

    // Docs
    let docs = check_docs(path, &profile, local_only);
    all_findings.extend(docs.findings.clone());
    category_scores.insert("docs".into(), docs);

    // Dependencies
    let deps = check_dependencies(path, &profile);
    all_findings.extend(deps.findings.clone());
    category_scores.insert("dependencies".into(), deps);

    // Security
    let security = check_security(path, &profile);
    all_findings.extend(security.findings.clone());
    category_scores.insert("security".into(), security);

    // Release
    let release = check_release(path, &profile, local_only);
    all_findings.extend(release.findings.clone());
    category_scores.insert("release".into(), release);

    // Git hygiene
    let git_hygiene = check_git_hygiene(path);
    all_findings.extend(git_hygiene.findings.clone());
    category_scores.insert("git_hygiene".into(), git_hygiene);

    // Public surface
    let public_surface = check_public_surface(path, &profile);
    all_findings.extend(public_surface.findings.clone());
    category_scores.insert("public_surface".into(), public_surface);

    // Platform compat
    let platform = check_platform_compat(path, &profile);
    all_findings.extend(platform.findings.clone());
    category_scores.insert("platform_compat".into(), platform);

    // Calculate overall score
    let total_weight: f64 = category_scores.values().map(|c| c.weight).sum();
    let weighted_sum: f64 = category_scores
        .values()
        .map(|c| c.score as f64 * c.weight)
        .sum();
    let overall_score = if total_weight > 0.0 {
        (weighted_sum / total_weight) as i32
    } else {
        0
    };

    // Recommended tasks
    let recommended_tasks = generate_recommended_tasks(&all_findings);

    // Save to database
    let snapshot = HealthSnapshot {
        id: uuid::Uuid::new_v4().to_string(),
        repo_id: repo_id.to_string(),
        scan_id: scan_id.map(|s| s.to_string()),
        score: overall_score,
        category_scores: serde_json::to_string(&category_scores).unwrap_or_default(),
        recommended_tasks: serde_json::to_string(&recommended_tasks).unwrap_or_default(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    save_snapshot(&snapshot, &all_findings, db)?;

    Ok(snapshot)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthSnapshot {
    pub id: String,
    pub repo_id: String,
    pub scan_id: Option<String>,
    pub score: i32,
    pub category_scores: String,
    pub recommended_tasks: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryScore {
    pub score: i32,
    pub max_score: i32,
    pub weight: f64,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub id: String,
    pub category: String,
    pub severity: String,
    pub title: String,
    pub description: String,
    pub evidence: String,
    pub file_path: Option<String>,
    pub suggested_fix: Option<String>,
    pub auto_fixable: bool,
}

// --- Checkers ---

fn check_runnable(path: &Path, profile: &Option<RepoProfile>) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 100;

    // Check for build/run scripts
    if let Some(p) = profile {
        let has_run_script = p.scripts.as_object().map_or(false, |obj| {
            obj.keys()
                .any(|k| k == "start" || k == "dev" || k == "build" || k == "run")
        });

        if !has_run_script {
            score -= 30;
            findings.push(Finding {
                id: uuid::Uuid::new_v4().to_string(),
                category: "runnable".into(),
                severity: "warning".into(),
                title: "No standard run/build script found".into(),
                description: "Could not find start, dev, build, or run script in package manifest."
                    .into(),
                evidence: format!("Scripts: {:?}", p.scripts),
                file_path: None,
                suggested_fix: Some(
                    "Add a 'start' or 'build' script to your package manifest.".into(),
                ),
                auto_fixable: false,
            });
        }

        // Check for Cargo.toml with missing main
        if p.languages.contains(&"Rust".to_string()) {
            if !path.join("src").join("main.rs").exists()
                && !path.join("src").join("lib.rs").exists()
            {
                score -= 20;
                findings.push(Finding {
                    id: uuid::Uuid::new_v4().to_string(),
                    category: "runnable".into(),
                    severity: "warning".into(),
                    title: "Rust project missing main.rs or lib.rs".into(),
                    description: "Standard Rust entry point not found.".into(),
                    evidence: "No src/main.rs or src/lib.rs".into(),
                    file_path: Some("src/".into()),
                    suggested_fix: Some("Create src/main.rs or src/lib.rs.".into()),
                    auto_fixable: false,
                });
            }
        }
    } else {
        score -= 40;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "runnable".into(),
            severity: "info".into(),
            title: "No project profile available".into(),
            description: "Cannot assess runnability without a profile. Run a scan first.".into(),
            evidence: String::new(),
            file_path: None,
            suggested_fix: None,
            auto_fixable: false,
        });
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 1.5,
        findings,
    }
}

fn check_tests(path: &Path, profile: &Option<RepoProfile>) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 50; // Start at 50 (neutral - tests may not be expected)

    let has_test_dir = path.join("test").exists()
        || path.join("tests").exists()
        || path.join("__tests__").exists()
        || path.join("spec").exists();
    let has_test_files = has_test_dir
        || walkdir::WalkDir::new(path)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
            .any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name.contains(".test.") || name.contains(".spec.") || name.contains("_test.")
            });

    if has_test_files {
        score = 90;
    } else if let Some(p) = profile {
        let has_test_script = p
            .scripts
            .as_object()
            .map_or(false, |obj| obj.keys().any(|k| k.contains("test")));
        if has_test_script {
            score = 70;
        } else {
            score = 30;
            findings.push(Finding {
                id: uuid::Uuid::new_v4().to_string(),
                category: "tests".into(),
                severity: "warning".into(),
                title: "No test infrastructure detected".into(),
                description: "No test directory, test files, or test script found.".into(),
                evidence: String::new(),
                file_path: None,
                suggested_fix: Some("Add a test framework and write basic tests.".into()),
                auto_fixable: false,
            });
        }
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 1.2,
        findings,
    }
}

fn check_ci(path: &Path, profile: &Option<RepoProfile>, local_only: bool) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score;

    let has_github_actions = path.join(".github").join("workflows").exists();
    let has_gitlab_ci = path.join(".gitlab-ci.yml").exists();
    let has_circleci = path.join(".circleci").exists();
    let has_travis = path.join(".travis.yml").exists();
    let has_jenkins = path.join("Jenkinsfile").exists();

    if has_github_actions || has_gitlab_ci || has_circleci || has_travis || has_jenkins {
        score = 90;
        // Check for basic CI quality
        if has_github_actions {
            if let Ok(entries) = std::fs::read_dir(path.join(".github").join("workflows")) {
                let count = entries.count();
                if count == 0 {
                    score = 60;
                    findings.push(Finding {
                        id: uuid::Uuid::new_v4().to_string(),
                        category: "ci".into(),
                        severity: "warning".into(),
                        title: "GitHub Actions directory exists but is empty".into(),
                        description:
                            "The .github/workflows directory exists but contains no workflow files."
                                .into(),
                        evidence: String::new(),
                        file_path: Some(".github/workflows/".into()),
                        suggested_fix: Some("Add a CI workflow file.".into()),
                        auto_fixable: false,
                    });
                }
            }
        }
    } else if local_only {
        // Local-only repos may not need CI; reduce severity
        score = 60;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "ci".into(),
            severity: "info".into(),
            title: "No CI configuration detected (local-only repo)".into(),
            description: "No CI/CD pipeline found, but this is a local-only repository without a remote. CI is less critical for local projects.".into(),
            evidence: "No git remote origin configured".into(),
            file_path: None,
            suggested_fix: Some("If this repo will be published, add CI before pushing to a remote.".into()),
            auto_fixable: false,
        });
    } else {
        score = 20;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "ci".into(),
            severity: "error".into(),
            title: "No CI configuration detected".into(),
            description: "No CI/CD pipeline configuration found. Automated testing and deployment are not set up.".into(),
            evidence: String::new(),
            file_path: None,
            suggested_fix: Some("Add a CI configuration (e.g. GitHub Actions workflow).".into()),
            auto_fixable: false,
        });
    }

    if let Some(p) = profile {
        if !p.ci_systems.is_empty() && score < 90 {
            score = 70;
        }
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 1.0,
        findings,
    }
}

fn check_docs(path: &Path, _profile: &Option<RepoProfile>, local_only: bool) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 100;

    let has_readme = path.join("README.md").exists()
        || path.join("README").exists()
        || path.join("readme.md").exists();
    let has_changelog = path.join("CHANGELOG.md").exists() || path.join("CHANGES.md").exists();
    let has_contributing = path.join("CONTRIBUTING.md").exists();
    let _has_api_docs = path.join("docs").exists() || path.join("doc").exists();

    if !has_readme {
        score -= 40;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "docs".into(),
            severity: if local_only { "warning" } else { "error" }.into(),
            title: "Missing README".into(),
            description: if local_only {
                "No README.md found. Consider adding one even for local projects to document purpose and usage.".into()
            } else {
                "No README.md found. This is the first thing users see.".into()
            },
            evidence: String::new(),
            file_path: None,
            suggested_fix: Some("Add a README.md with project description, installation, and usage.".into()),
            auto_fixable: false,
        });
    } else {
        // Check README quality
        if let Ok(content) = std::fs::read_to_string(path.join("README.md")) {
            if content.lines().count() < 5 {
                score -= 20;
                findings.push(Finding {
                    id: uuid::Uuid::new_v4().to_string(),
                    category: "docs".into(),
                    severity: "warning".into(),
                    title: "README is too short".into(),
                    description: "README has fewer than 5 lines. Consider expanding it.".into(),
                    evidence: format!("{} lines", content.lines().count()),
                    file_path: Some("README.md".into()),
                    suggested_fix: Some(
                        "Add installation instructions, usage examples, and API documentation."
                            .into(),
                    ),
                    auto_fixable: false,
                });
            }
        }
    }

    if !has_changelog {
        score -= 15;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "docs".into(),
            severity: "info".into(),
            title: "Missing CHANGELOG".into(),
            description: "No changelog file found. Users cannot track version changes.".into(),
            evidence: String::new(),
            file_path: None,
            suggested_fix: Some("Add a CHANGELOG.md to track version history.".into()),
            auto_fixable: false,
        });
    }

    if !has_contributing {
        score -= 10;
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 1.0,
        findings,
    }
}

fn check_dependencies(path: &Path, profile: &Option<RepoProfile>) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 80;

    // Check for lock files
    let has_lock = path.join("package-lock.json").exists()
        || path.join("yarn.lock").exists()
        || path.join("pnpm-lock.yaml").exists()
        || path.join("Cargo.lock").exists()
        || path.join("poetry.lock").exists()
        || path.join("go.sum").exists();

    if !has_lock {
        score -= 30;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "dependencies".into(),
            severity: "warning".into(),
            title: "No lock file detected".into(),
            description: "No dependency lock file found. Builds may not be reproducible.".into(),
            evidence: String::new(),
            file_path: None,
            suggested_fix: Some("Commit your dependency lock file for reproducible builds.".into()),
            auto_fixable: false,
        });
    }

    // Check for outdated/known vulnerable packages would require network access
    // For now, just check structure
    if let Some(p) = profile {
        if p.package_managers.is_empty() {
            score -= 10;
        }
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 0.8,
        findings,
    }
}

fn check_security(path: &Path, profile: &Option<RepoProfile>) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 90;

    // Check for .env files (should not be committed)
    let env_files = [".env", ".env.local", ".env.production", ".env.staging"];
    for env_file in &env_files {
        let env_path = path.join(env_file);
        if env_path.exists() {
            // Check if .gitignore covers it
            let gitignore_path = path.join(".gitignore");
            let gitignore_covers = if let Ok(content) = std::fs::read_to_string(&gitignore_path) {
                content
                    .lines()
                    .any(|line| line.trim() == *env_file || line.trim() == ".env*")
            } else {
                false
            };

            if !gitignore_covers {
                score -= 20;
                findings.push(Finding {
                    id: uuid::Uuid::new_v4().to_string(),
                    category: "security".into(),
                    severity: "critical".into(),
                    title: format!("{} may be committed to git", env_file),
                    description: "Environment file found that may not be covered by .gitignore."
                        .into(),
                    evidence: format!(
                        "File exists: {}, .gitignore coverage: {}",
                        env_file, gitignore_covers
                    ),
                    file_path: Some(env_file.to_string()),
                    suggested_fix: Some(format!(
                        "Add '{}' to .gitignore and rotate any exposed secrets.",
                        env_file
                    )),
                    auto_fixable: true,
                });
            }
        }
    }

    // Check for .gitignore
    if !path.join(".gitignore").exists() {
        score -= 15;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "security".into(),
            severity: "warning".into(),
            title: "No .gitignore file".into(),
            description:
                "No .gitignore found. Build artifacts and secrets may be accidentally committed."
                    .into(),
            evidence: String::new(),
            file_path: None,
            suggested_fix: Some("Add a .gitignore file appropriate for your project type.".into()),
            auto_fixable: true,
        });
    }

    // Check for license
    if let Some(p) = profile {
        if !p.has_license {
            score -= 10;
            findings.push(Finding {
                id: uuid::Uuid::new_v4().to_string(),
                category: "security".into(),
                severity: "warning".into(),
                title: "No license file".into(),
                description:
                    "No open-source license detected. This may limit how others can use the code."
                        .into(),
                evidence: String::new(),
                file_path: None,
                suggested_fix: Some(
                    "Add an appropriate license file (e.g. MIT, Apache-2.0).".into(),
                ),
                auto_fixable: false,
            });
        }
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 1.5,
        findings,
    }
}

fn check_release(path: &Path, _profile: &Option<RepoProfile>, local_only: bool) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 50;

    // Check for version in package.json / Cargo.toml
    let has_version = path.join("package.json").exists()
        || path.join("Cargo.toml").exists()
        || path.join("pyproject.toml").exists();

    if has_version {
        score = 80;
    } else {
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "release".into(),
            severity: if local_only { "info" } else { "warning" }.into(),
            title: "No standard version manifest".into(),
            description: if local_only {
                "No standard package manifest with version info found. This is acceptable for personal/local projects but recommended for published code.".into()
            } else {
                "Could not find a standard package manifest with version info.".into()
            },
            evidence: String::new(),
            file_path: None,
            suggested_fix: if local_only { None } else { Some("Add a package.json, Cargo.toml, or pyproject.toml with a version field.".into()) },
            auto_fixable: false,
        });
    }

    // Check for tags
    let has_tags = std::process::Command::new("git")
        .args(&["tag"])
        .current_dir(path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    if has_tags {
        score = 90;
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 0.7,
        findings,
    }
}

fn check_git_hygiene(path: &Path) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 90;

    // Check for large files in git
    let is_dirty = std::process::Command::new("git")
        .args(&["status", "--porcelain"])
        .current_dir(path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    if is_dirty {
        score -= 5;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "git_hygiene".into(),
            severity: "info".into(),
            title: "Working tree has uncommitted changes".into(),
            description: "The working tree contains uncommitted modifications.".into(),
            evidence: String::new(),
            file_path: None,
            suggested_fix: None,
            auto_fixable: false,
        });
    }

    // Check for stashes
    let stash_count = std::process::Command::new("git")
        .args(&["stash", "list"])
        .current_dir(path)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
        .unwrap_or(0);

    if stash_count > 5 {
        score -= 10;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "git_hygiene".into(),
            severity: "info".into(),
            title: "Many git stashes accumulated".into(),
            description: format!("{} stash entries found. Consider cleaning up.", stash_count),
            evidence: format!("{} stashes", stash_count),
            file_path: None,
            suggested_fix: Some("Review and apply or drop old stashes.".into()),
            auto_fixable: false,
        });
    }

    CategoryScore {
        score: score.max(0),
        max_score: 100,
        weight: 0.5,
        findings,
    }
}

fn check_public_surface(path: &Path, profile: &Option<RepoProfile>) -> CategoryScore {
    let findings = Vec::new();
    let mut score = 70;

    if let Some(p) = profile {
        if p.has_readme {
            score += 10;
        }
        if p.has_license {
            score += 10;
        }
        if !p.ci_systems.is_empty() {
            score += 5;
        }
    }

    // Check for GitHub-specific files
    if path.join(".github").exists() {
        score += 5;
    }

    CategoryScore {
        score: score.min(100),
        max_score: 100,
        weight: 0.6,
        findings,
    }
}

fn check_platform_compat(path: &Path, _profile: &Option<RepoProfile>) -> CategoryScore {
    let mut findings = Vec::new();
    let mut score = 80;

    // Check for cross-platform configs
    let has_editorconfig = path.join(".editorconfig").exists();
    let _has_nvmrc = path.join(".nvmrc").exists() || path.join(".node-version").exists();
    let _has_rust_toolchain =
        path.join("rust-toolchain.toml").exists() || path.join("rust-toolchain").exists();
    let has_docker = path.join("Dockerfile").exists() || path.join("docker-compose.yml").exists();

    if !has_editorconfig {
        score -= 5;
        findings.push(Finding {
            id: uuid::Uuid::new_v4().to_string(),
            category: "platform_compat".into(),
            severity: "info".into(),
            title: "No .editorconfig file".into(),
            description: "Without .editorconfig, different editors may use different formatting."
                .into(),
            evidence: String::new(),
            file_path: None,
            suggested_fix: Some(
                "Add a .editorconfig for consistent formatting across editors.".into(),
            ),
            auto_fixable: true,
        });
    }

    if has_docker {
        score += 10;
    }

    CategoryScore {
        score: score.min(100),
        max_score: 100,
        weight: 0.4,
        findings,
    }
}

fn generate_recommended_tasks(findings: &[Finding]) -> Vec<serde_json::Value> {
    findings
        .iter()
        .filter(|f| f.severity == "error" || f.severity == "critical" || f.severity == "warning")
        .map(|f| {
            let priority = match f.severity.as_str() {
                "critical" => "high",
                "error" => "high",
                "warning" => "medium",
                _ => "low",
            };
            serde_json::json!({
                "title": f.title,
                "category": f.category,
                "priority": priority,
                "description": f.description,
                "severity": f.severity,
                "autoFixable": f.auto_fixable,
            })
        })
        .collect()
}

fn save_snapshot(snapshot: &HealthSnapshot, findings: &[Finding], db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Delete old snapshots for this repo (keep only latest)
    conn.execute("DELETE FROM finding WHERE snapshot_id IN (SELECT id FROM repo_health_snapshot WHERE repo_id = ?1)", rusqlite::params![snapshot.repo_id])
        .map_err(|e| e.to_string())?;
    conn.execute(
        "DELETE FROM repo_health_snapshot WHERE repo_id = ?1",
        rusqlite::params![snapshot.repo_id],
    )
    .map_err(|e| e.to_string())?;

    // Insert new snapshot
    conn.execute(
        "INSERT INTO repo_health_snapshot (id, repo_id, scan_id, score, category_scores, recommended_tasks, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![
            snapshot.id,
            snapshot.repo_id,
            snapshot.scan_id,
            snapshot.score,
            snapshot.category_scores,
            snapshot.recommended_tasks,
            snapshot.created_at,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Insert findings
    for finding in findings {
        conn.execute(
            "INSERT INTO finding (id, snapshot_id, category, severity, title, description, evidence, file_path, suggested_fix, auto_fixable) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                finding.id,
                snapshot.id,
                finding.category,
                finding.severity,
                finding.title,
                finding.description,
                finding.evidence,
                finding.file_path,
                finding.suggested_fix,
                finding.auto_fixable as i32,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Load the latest health snapshot for a repo.
pub fn load_latest_snapshot(repo_id: &str, db: &Db) -> Result<Option<HealthSnapshot>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let result = conn.query_row(
        "SELECT id, repo_id, scan_id, score, category_scores, recommended_tasks, created_at FROM repo_health_snapshot WHERE repo_id = ?1 ORDER BY created_at DESC LIMIT 1",
        rusqlite::params![repo_id],
        |row| {
            Ok(HealthSnapshot {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                scan_id: row.get(2)?,
                score: row.get(3)?,
                category_scores: row.get(4)?,
                recommended_tasks: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    );

    match result {
        Ok(snapshot) => Ok(Some(snapshot)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Load a specific health snapshot and verify repository ownership.
pub fn load_snapshot(
    snapshot_id: &str,
    repo_id: &str,
    db: &Db,
) -> Result<HealthSnapshot, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, repo_id, scan_id, score, category_scores, recommended_tasks, created_at
         FROM repo_health_snapshot
         WHERE id = ?1 AND repo_id = ?2",
        rusqlite::params![snapshot_id, repo_id],
        |row| {
            Ok(HealthSnapshot {
                id: row.get(0)?,
                repo_id: row.get(1)?,
                scan_id: row.get(2)?,
                score: row.get(3)?,
                category_scores: row.get(4)?,
                recommended_tasks: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    )
    .map_err(|e| format!("Health snapshot not found for repository: {}", e))
}

/// Load findings for a snapshot.
pub fn load_findings(snapshot_id: &str, db: &Db) -> Result<Vec<Finding>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, category, severity, title, description, evidence, file_path, suggested_fix, auto_fixable FROM finding WHERE snapshot_id = ?1 ORDER BY severity, category")
        .map_err(|e| e.to_string())?;

    let findings = stmt
        .query_map(rusqlite::params![snapshot_id], |row| {
            Ok(Finding {
                id: row.get(0)?,
                category: row.get(1)?,
                severity: row.get(2)?,
                title: row.get(3)?,
                description: row.get(4)?,
                evidence: row.get(5)?,
                file_path: row.get(6)?,
                suggested_fix: row.get(7)?,
                auto_fixable: row.get::<_, i32>(8)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper: create a temp dir with a minimal project structure
    fn make_project(_name: &str, files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, content) in files {
            let full = dir.path().join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full, content).unwrap();
        }
        dir
    }

    #[test]
    fn check_docs_local_only_downgrades_severity() {
        // A project with no README, local-only: severity should be "warning" not "error"
        let dir = make_project(
            "no-readme",
            &[("Cargo.toml", "[package]\nname=\"x\"\nversion=\"0.1.0\"\n")],
        );
        let result = check_docs(dir.path(), &None, true);
        let readme_finding = result
            .findings
            .iter()
            .find(|f| f.title.contains("README"))
            .unwrap();
        assert_eq!(readme_finding.severity, "warning");

        // Same project, not local: severity should be "error"
        let result2 = check_docs(dir.path(), &None, false);
        let readme_finding2 = result2
            .findings
            .iter()
            .find(|f| f.title.contains("README"))
            .unwrap();
        assert_eq!(readme_finding2.severity, "error");
    }

    #[test]
    fn check_ci_local_only_gets_info_not_error() {
        // No CI at all, local-only repo
        let dir = make_project("no-ci-local", &[("src/main.rs", "fn main(){}\n")]);
        let result = check_ci(dir.path(), &None, true);
        let ci_finding = result
            .findings
            .iter()
            .find(|f| f.title.contains("CI"))
            .unwrap();
        assert_eq!(ci_finding.severity, "info");
        assert!(ci_finding.title.contains("local-only"));
        assert!(result.score >= 50); // Should be 60 for local-only

        // Same project, not local: should be error severity
        let result2 = check_ci(dir.path(), &None, false);
        let ci_finding2 = result2
            .findings
            .iter()
            .find(|f| f.title.contains("CI"))
            .unwrap();
        assert_eq!(ci_finding2.severity, "error");
        assert!(result2.score < 30); // Should be 20
    }

    #[test]
    fn check_ci_with_github_actions_scores_high() {
        let dir = make_project(
            "with-ci",
            &[(".github/workflows/ci.yml", "name: CI\non: push\n")],
        );
        let result = check_ci(dir.path(), &None, false);
        assert_eq!(result.score, 90);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn check_docs_with_good_readme_scores_high() {
        let readme_content = "# My Project\n\nA great project.\n\n## Install\n\nRun `cargo install`.\n\n## Usage\n\nJust run it.";
        let dir = make_project(
            "good-docs",
            &[
                ("README.md", readme_content),
                ("CHANGELOG.md", "# Changelog\n## 0.1.0\n- Initial release\n"),
                ("CONTRIBUTING.md", "# Contributing\nPRs welcome!\n"),
            ],
        );
        let result = check_docs(dir.path(), &None, false);
        assert!(result.score >= 90);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn check_release_local_only_downgrades_severity() {
        // No package manifest, local-only
        let dir = make_project("no-manifest", &[("src/main.rs", "fn main(){}\n")]);
        let result = check_release(dir.path(), &None, true);
        let finding = result
            .findings
            .iter()
            .find(|f| f.title.contains("version manifest"))
            .unwrap();
        assert_eq!(finding.severity, "info");

        // Not local: should be "warning"
        let result2 = check_release(dir.path(), &None, false);
        let finding2 = result2
            .findings
            .iter()
            .find(|f| f.title.contains("version manifest"))
            .unwrap();
        assert_eq!(finding2.severity, "warning");
    }

    #[test]
    fn category_score_max_score_is_100() {
        let dir = make_project(
            "basic",
            &[
                ("README.md", "# Test\n\nA test project.\n"),
                (
                    "package.json",
                    "{\"name\":\"test\",\"version\":\"1.0.0\"}\n",
                ),
            ],
        );
        let runnable = check_runnable(dir.path(), &None);
        assert_eq!(runnable.max_score, 100);
        let tests = check_tests(dir.path(), &None);
        assert_eq!(tests.max_score, 100);
        let ci = check_ci(dir.path(), &None, false);
        assert_eq!(ci.max_score, 100);
        let docs = check_docs(dir.path(), &None, false);
        assert_eq!(docs.max_score, 100);
    }

    #[test]
    fn generate_recommended_tasks_filters_info() {
        let findings = vec![
            Finding {
                id: "1".into(),
                category: "ci".into(),
                severity: "error".into(),
                title: "No CI".into(),
                description: "No CI found".into(),
                evidence: String::new(),
                file_path: None,
                suggested_fix: Some("Add CI".into()),
                auto_fixable: false,
            },
            Finding {
                id: "2".into(),
                category: "docs".into(),
                severity: "info".into(),
                title: "Missing CHANGELOG".into(),
                description: "No changelog".into(),
                evidence: String::new(),
                file_path: None,
                suggested_fix: None,
                auto_fixable: false,
            },
            Finding {
                id: "3".into(),
                category: "security".into(),
                severity: "warning".into(),
                title: "Large file".into(),
                description: "File over 10MB".into(),
                evidence: "big.bin".into(),
                file_path: Some("big.bin".into()),
                suggested_fix: None,
                auto_fixable: false,
            },
        ];
        let tasks = generate_recommended_tasks(&findings);
        assert_eq!(tasks.len(), 2); // info finding filtered out
        assert_eq!(tasks[0]["priority"], "high"); // error -> high
        assert_eq!(tasks[1]["priority"], "medium"); // warning -> medium
    }

    #[test]
    fn finding_has_all_required_fields() {
        let dir = make_project("no-ci", &[]);
        let result = check_ci(dir.path(), &None, false);
        let finding = &result.findings[0];
        assert!(!finding.id.is_empty());
        assert!(!finding.category.is_empty());
        assert!(!finding.severity.is_empty());
        assert!(!finding.title.is_empty());
        assert!(!finding.description.is_empty());
        // evidence may be empty for some findings, but the field exists
        // file_path is Option<String>
        // suggested_fix is Option<String>
        // auto_fixable is bool
    }

    #[test]
    fn check_tests_detects_test_dir() {
        let dir = make_project(
            "with-tests",
            &[
                ("tests/integration_test.rs", "#[test]\nfn test_it() {}\n"),
                (
                    "package.json",
                    "{\"name\":\"test\",\"scripts\":{\"test\":\"vitest\"}}\n",
                ),
            ],
        );
        let result = check_tests(dir.path(), &None);
        assert!(result.score >= 80);
    }

    #[test]
    fn check_docs_short_readme_gets_warning() {
        let dir = make_project("short-readme", &[("README.md", "# Hi\n")]);
        let result = check_docs(dir.path(), &None, false);
        let short_finding = result.findings.iter().find(|f| f.title.contains("short"));
        assert!(short_finding.is_some());
        assert!(result.score < 80);
    }
}
