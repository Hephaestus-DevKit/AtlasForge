use crate::db::Db;
use crate::models::{AddRootInput, WorkspaceRoot};
use crate::security;

const MAX_ROOT_GLOBS: usize = 128;
const MAX_GLOB_LENGTH: usize = 512;

pub fn validate_root_settings(input: &AddRootInput) -> Result<(), String> {
    if !matches!(input.access_mode.as_str(), "read_only" | "read_write") {
        return Err("Access mode must be 'read_only' or 'read_write'".into());
    }

    for (kind, patterns) in [
        ("include", &input.include_globs),
        ("exclude", &input.exclude_globs),
    ] {
        if patterns.len() > MAX_ROOT_GLOBS {
            return Err(format!(
                "Too many {} globs: maximum is {}",
                kind, MAX_ROOT_GLOBS
            ));
        }
        for pattern in patterns {
            if pattern.trim().is_empty() {
                return Err(format!("{} glob must not be empty", kind));
            }
            if pattern.len() > MAX_GLOB_LENGTH {
                return Err(format!(
                    "{} glob exceeds {} characters",
                    kind, MAX_GLOB_LENGTH
                ));
            }
            glob::Pattern::new(&pattern.replace('\\', "/"))
                .map_err(|err| format!("Invalid {} glob '{}': {}", kind, pattern, err))?;
        }
    }
    Ok(())
}

pub fn effective_exclude_globs(input: &AddRootInput) -> Vec<String> {
    if input.exclude_globs.is_empty() {
        security::DEFAULT_EXCLUDE_GLOBS
            .iter()
            .map(|pattern| pattern.to_string())
            .collect()
    } else {
        input.exclude_globs.clone()
    }
}

pub fn load_workspace_roots(db: &Db) -> Result<Vec<WorkspaceRoot>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, path, label, access_mode, scan_enabled, include_globs, exclude_globs, created_at, last_scanned_at FROM workspace_root ORDER BY created_at")
        .map_err(|e| e.to_string())?;

    let roots = stmt
        .query_map([], |row| {
            let include_globs_str: String = row.get(5)?;
            let exclude_globs_str: String = row.get(6)?;
            Ok(WorkspaceRoot {
                id: row.get(0)?,
                path: row.get(1)?,
                label: row.get(2)?,
                access_mode: row.get(3)?,
                scan_enabled: row.get::<_, i32>(4)? != 0,
                include_globs: serde_json::from_str(&include_globs_str).unwrap_or_default(),
                exclude_globs: serde_json::from_str(&exclude_globs_str).unwrap_or_default(),
                created_at: row.get(7)?,
                last_scanned_at: row.get(8)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    Ok(roots)
}

pub fn repository_path(repo_id: &str, db: &Db) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|error| error.to_string())?;
    conn.query_row(
        "SELECT r.worktree_path FROM repository r
         JOIN project_asset a ON a.id = r.asset_id
         WHERE r.id = ?1 AND a.is_available = 1",
        rusqlite::params![repo_id],
        |row| row.get(0),
    )
    .map_err(|error| format!("Repository not found: {}", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_input(access_mode: &str, include_globs: Vec<String>) -> AddRootInput {
        AddRootInput {
            path: "C:/workspace".into(),
            label: "Workspace".into(),
            access_mode: access_mode.into(),
            scan_enabled: true,
            include_globs,
            exclude_globs: vec!["node_modules".into()],
        }
    }

    #[test]
    fn rejects_invalid_modes_and_globs() {
        assert!(validate_root_settings(&root_input("admin", vec![])).is_err());
        assert!(validate_root_settings(&root_input(
            "read_only",
            vec!["[unterminated".into()]
        ))
        .is_err());
        assert!(validate_root_settings(&root_input(
            "read_write",
            vec!["clients/**".into()]
        ))
        .is_ok());
    }
}
