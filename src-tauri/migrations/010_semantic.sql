-- Semantic Index: Embedding queue, knowledge items

CREATE TABLE IF NOT EXISTS embedding_queue (
    id TEXT PRIMARY KEY NOT NULL,
    chunk_id TEXT NOT NULL REFERENCES chunk(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN ('pending', 'processing', 'completed', 'failed')),
    model_id TEXT NOT NULL,
    model_version TEXT NOT NULL DEFAULT '',
    dimensions INTEGER,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_embedding_queue_status ON embedding_queue(status);
CREATE INDEX IF NOT EXISTS idx_embedding_queue_chunk ON embedding_queue(chunk_id);

CREATE TABLE IF NOT EXISTS embedding (
    id TEXT PRIMARY KEY NOT NULL,
    chunk_id TEXT NOT NULL REFERENCES chunk(id) ON DELETE CASCADE,
    queue_id TEXT NOT NULL REFERENCES embedding_queue(id) ON DELETE CASCADE,
    model_id TEXT NOT NULL,
    model_version TEXT NOT NULL DEFAULT '',
    dimensions INTEGER NOT NULL,
    vector BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(chunk_id, model_id)
);

CREATE INDEX IF NOT EXISTS idx_embedding_chunk ON embedding(chunk_id);

CREATE TABLE IF NOT EXISTS knowledge_item (
    id TEXT PRIMARY KEY NOT NULL,
    repo_id TEXT REFERENCES repository(id) ON DELETE CASCADE,
    source_type TEXT NOT NULL CHECK(source_type IN (
        'task_log', 'project_manual', 'ai_summary', 'user_note', 'api_doc'
    )),
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    source_refs TEXT NOT NULL DEFAULT '[]',
    chunk_ids TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_knowledge_repo ON knowledge_item(repo_id);
CREATE INDEX IF NOT EXISTS idx_knowledge_source_type ON knowledge_item(source_type);
