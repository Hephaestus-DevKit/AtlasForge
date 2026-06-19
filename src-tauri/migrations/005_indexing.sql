-- Indexing v1: Full-text search with FTS5, source documents, chunks

CREATE TABLE IF NOT EXISTS source_document (
    id TEXT PRIMARY KEY NOT NULL,
    repo_id TEXT REFERENCES repository(id) ON DELETE CASCADE,
    path TEXT NOT NULL,
    mime_type TEXT NOT NULL DEFAULT 'text/plain',
    language TEXT,
    size_bytes INTEGER NOT NULL DEFAULT 0,
    modified_at TEXT,
    indexed_at TEXT NOT NULL DEFAULT (datetime('now')),
    content_hash TEXT,
    UNIQUE(repo_id, path)
);

CREATE INDEX IF NOT EXISTS idx_source_document_repo ON source_document(repo_id);

CREATE TABLE IF NOT EXISTS chunk (
    id TEXT PRIMARY KEY NOT NULL,
    document_id TEXT NOT NULL REFERENCES source_document(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    heading TEXT,
    start_line INTEGER,
    end_line INTEGER,
    chunk_type TEXT NOT NULL DEFAULT 'text' CHECK(chunk_type IN ('text', 'code', 'config', 'log')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(document_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_chunk_document ON chunk(document_id);

-- FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS chunk_fts USING fts5(
    content,
    heading,
    path,
    chunk_type,
    content='chunk',
    content_rowid='rowid',
    tokenize='unicode61'
);

-- Triggers to keep FTS5 in sync
CREATE TRIGGER IF NOT EXISTS chunk_ai AFTER INSERT ON chunk BEGIN
    INSERT INTO chunk_fts(rowid, content, heading, path, chunk_type)
    SELECT new.rowid, new.content, new.heading, sd.path, new.chunk_type
    FROM source_document sd WHERE sd.id = new.document_id;
END;

CREATE TRIGGER IF NOT EXISTS chunk_ad AFTER DELETE ON chunk BEGIN
    INSERT INTO chunk_fts(chunk_fts, rowid, content, heading, path, chunk_type)
    VALUES('delete', old.rowid, old.content, old.heading, '', old.chunk_type);
END;

CREATE TRIGGER IF NOT EXISTS chunk_au AFTER UPDATE ON chunk BEGIN
    INSERT INTO chunk_fts(chunk_fts, rowid, content, heading, path, chunk_type)
    VALUES('delete', old.rowid, old.content, old.heading, '', old.chunk_type);
    INSERT INTO chunk_fts(rowid, content, heading, path, chunk_type)
    SELECT new.rowid, new.content, new.heading, sd.path, new.chunk_type
    FROM source_document sd WHERE sd.id = new.document_id;
END;
