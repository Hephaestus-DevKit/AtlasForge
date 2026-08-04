-- Preserve repository history while hiding assets that disappeared or became excluded.
ALTER TABLE project_asset ADD COLUMN is_available INTEGER NOT NULL DEFAULT 1;
ALTER TABLE project_asset ADD COLUMN missing_since TEXT;
CREATE INDEX IF NOT EXISTS idx_project_asset_root_available
    ON project_asset(root_id, is_available);
