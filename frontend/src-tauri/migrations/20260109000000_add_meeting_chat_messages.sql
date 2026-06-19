CREATE TABLE IF NOT EXISTS meeting_chat_messages (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN ('pending', 'completed', 'failed', 'canceled')),
    provider TEXT,
    model TEXT,
    citations TEXT,
    error TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_meeting_chat_messages_meeting_created
ON meeting_chat_messages(meeting_id, created_at);
