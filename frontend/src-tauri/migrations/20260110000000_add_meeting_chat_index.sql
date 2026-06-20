CREATE TABLE IF NOT EXISTS meeting_chat_index (
    id TEXT PRIMARY KEY NOT NULL,
    meeting_id TEXT NOT NULL,
    source_type TEXT NOT NULL CHECK (
        source_type IN (
            'transcript',
            'summary',
            'action_item',
            'key_point',
            'note',
            'screenshot'
        )
    ),
    source_id TEXT NOT NULL,
    source_label TEXT NOT NULL,
    title TEXT,
    text TEXT NOT NULL,
    timestamp TEXT,
    audio_start_time REAL,
    audio_end_time REAL,
    file_path TEXT,
    metadata_json TEXT,
    chunk_index INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_meeting_chat_index_meeting_type
ON meeting_chat_index(meeting_id, source_type);

CREATE INDEX IF NOT EXISTS idx_meeting_chat_index_meeting_source
ON meeting_chat_index(meeting_id, source_id);
