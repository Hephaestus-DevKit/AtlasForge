-- Keep external-content FTS rows in sync with the source document path.
-- The original delete/update triggers supplied an empty path, leaving stale
-- path terms behind after reindexing or deleting a document. Since FTS5
-- rebuild requires every indexed column to exist in the content table, store
-- a denormalized path on chunk and rebuild the index once during migration.
ALTER TABLE chunk ADD COLUMN path TEXT NOT NULL DEFAULT '';
UPDATE chunk
SET path = COALESCE((SELECT path FROM source_document WHERE source_document.id = chunk.document_id), '');

DROP TRIGGER IF EXISTS chunk_ai;
DROP TRIGGER IF EXISTS chunk_ad;
DROP TRIGGER IF EXISTS chunk_au;
DROP TRIGGER IF EXISTS source_document_ad;
DROP TABLE IF EXISTS chunk_fts;

CREATE VIRTUAL TABLE chunk_fts USING fts5(
    content,
    heading,
    path,
    chunk_type,
    content='chunk',
    content_rowid='rowid',
    tokenize='unicode61'
);

CREATE TRIGGER chunk_ai AFTER INSERT ON chunk BEGIN
    INSERT INTO chunk_fts(rowid, content, heading, path, chunk_type)
    VALUES (new.rowid, new.content, new.heading, new.path, new.chunk_type);
END;

CREATE TRIGGER chunk_ad AFTER DELETE ON chunk BEGIN
    INSERT INTO chunk_fts(chunk_fts, rowid, content, heading, path, chunk_type)
    VALUES('delete', old.rowid, old.content, old.heading, old.path, old.chunk_type);
END;

CREATE TRIGGER chunk_au AFTER UPDATE ON chunk BEGIN
    INSERT INTO chunk_fts(chunk_fts, rowid, content, heading, path, chunk_type)
    VALUES('delete', old.rowid, old.content, old.heading, old.path, old.chunk_type);
    INSERT INTO chunk_fts(rowid, content, heading, path, chunk_type)
    VALUES(new.rowid, new.content, new.heading, new.path, new.chunk_type);
END;

INSERT INTO chunk_fts(chunk_fts) VALUES ('rebuild');
