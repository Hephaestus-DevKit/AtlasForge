-- AI Provider v1: provider config, context packs, AI sessions

CREATE TABLE IF NOT EXISTS ai_provider (
    id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL UNIQUE,
    adapter_type TEXT NOT NULL CHECK(adapter_type IN ('openai_compatible', 'ollama', 'anthropic', 'custom')),
    base_url TEXT NOT NULL,
    api_key_ref TEXT,
    default_model TEXT NOT NULL,
    available_models TEXT NOT NULL DEFAULT '[]',
    is_local INTEGER NOT NULL DEFAULT 0,
    is_default INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    config TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS ai_session (
    id TEXT PRIMARY KEY NOT NULL,
    job_id TEXT REFERENCES job(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL REFERENCES ai_provider(id),
    model_id TEXT NOT NULL,
    purpose TEXT NOT NULL,
    context_pack TEXT NOT NULL DEFAULT '{}',
    system_prompt TEXT,
    total_tokens_in INTEGER NOT NULL DEFAULT 0,
    total_tokens_out INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'completed', 'failed', 'cancelled')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_ai_session_job ON ai_session(job_id);
CREATE INDEX IF NOT EXISTS idx_ai_session_provider ON ai_session(provider_id);

CREATE TABLE IF NOT EXISTS ai_message (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES ai_session(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant', 'tool')),
    content TEXT NOT NULL,
    tool_calls TEXT,
    tool_call_id TEXT,
    tokens_in INTEGER NOT NULL DEFAULT 0,
    tokens_out INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_ai_message_session ON ai_message(session_id);
