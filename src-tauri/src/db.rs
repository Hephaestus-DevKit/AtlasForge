use rusqlite::{Connection, Result as SqlResult};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

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

pub struct Db {
    pub conn: Mutex<Connection>,
}

impl Db {
    pub fn new(db_path: &PathBuf) -> SqlResult<Self> {
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut conn = Connection::open(db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

        // Run migrations in order
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
        ];

        // Track applied migrations
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS _migration (
                id INTEGER PRIMARY KEY NOT NULL,
                applied_at TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )?;

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
        })
    }
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
            assert_eq!(applied, 13);
        }
        drop(db);

        let reopened = Db::new(&path).unwrap();
        let conn = reopened.conn.lock().unwrap();
        let applied: i64 = conn
            .query_row("SELECT COUNT(*) FROM _migration", [], |row| row.get(0))
            .unwrap();
        assert_eq!(applied, 13);
    }
}
