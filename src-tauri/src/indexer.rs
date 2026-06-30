use crate::db::Db;
use std::collections::HashSet;
use std::path::Path;

const INDEX_VERSION: i64 = 1;
type ChunkData = (Option<String>, Option<i32>, Option<i32>, String);

pub fn index_repo(repo_id: &str, worktree_path: &str, db: &Db) -> Result<IndexStats, String> {
    let root = Path::new(worktree_path);
    if !root.exists() || !root.is_dir() {
        return Err(format!("Repo path does not exist: {}", worktree_path));
    }

    #[allow(dead_code)]
    struct ExistingDoc {
        id: String,
        content_hash: Option<String>,
        modified_at: Option<String>,
        size_bytes: i64,
        index_version: i64,
        chunk_count: i64,
    }

    let mut stats = IndexStats::default();
    let files = collect_indexable_files(root);
    let current_paths: HashSet<String> =
        files.iter().map(|(relative, _)| relative.clone()).collect();

    // Use a dedicated connection to avoid locking the main Mutex
    let mut conn = db.connection().map_err(|e| e.to_string())?;

    // Pre-load existing document metadata to avoid N+1 queries
    let existing_docs: std::collections::HashMap<String, ExistingDoc> = {
        let mut stmt = conn
            .prepare(
                "SELECT path, content_hash, modified_at, size_bytes, index_version, id,
                        (SELECT COUNT(*) FROM chunk WHERE document_id = source_document.id)
                 FROM source_document WHERE repo_id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![repo_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    ExistingDoc {
                        content_hash: row.get(1)?,
                        modified_at: row.get(2)?,
                        size_bytes: row.get(3)?,
                        index_version: row.get(4)?,
                        id: row.get(5)?,
                        chunk_count: row.get(6)?,
                    },
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut map = std::collections::HashMap::new();
        for row in rows {
            if let Ok((path, doc)) = row {
                map.insert(path, doc);
            }
        }
        map
    };

    let mut files_to_process = Vec::new();

    for (relative_path, abs_path) in files {
        if let Ok(metadata) = std::fs::metadata(&abs_path) {
            let size = metadata.len() as i64;
            let modified = metadata
                .modified()
                .ok()
                .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|value| value.as_secs().to_string());

            let mut skip = false;
            if let Some(existing) = existing_docs.get(&relative_path) {
                if existing.size_bytes == size
                    && existing.modified_at == modified
                    && existing.index_version == INDEX_VERSION
                {
                    stats.documents += 1;
                    stats.chunks += existing.chunk_count as usize;
                    stats.skipped_documents += 1;
                    skip = true;
                }
            }

            if !skip {
                files_to_process.push((relative_path, abs_path, metadata));
            }
        }
    }

    let processed_files = std::sync::Mutex::new(Vec::new());
    let process_errors = std::sync::Mutex::new(Vec::new());

    // Process new or modified files in parallel using scoped threads
    std::thread::scope(|s| {
        let mut threads = Vec::new();
        for (relative_path, abs_path, metadata) in files_to_process {
            let processed_ref = &processed_files;
            let errors_ref = &process_errors;
            threads.push(s.spawn(move || {
                match std::fs::read_to_string(&abs_path) {
                    Ok(content) => {
                        let content = crate::ai_provider::redact_secrets(&content);
                        let language = detect_language_from_path(&relative_path);
                        let mime_type = detect_mime_type(&relative_path);
                        let content_hash = simple_hash(&content);
                        let chunks = chunk_content(&content, &relative_path);
                        processed_ref.lock().unwrap().push((
                            relative_path,
                            mime_type,
                            language,
                            metadata,
                            content_hash,
                            chunks,
                        ));
                    }
                    Err(e) => {
                        errors_ref
                            .lock()
                            .unwrap()
                            .push(format!("{}: {}", relative_path, e));
                    }
                }
            }));
        }
        for t in threads {
            let _ = t.join();
        }
    });

    for err in process_errors.into_inner().unwrap() {
        stats.errors.push(err);
    }

    // Write new or modified files sequentially in a single transaction
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    for (relative_path, mime_type, language, metadata, content_hash, chunks) in
        processed_files.into_inner().unwrap()
    {
        let chunk_count = chunks.len();

        let doc_id: String = tx
            .query_row(
                "SELECT id FROM source_document WHERE repo_id = ?1 AND path = ?2",
                rusqlite::params![repo_id, relative_path],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| uuid::Uuid::new_v4().to_string());

        tx.execute(
            "INSERT INTO source_document
             (id, repo_id, path, mime_type, language, size_bytes, modified_at, indexed_at, content_hash, index_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, datetime('now'), ?8, ?9)
             ON CONFLICT(repo_id, path) DO UPDATE SET
                 mime_type = excluded.mime_type,
                 language = excluded.language,
                 size_bytes = excluded.size_bytes,
                 modified_at = excluded.modified_at,
                 indexed_at = excluded.indexed_at,
                 content_hash = excluded.content_hash,
                 index_version = excluded.index_version",
            rusqlite::params![
                doc_id,
                repo_id,
                relative_path,
                mime_type,
                language,
                metadata.len() as i64,
                metadata.modified().ok().and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok()).map(|value| value.as_secs().to_string()),
                content_hash,
                INDEX_VERSION,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Delete old chunks
        tx.execute(
            "DELETE FROM chunk WHERE document_id = ?1",
            rusqlite::params![doc_id],
        )
        .map_err(|e| e.to_string())?;

        for (seq, (heading, start_line, end_line, chunk_text)) in chunks.iter().enumerate() {
            let chunk_id = uuid::Uuid::new_v4().to_string();
            let chunk_type = detect_chunk_type(&relative_path);

            tx.execute(
                "INSERT INTO chunk (id, document_id, seq, content, heading, start_line, end_line, chunk_type) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![chunk_id, doc_id, seq as i64, chunk_text, heading, start_line, end_line, chunk_type],
            )
            .map_err(|e| e.to_string())?;
        }

        stats.documents += 1;
        stats.chunks += chunk_count;
        stats.indexed_documents += 1;
    }

    remove_stale_documents(repo_id, &current_paths, &tx)?;
    tx.commit().map_err(|e| e.to_string())?;

    Ok(stats)
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStats {
    pub documents: usize,
    pub chunks: usize,
    pub indexed_documents: usize,
    pub skipped_documents: usize,
    pub errors: Vec<String>,
}

/// Search the full-text index.
pub fn search(
    query: &str,
    limit: i64,
    repo_id: Option<&str>,
    db: &Db,
) -> Result<Vec<SearchResult>, String> {
    let query = build_fts_query(query);
    if query.is_empty() {
        return Ok(vec![]);
    }
    let limit = limit.clamp(1, 100);

    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let sql = match repo_id {
        Some(_) =>
            "SELECT c.id, c.content, c.heading, c.start_line, c.end_line, c.chunk_type, sd.path, sd.repo_id, rank
             FROM chunk_fts cf
             JOIN chunk c ON c.rowid = cf.rowid
             JOIN source_document sd ON sd.id = c.document_id
             WHERE chunk_fts MATCH ?1 AND sd.repo_id = ?2
             ORDER BY rank
             LIMIT ?3",
        None =>
            "SELECT c.id, c.content, c.heading, c.start_line, c.end_line, c.chunk_type, sd.path, sd.repo_id, rank
             FROM chunk_fts cf
             JOIN chunk c ON c.rowid = cf.rowid
             JOIN source_document sd ON sd.id = c.document_id
             WHERE chunk_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
    };

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;

    let results = match repo_id {
        Some(rid) => stmt
            .query_map(rusqlite::params![query, rid, limit], |row| {
                Ok(SearchResult {
                    chunk_id: row.get(0)?,
                    content: row.get(1)?,
                    heading: row.get(2)?,
                    start_line: row.get(3)?,
                    end_line: row.get(4)?,
                    chunk_type: row.get(5)?,
                    path: row.get(6)?,
                    repo_id: row.get(7)?,
                    rank: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?,
        None => stmt
            .query_map(rusqlite::params![query, limit], |row| {
                Ok(SearchResult {
                    chunk_id: row.get(0)?,
                    content: row.get(1)?,
                    heading: row.get(2)?,
                    start_line: row.get(3)?,
                    end_line: row.get(4)?,
                    chunk_type: row.get(5)?,
                    path: row.get(6)?,
                    repo_id: row.get(7)?,
                    rank: row.get(8)?,
                })
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?,
    };

    Ok(results)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub chunk_id: String,
    pub content: String,
    pub heading: Option<String>,
    pub start_line: Option<i32>,
    pub end_line: Option<i32>,
    pub chunk_type: String,
    pub path: String,
    pub repo_id: String,
    pub rank: f64,
}

/// List all indexed documents for a repo.
pub fn list_documents(repo_id: &str, db: &Db) -> Result<Vec<serde_json::Value>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare("SELECT id, path, mime_type, language, size_bytes, indexed_at FROM source_document WHERE repo_id = ?1 ORDER BY path")
        .map_err(|e| e.to_string())?;

    let documents = stmt
        .query_map(rusqlite::params![repo_id], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "path": row.get::<_, String>(1)?,
                "mimeType": row.get::<_, String>(2)?,
                "language": row.get::<_, Option<String>>(3)?,
                "sizeBytes": row.get::<_, i64>(4)?,
                "indexedAt": row.get::<_, String>(5)?,
            }))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(documents)
}

// --- Internal ---

const EXCLUDED_EXTENSIONS: &[&str] = &[
    "env",
    "pem",
    "key",
    "p12",
    "pfx",
    "jks",
    "keystore",
    "secret",
    "credentials",
    "token",
    "passwd",
    "shadow",
    "exe",
    "dll",
    "so",
    "dylib",
    "bin",
    "obj",
    "o",
    "pyc",
    "class",
    "png",
    "jpg",
    "jpeg",
    "gif",
    "ico",
    "svg",
    "woff",
    "woff2",
    "ttf",
    "eot",
    "zip",
    "tar",
    "gz",
    "bz2",
    "7z",
    "rar",
    "db",
    "sqlite",
    "lock",
];

const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    "__pycache__",
    ".next",
    ".cache",
    ".venv",
    "vendor",
    "coverage",
    ".secret",
    ".secrets",
    "secrets",
    ".credentials",
    "credentials",
];

const INDEXABLE_EXTENSIONS: &[&str] = &[
    "md",
    "txt",
    "rst",
    "adoc",
    "ts",
    "tsx",
    "js",
    "jsx",
    "mjs",
    "cjs",
    "rs",
    "go",
    "py",
    "rb",
    "java",
    "kt",
    "swift",
    "c",
    "cpp",
    "h",
    "hpp",
    "toml",
    "yaml",
    "yml",
    "json",
    "xml",
    "ini",
    "cfg",
    "conf",
    "sh",
    "bash",
    "zsh",
    "fish",
    "dockerfile",
    "dockerignore",
    "gitignore",
    "envrc",
    "editorconfig",
    "sql",
];

fn should_index(path: &Path) -> bool {
    // Check directory components
    for component in path.components() {
        if let std::path::Component::Normal(os_str) = component {
            if let Some(s) = os_str.to_str() {
                if EXCLUDED_DIRS.contains(&s) {
                    return false;
                }
            }
        }
    }

    // Check extension
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        if EXCLUDED_EXTENSIONS.contains(&ext_lower.as_str()) {
            return false;
        }
        // Files named .env, .env.local etc
        if ext_lower.starts_with("env") {
            return false;
        }
    }

    // Check filename
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let name_lower = name.to_lowercase();
        if name_lower.starts_with(".env") {
            return false;
        }
        // Credential and secret filenames
        if matches!(
            name_lower.as_str(),
            "id_rsa"
                | "id_ed25519"
                | "id_ecdsa"
                | "id_dsa"
                | "authorized_keys"
                | "known_hosts"
                | ".npmrc"
                | ".pypirc"
                | ".netrc"
                | ".aws_credentials"
                | "credentials.json"
                | "service-account.json"
                | "serviceaccount.json"
                | ".kubeconfig"
                | "sa.key"
                | "sa.pem"
        ) {
            return false;
        }
    }

    // Must be a known indexable type or have no extension (e.g. Makefile, Dockerfile)
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        INDEXABLE_EXTENSIONS.contains(&ext_lower.as_str())
    } else {
        // Files without extension: index common ones
        matches!(
            path.file_name().and_then(|n| n.to_str()),
            Some(
                "Makefile" | "Dockerfile" | "Rakefile" | "Gemfile" | "Vagrantfile" | "Jenkinsfile"
            )
        )
    }
}

fn collect_indexable_files(root: &Path) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();

    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            if path == root {
                return true;
            }
            if path.is_dir() {
                for component in path.components() {
                    if let std::path::Component::Normal(os_str) = component {
                        if let Some(s) = os_str.to_str() {
                            if EXCLUDED_DIRS.contains(&s) {
                                return false;
                            }
                        }
                    }
                }
                return true;
            }
            should_index(path)
        })
    {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();
        let abs = path.to_string_lossy().to_string();
        if visited.contains(&abs) {
            continue;
        }
        visited.insert(abs.clone());

        let relative = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .to_string();

        // Size check: skip files > 1MB
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.len() > 1_000_000 {
                continue;
            }
        }

        result.push((relative, abs));
    }

    result
}



fn remove_stale_documents(
    repo_id: &str,
    current_paths: &HashSet<String>,
    conn: &rusqlite::Connection,
) -> Result<(), String> {
    let stale_ids = {
        let mut stmt = conn
            .prepare("SELECT id, path FROM source_document WHERE repo_id = ?1")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![repo_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        rows.filter_map(Result::ok)
            .filter(|(_, path)| !current_paths.contains(path))
            .map(|(id, _)| id)
            .collect::<Vec<_>>()
    };
    for id in stale_ids {
        conn.execute(
            "DELETE FROM source_document WHERE id = ?1",
            rusqlite::params![id],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn build_fts_query(query: &str) -> String {
    query
        .split_whitespace()
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn chunk_content(content: &str, path: &str) -> Vec<ChunkData> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    match ext {
        "md" | "rst" | "adoc" | "txt" => chunk_markdown(content),
        "toml" | "yaml" | "yml" | "json" | "xml" | "ini" | "cfg" | "conf" => chunk_config(content),
        _ => chunk_code(content),
    }
}

fn chunk_markdown(content: &str) -> Vec<ChunkData> {
    let mut chunks = Vec::new();
    let mut current_heading: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();
    let mut start_line: usize = 1;

    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;

        if line.starts_with("#") {
            if !current_lines.is_empty() {
                chunks.push((
                    current_heading.clone(),
                    Some(start_line as i32),
                    Some((line_num - 1) as i32),
                    current_lines.join("\n"),
                ));
                start_line = line_num;
            }
            current_heading = Some(line.trim_start_matches('#').trim().to_string());
            current_lines = vec![line.to_string()];
        } else {
            current_lines.push(line.to_string());
        }

        // Flush if too long
        if current_lines.len() > 200 {
            chunks.push((
                current_heading.clone(),
                Some(start_line as i32),
                Some(line_num as i32),
                current_lines.join("\n"),
            ));
            current_lines = Vec::new();
            start_line = line_num + 1;
        }
    }

    if !current_lines.is_empty() {
        let end = content.lines().count();
        chunks.push((
            current_heading,
            Some(start_line as i32),
            Some(end as i32),
            current_lines.join("\n"),
        ));
    }

    if chunks.is_empty() {
        chunks.push((None, Some(1), Some(1), content.to_string()));
    }

    chunks
}

fn chunk_config(content: &str) -> Vec<ChunkData> {
    // Config files: chunk by top-level sections or whole file if small
    if content.lines().count() < 100 {
        return vec![(
            None,
            Some(1),
            Some(content.lines().count() as i32),
            content.to_string(),
        )];
    }

    let mut chunks = Vec::new();
    let mut current_lines: Vec<String> = Vec::new();
    let mut start_line: usize = 1;

    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;

        // Top-level key in yaml/toml/json
        if !line.starts_with(' ')
            && !line.starts_with('\t')
            && !line.is_empty()
            && !line.starts_with('#')
            && !line.starts_with('{')
            && !line.starts_with('}')
            && !current_lines.is_empty()
        {
            chunks.push((
                Some(
                    current_lines[0]
                        .split(':')
                        .next()
                        .unwrap_or("")
                        .split('=')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                ),
                Some(start_line as i32),
                Some((line_num - 1) as i32),
                current_lines.join("\n"),
            ));
            start_line = line_num;
        }

        current_lines.push(line.to_string());

        if current_lines.len() > 80 {
            chunks.push((
                None,
                Some(start_line as i32),
                Some(line_num as i32),
                current_lines.join("\n"),
            ));
            current_lines = Vec::new();
            start_line = line_num + 1;
        }
    }

    if !current_lines.is_empty() {
        chunks.push((
            None,
            Some(start_line as i32),
            Some(content.lines().count() as i32),
            current_lines.join("\n"),
        ));
    }

    if chunks.is_empty() {
        chunks.push((None, Some(1), Some(1), content.to_string()));
    }

    chunks
}

fn chunk_code(content: &str) -> Vec<ChunkData> {
    let mut chunks = Vec::new();
    let mut current_lines: Vec<String> = Vec::new();
    let mut start_line: usize = 1;
    let mut current_heading: Option<String> = None;

    for (i, line) in content.lines().enumerate() {
        let line_num = i + 1;

        // Detect function/class definitions as section markers
        let trimmed = line.trim();
        if trimmed.starts_with("fn ")
            || trimmed.starts_with("pub fn ")
            || trimmed.starts_with("async fn ")
            || trimmed.starts_with("function ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("export function ")
            || trimmed.starts_with("export default function ")
            || trimmed.starts_with("pub struct ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("interface ")
            || trimmed.starts_with("type ")
            || trimmed.starts_with("enum ")
        {
            if !current_lines.is_empty() {
                chunks.push((
                    current_heading.clone(),
                    Some(start_line as i32),
                    Some((line_num - 1) as i32),
                    current_lines.join("\n"),
                ));
                start_line = line_num;
            }
            current_heading = Some(
                trimmed
                    .split('(')
                    .next()
                    .unwrap_or(trimmed)
                    .trim()
                    .to_string(),
            );
            current_lines = vec![line.to_string()];
        } else {
            current_lines.push(line.to_string());
        }

        // Flush if too long
        if current_lines.len() > 150 {
            chunks.push((
                current_heading.clone(),
                Some(start_line as i32),
                Some(line_num as i32),
                current_lines.join("\n"),
            ));
            current_lines = Vec::new();
            start_line = line_num + 1;
        }
    }

    if !current_lines.is_empty() {
        let end = content.lines().count();
        chunks.push((
            current_heading,
            Some(start_line as i32),
            Some(end as i32),
            current_lines.join("\n"),
        ));
    }

    if chunks.is_empty() {
        chunks.push((None, Some(1), Some(1), content.to_string()));
    }

    chunks
}

fn detect_language_from_path(path: &str) -> Option<String> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())?;
    Some(
        match ext {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" | "mjs" | "cjs" => "javascript",
            "py" => "python",
            "go" => "go",
            "java" => "java",
            "kt" => "kotlin",
            "swift" => "swift",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" => "cpp",
            "rb" => "ruby",
            "sh" | "bash" => "shell",
            "sql" => "sql",
            "toml" => "toml",
            "yaml" | "yml" => "yaml",
            "json" => "json",
            "xml" => "xml",
            "md" => "markdown",
            "html" => "html",
            "css" => "css",
            _ => ext,
        }
        .to_string(),
    )
}

fn detect_mime_type(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "md" => "text/markdown",
        "json" => "application/json",
        "yaml" | "yml" => "text/yaml",
        "xml" => "application/xml",
        "html" => "text/html",
        "css" => "text/css",
        _ => "text/plain",
    }
    .to_string()
}

fn detect_chunk_type(path: &str) -> String {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    match ext {
        "toml" | "yaml" | "yml" | "json" | "xml" | "ini" | "cfg" | "conf" => "config",
        "md" | "rst" | "adoc" | "txt" => "text",
        _ => "code",
    }
    .to_string()
}

fn simple_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Db;
    use std::fs;

    fn indexed_repo_db(repo_path: &Path) -> Db {
        let db = Db::new(&repo_path.join("test_db.db")).unwrap();
        let conn = db.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO workspace_root (id, path, label) VALUES ('root', ?1, 'Root')",
            rusqlite::params![repo_path.to_string_lossy()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO project_asset (id, root_id, path, name) VALUES ('asset', 'root', ?1, 'Repo')",
            rusqlite::params![repo_path.to_string_lossy()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO repository (id, asset_id, worktree_path, git_dir_path) VALUES ('repo', 'asset', ?1, ?2)",
            rusqlite::params![
                repo_path.to_string_lossy(),
                repo_path.join(".git").to_string_lossy()
            ],
        )
        .unwrap();
        drop(conn);
        db
    }

    #[test]
    fn test_should_index_excludes_env_files() {
        assert!(!should_index(Path::new(".env")));
        assert!(!should_index(Path::new(".env.local")));
        assert!(!should_index(Path::new("config/.env.production")));
    }

    #[test]
    fn test_should_index_excludes_secret_dirs() {
        assert!(!should_index(Path::new("secrets/key.txt")));
        assert!(!should_index(Path::new(".secrets/token.json")));
        assert!(!should_index(Path::new("credentials/aws.json")));
        assert!(!should_index(Path::new(".credentials/db.yaml")));
    }

    #[test]
    fn test_should_index_excludes_credential_files() {
        assert!(!should_index(Path::new("id_rsa")));
        assert!(!should_index(Path::new("id_ed25519")));
        assert!(!should_index(Path::new("service-account.json")));
        assert!(!should_index(Path::new("credentials.json")));
        assert!(!should_index(Path::new(".npmrc")));
        assert!(!should_index(Path::new(".netrc")));
    }

    #[test]
    fn test_should_index_excludes_sensitive_extensions() {
        assert!(!should_index(Path::new("cert.pem")));
        assert!(!should_index(Path::new("server.key")));
        assert!(!should_index(Path::new("app.p12")));
        assert!(!should_index(Path::new("token.secret")));
    }

    #[test]
    fn test_should_index_allows_source_code() {
        assert!(should_index(Path::new("src/main.rs")));
        assert!(should_index(Path::new("lib/index.ts")));
        assert!(should_index(Path::new("app.py")));
        assert!(should_index(Path::new("README.md")));
        assert!(should_index(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_should_index_excludes_binary_and_media() {
        assert!(!should_index(Path::new("app.exe")));
        assert!(!should_index(Path::new("logo.png")));
        assert!(!should_index(Path::new("archive.zip")));
        assert!(!should_index(Path::new("data.db")));
    }

    #[test]
    fn test_should_index_excludes_common_dirs() {
        assert!(!should_index(Path::new("node_modules/react/index.js")));
        assert!(!should_index(Path::new("target/debug/main.rs")));
        assert!(!should_index(Path::new("__pycache__/app.pyc")));
        assert!(!should_index(Path::new("dist/bundle.js")));
    }

    #[test]
    fn test_should_index_allows_extensionless_makefiles() {
        assert!(should_index(Path::new("Makefile")));
        assert!(should_index(Path::new("Dockerfile")));
    }

    #[test]
    fn fts_query_treats_user_input_as_literal_terms() {
        assert_eq!(build_fts_query("hello C++"), "\"hello\" AND \"C++\"");
        assert_eq!(build_fts_query("  quoted\"value  "), "\"quoted\"\"value\"");
    }

    #[test]
    fn reindex_removes_stale_documents_and_redacts_secrets() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("main.ts");
        fs::write(&source, "const token = \"abcdefghijklmnop123456\";").unwrap();
        let db = indexed_repo_db(temp.path());

        let stats = index_repo("repo", &temp.path().to_string_lossy(), &db).unwrap();
        assert_eq!(stats.documents, 1);
        assert_eq!(stats.indexed_documents, 1);
        assert_eq!(stats.skipped_documents, 0);
        let content: String = db
            .conn
            .lock()
            .unwrap()
            .query_row("SELECT content FROM chunk LIMIT 1", [], |row| row.get(0))
            .unwrap();
        assert!(content.contains("[REDACTED]"));
        assert!(!content.contains("abcdefghijklmnop123456"));

        let unchanged = index_repo("repo", &temp.path().to_string_lossy(), &db).unwrap();
        assert_eq!(unchanged.indexed_documents, 0);
        assert_eq!(unchanged.skipped_documents, 1);

        fs::remove_file(source).unwrap();
        index_repo("repo", &temp.path().to_string_lossy(), &db).unwrap();
        assert!(list_documents("repo", &db).unwrap().is_empty());
    }
}
