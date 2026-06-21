use crate::db::Db;
use crate::models::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Default)]
pub struct JobRuntime {
    cancellations: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl JobRuntime {
    pub fn register(&self, job_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut cancellations) = self.cancellations.lock() {
            cancellations.insert(job_id.to_string(), flag.clone());
        }
        flag
    }

    pub fn cancel(&self, job_id: &str) {
        if let Ok(cancellations) = self.cancellations.lock() {
            if let Some(flag) = cancellations.get(job_id) {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }

    pub fn finish(&self, job_id: &str) {
        if let Ok(mut cancellations) = self.cancellations.lock() {
            cancellations.remove(job_id);
        }
    }
}

/// Create a queued job and return its ID.
pub fn create_job(job_type: &str, input: &str, db: &Db) -> Result<String, String> {
    let job_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO job (id, type, status, input, priority, progress, progress_total, retry_count, max_retries, created_at, updated_at) VALUES (?1, ?2, 'pending', ?3, 0, 0, 0, 0, 3, ?4, ?5)",
        rusqlite::params![job_id, job_type, input, now, now],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);
    append_job_event(
        job_id.as_str(),
        "job_created",
        &serde_json::json!({"jobType": job_type}).to_string(),
        db,
    )?;
    Ok(job_id)
}

pub fn begin_job(job_id: &str, db: &Db, runtime: &JobRuntime) -> Result<Arc<AtomicBool>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let changed = conn
        .execute(
            "UPDATE job
             SET status = 'running', updated_at = ?1, completed_at = NULL, error_message = NULL
             WHERE id = ?2 AND status = 'pending'",
            rusqlite::params![chrono::Utc::now().to_rfc3339(), job_id],
        )
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("Job is not pending and cannot be started".into());
    }
    drop(conn);
    let cancellation = runtime.register(job_id);
    append_job_event(job_id, "job_started", "{}", db)?;
    Ok(cancellation)
}

/// Append a typed event to a job's event timeline.
pub fn append_job_event(
    job_id: &str,
    event_type: &str,
    payload: &str,
    db: &Db,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let next_seq: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(seq), 0) + 1 FROM job_event WHERE job_id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .unwrap_or(1);

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO job_event (id, job_id, seq, type, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![id, job_id, next_seq, event_type, payload, now],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// Cancel a running or pending job and signal any active worker.
pub fn cancel_job(job_id: &str, db: &Db, runtime: &JobRuntime) -> Result<Job, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let status: String = conn
        .query_row(
            "SELECT status FROM job WHERE id = ?1",
            rusqlite::params![job_id],
            |row| row.get(0),
        )
        .map_err(|e| format!("Job not found: {}", e))?;

    if status != "pending" && status != "running" {
        return Err(format!("Cannot cancel job with status '{}'", status));
    }

    runtime.cancel(job_id);
    conn.execute(
        "UPDATE job SET status = 'cancelled', updated_at = datetime('now'), completed_at = datetime('now') WHERE id = ?1",
        rusqlite::params![job_id],
    )
    .map_err(|e| e.to_string())?;

    drop(conn);
    append_job_event(job_id, "job_cancelled", "{}", db)?;

    load_job(job_id, db)
}

/// Retry a failed or cancelled job by creating a queued job with the same input.
pub fn retry_job(job_id: &str, db: &Db) -> Result<Job, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let (job_type, input, status, retry_count, max_retries): (String, String, String, i64, i64) =
        conn.query_row(
            "SELECT type, input, status, retry_count, max_retries FROM job WHERE id = ?1",
            rusqlite::params![job_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .map_err(|e| format!("Job not found: {}", e))?;
    if status != "failed" && status != "cancelled" {
        return Err(format!("Cannot retry job with status '{}'", status));
    }
    if retry_count >= max_retries {
        return Err(format!("Job has reached its retry limit ({})", max_retries));
    }

    // Increment retry_count on original
    conn.execute(
        "UPDATE job SET retry_count = retry_count + 1 WHERE id = ?1",
        rusqlite::params![job_id],
    )
    .map_err(|e| e.to_string())?;

    drop(conn);

    // Create new job
    let new_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();

    let conn2 = db.conn.lock().map_err(|e| e.to_string())?;
    conn2
        .execute(
            "INSERT INTO job (id, type, status, input, priority, progress, progress_total, retry_count, max_retries, parent_job_id, created_at, updated_at) VALUES (?1, ?2, 'pending', ?3, 0, 0, 0, 0, 3, ?4, ?5, ?6)",
            rusqlite::params![new_id, job_type, input, job_id, now, now],
        )
        .map_err(|e| e.to_string())?;

    drop(conn2);
    append_job_event(
        &new_id,
        "job_created",
        &serde_json::json!({"retryOf": job_id}).to_string(),
        db,
    )?;

    load_job(&new_id, db)
}

/// Update job progress.
pub fn update_progress(job_id: &str, progress: i32, total: i32, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE job SET progress = ?1, progress_total = ?2, updated_at = datetime('now') WHERE id = ?3",
        rusqlite::params![progress, total, job_id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Mark a job as failed with error message.
pub fn fail_job(job_id: &str, error: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let changed = conn.execute(
        "UPDATE job SET status = 'failed', error_message = ?1, updated_at = datetime('now'), completed_at = datetime('now') WHERE id = ?2 AND status != 'cancelled'",
        rusqlite::params![error, job_id],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);
    if changed > 0 {
        append_job_event(
            job_id,
            "job_failed",
            &serde_json::json!({"error": error}).to_string(),
            db,
        )?;
    }
    Ok(())
}

/// Mark a job as completed.
pub fn complete_job(job_id: &str, db: &Db) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let changed = conn.execute(
        "UPDATE job SET status = 'completed', progress = progress_total, updated_at = datetime('now'), completed_at = datetime('now') WHERE id = ?1 AND status != 'cancelled'",
        rusqlite::params![job_id],
    )
    .map_err(|e| e.to_string())?;
    drop(conn);
    if changed > 0 {
        append_job_event(job_id, "job_completed", "{}", db)?;
    }
    Ok(())
}

pub fn recover_interrupted_jobs(db: &Db) -> Result<usize, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id FROM job WHERE status = 'running'")
        .map_err(|e| e.to_string())?;
    let ids = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);
    drop(conn);
    for id in &ids {
        fail_job(id, "Interrupted by the previous application shutdown", db)?;
        append_job_event(id, "job_interrupted", "{}", db)?;
    }
    Ok(ids.len())
}

/// Load a single job by ID.
pub fn load_job(job_id: &str, db: &Db) -> Result<Job, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        "SELECT id, type, status, input, created_at, updated_at, completed_at, error_message, progress, progress_total, parent_job_id FROM job WHERE id = ?1",
        rusqlite::params![job_id],
        |row| {
            Ok(Job {
                id: row.get(0)?,
                job_type: row.get(1)?,
                status: row.get(2)?,
                input: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                completed_at: row.get(6)?,
                error_message: row.get(7)?,
                progress: row.get::<_, i32>(8)?,
                progress_total: row.get::<_, i32>(9)?,
                parent_job_id: row.get(10)?,
            })
        },
    )
    .map_err(|e| e.to_string())
}

/// List recent jobs with optional type filter.
pub fn list_jobs_by_type(job_type: &str, limit: i64, db: &Db) -> Result<Vec<Job>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, type, status, input, created_at, updated_at, completed_at, error_message, progress, progress_total, parent_job_id FROM job WHERE type = ?1 ORDER BY created_at DESC LIMIT ?2")
        .map_err(|e| e.to_string())?;

    let jobs = stmt
        .query_map(rusqlite::params![job_type, limit], |row| {
            Ok(Job {
                id: row.get(0)?,
                job_type: row.get(1)?,
                status: row.get(2)?,
                input: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                completed_at: row.get(6)?,
                error_message: row.get(7)?,
                progress: row.get::<_, i32>(8)?,
                progress_total: row.get::<_, i32>(9)?,
                parent_job_id: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(jobs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use rusqlite::Connection;
    use std::sync::Mutex;

    fn test_db() -> Db {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        let migrations = [
            crate::db::MIGRATION_001_SQL,
            crate::db::MIGRATION_002_SQL,
            crate::db::MIGRATION_003_SQL,
            crate::db::MIGRATION_004_SQL,
            crate::db::MIGRATION_005_SQL,
            crate::db::MIGRATION_006_SQL,
            crate::db::MIGRATION_007_SQL,
            crate::db::MIGRATION_008_SQL,
            crate::db::MIGRATION_009_SQL,
            crate::db::MIGRATION_010_SQL,
            crate::db::MIGRATION_011_SQL,
            crate::db::MIGRATION_012_SQL,
            crate::db::MIGRATION_013_SQL,
        ];
        for sql in &migrations {
            conn.execute_batch(sql).unwrap();
        }

        Db {
            conn: Mutex::new(conn),
        }
    }

    #[test]
    fn test_create_job() {
        let db = test_db();
        let job_id = create_job("scan", r#"{"rootIds":["abc"]}"#, &db).unwrap();
        assert!(!job_id.is_empty());

        let job = load_job(&job_id, &db).unwrap();
        assert_eq!(job.job_type, "scan");
        assert_eq!(job.status, "pending");
    }

    #[test]
    fn test_complete_job() {
        let db = test_db();
        let job_id = create_job("audit", "{}", &db).unwrap();
        begin_job(&job_id, &db, &JobRuntime::default()).unwrap();
        complete_job(&job_id, &db).unwrap();

        let job = load_job(&job_id, &db).unwrap();
        assert_eq!(job.status, "completed");
        assert!(job.completed_at.is_some());
    }

    #[test]
    fn test_fail_job() {
        let db = test_db();
        let job_id = create_job("reindex", "{}", &db).unwrap();
        begin_job(&job_id, &db, &JobRuntime::default()).unwrap();
        fail_job(&job_id, "disk full", &db).unwrap();

        let job = load_job(&job_id, &db).unwrap();
        assert_eq!(job.status, "failed");
        assert_eq!(job.error_message, Some("disk full".to_string()));
    }

    #[test]
    fn test_append_job_event() {
        let db = test_db();
        let job_id = create_job("scan", "{}", &db).unwrap();
        append_job_event(&job_id, "custom_event", r#"{"detail":"test"}"#, &db).unwrap();

        let conn = db.conn.lock().unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM job_event WHERE job_id = ?1 AND type = 'custom_event'",
                rusqlite::params![job_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_cancel_job() {
        let db = test_db();
        let job_id = create_job("verification", "{}", &db).unwrap();
        let runtime = JobRuntime::default();
        begin_job(&job_id, &db, &runtime).unwrap();
        let job = cancel_job(&job_id, &db, &runtime).unwrap();
        assert_eq!(job.status, "cancelled");
    }

    #[test]
    fn test_retry_job() {
        let db = test_db();
        let job_id = create_job("scan", r#"{"roots":["r1"]}"#, &db).unwrap();
        begin_job(&job_id, &db, &JobRuntime::default()).unwrap();
        fail_job(&job_id, "timeout", &db).unwrap();

        let new_job = retry_job(&job_id, &db).unwrap();
        assert_eq!(new_job.job_type, "scan");
        assert_eq!(new_job.status, "pending");
        assert_eq!(new_job.parent_job_id, Some(job_id));
    }

    #[test]
    fn test_update_progress() {
        let db = test_db();
        let job_id = create_job("scan", "{}", &db).unwrap();
        update_progress(&job_id, 3, 10, &db).unwrap();

        let job = load_job(&job_id, &db).unwrap();
        assert_eq!(job.progress, 3);
        assert_eq!(job.progress_total, 10);
    }
}
