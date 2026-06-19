-- Job Engine enhancements: cancel, retry, priority, progress tracking
ALTER TABLE job ADD COLUMN priority INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job ADD COLUMN progress INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job ADD COLUMN progress_total INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job ADD COLUMN error_message TEXT;
ALTER TABLE job ADD COLUMN retry_count INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job ADD COLUMN max_retries INTEGER NOT NULL DEFAULT 0;
ALTER TABLE job ADD COLUMN parent_job_id TEXT REFERENCES job(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_job_parent ON job(parent_job_id);
CREATE INDEX IF NOT EXISTS idx_job_type_status ON job(type, status);
