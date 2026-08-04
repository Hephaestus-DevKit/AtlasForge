use rusqlite::Connection;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sql(#[from] rusqlite::Error),
    #[error("database I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database integrity check failed: {0}")]
    Integrity(String),
}

pub const MIGRATION_001_SQL: &str = include_str!("../migrations/001_initial.sql");
pub const MIGRATION_002_SQL: &str = include_str!("../migrations/002_repo_profile.sql");
pub const MIGRATION_003_SQL: &str = include_str!("../migrations/003_job_engine.sql");
pub const MIGRATION_004_SQL: &str = include_str!("../migrations/004_tool_broker.sql");
pub const MIGRATION_005_SQL: &str = include_str!("../migrations/005_indexing.sql");
pub const MIGRATION_006_SQL: &str = include_str!("../migrations/006_audit_report.sql");
pub const MIGRATION_007_SQL: &str = include_str!("../migrations/007_ai_provider.sql");
pub const MIGRATION_008_SQL: &str = include_str!("../migrations/008_ai_fix.sql");
pub const MIGRATION_009_SQL: &str = include_str!("../migrations/009_github.sql");
pub const MIGRATION_010_SQL: &str = include_str!("../migrations/010_semantic.sql");
pub const MIGRATION_011_SQL: &str = include_str!("../migrations/011_automation.sql");
pub const MIGRATION_012_SQL: &str = include_str!("../migrations/012_scan_reliability.sql");
pub const MIGRATION_013_SQL: &str = include_str!("../migrations/013_verification_run.sql");
pub const MIGRATION_014_SQL: &str = include_str!("../migrations/014_trusted_execution.sql");
pub const MIGRATION_015_SQL: &str = include_str!("../migrations/015_incremental_index.sql");
pub const MIGRATION_016_SQL: &str = include_str!("../migrations/016_security_wording.sql");
pub const MIGRATION_017_SQL: &str = include_str!("../migrations/017_fts_trigger_cleanup.sql");
pub const MIGRATION_018_SQL: &str = include_str!("../migrations/018_asset_availability.sql");

pub struct Db {
    pub conn: Mutex<Connection>,
    pub path: PathBuf,
}

impl Db {
    pub fn new(db_path: &PathBuf) -> Result<Self, DbError> {
        let existed = db_path.is_file();
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut conn = Connection::open(db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;
        let integrity: String = conn.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(DbError::Integrity(integrity));
        }

        let migrations = [
            MIGRATION_001_SQL,
            MIGRATION_002_SQL,
            MIGRATION_003_SQL,
            MIGRATION_004_SQL,
            MIGRATION_005_SQL,
            MIGRATION_006_SQL,
            MIGRATION_007_SQL,
            MIGRATION_008_SQL,
            MIGRATION_009_SQL,
            MIGRATION_010_SQL,
            MIGRATION_011_SQL,
            MIGRATION_012_SQL,
            MIGRATION_013_SQL,
            MIGRATION_014_SQL,
            MIGRATION_015_SQL,
            MIGRATION_016_SQL,
            MIGRATION_017_SQL,
            MIGRATION_018_SQL,
        ];

        // Track applied migrations
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migration (
                id INTEGER PRIMARY KEY NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

        let applied_count: i64 =
            conn.query_row("SELECT COUNT(*) FROM _migration", [], |row| row.get(0))?;
        if existed && applied_count < migrations.len() as i64 {
            create_migration_backup(&conn, db_path)?;
            prune_backups(db_path, 3)?;
        }

        for (i, sql) in migrations.iter().enumerate() {
            let idx = (i + 1) as i64;
            let applied: i64 = conn
                .query_row(
                    "SELECT COUNT(*) FROM _migration WHERE id = ?1",
                    rusqlite::params![idx],
                    |row| row.get(0),
                )
                .unwrap_or(0);

            if applied == 0 {
                let tx = conn.transaction()?;
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO _migration (id) VALUES (?1)",
                    rusqlite::params![idx],
                )?;
                tx.commit()?;
                log::info!("Applied migration {:03}", idx);
            }
        }

        Ok(Self {
            conn: Mutex::new(conn),
            path: db_path.clone(),
        })
    }

    /// Open a new connection to the database file.
    /// This allows concurrent readers and writers in WAL mode without locking the main conn Mutex.
    pub fn connection(&self) -> Result<Connection, rusqlite::Error> {
        let conn = Connection::open(&self.path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
        )?;
        Ok(conn)
    }
}

fn create_migration_backup(source: &Connection, db_path: &Path) -> Result<PathBuf, DbError> {
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = db_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("atlasforge");
    let timestamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let backup_path = parent.join(format!("{}.pre-migration-{}.db", stem, timestamp));
    let mut destination = Connection::open(&backup_path)?;
    let backup = rusqlite::backup::Backup::new(source, &mut destination)?;
    backup.run_to_completion(16, Duration::from_millis(25), None)?;
    drop(backup);
    let integrity: String = destination.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        let _ = fs::remove_file(&backup_path);
        return Err(DbError::Integrity(format!(
            "migration backup is invalid: {}",
            integrity
        )));
    }
    log::info!("Created migration backup at {:?}", backup_path);
    Ok(backup_path)
}

fn prune_backups(db_path: &Path, retain: usize) -> Result<(), DbError> {
    let parent = db_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = db_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("atlasforge");
    let prefix = format!("{}.pre-migration-", stem);
    let mut backups = fs::read_dir(parent)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|value| value.to_str())
                .map(|name| name.starts_with(&prefix) && name.ends_with(".db"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    backups.sort();
    let remove_count = backups.len().saturating_sub(retain);
    for path in backups.into_iter().take(remove_count) {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_all_migrations_and_reopens_cleanly() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atlasforge.db");
        let db = Db::new(&path).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            let applied: i64 = conn
                .query_row("SELECT COUNT(*) FROM _migration", [], |row| row.get(0))
                .unwrap();
            assert_eq!(applied, 18);
        }
        drop(db);

        let reopened = Db::new(&path).unwrap();
        let conn = reopened.conn.lock().unwrap();
        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migration", [], |row| row.get(0))
            .unwrap();
        assert_eq!(applied, 18);
    }

    #[test]
    fn rejects_corrupt_database_files() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("atlasforge.db");
        fs::write(&path, b"not a sqlite database").unwrap();
        assert!(Db::new(&path).is_err());
    }
}
