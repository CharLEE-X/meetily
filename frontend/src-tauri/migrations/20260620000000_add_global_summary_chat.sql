CREATE TABLE IF NOT EXISTS global_summary_chat_messages (
    id TEXT PRIMARY KEY NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'completed', 'failed', 'canceled')),
    provider TEXT,
    model TEXT,
    citations TEXT,
    error TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_global_summary_chat_messages_created
ON global_summary_chat_messages(created_at);
