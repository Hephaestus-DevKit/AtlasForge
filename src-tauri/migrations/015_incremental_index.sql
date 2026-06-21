-- Incremental indexing metadata.
ALTER TABLE source_document ADD COLUMN index_version INTEGER NOT NULL DEFAULT 1;
CREATE INDEX IF NOT EXISTS idx_source_document_repo_hash
    ON source_document(repo_id, content_hash);
