-- Screenshot artifact metadata for opt-in periodic capture.
-- Image files are stored under app-managed meeting artifact folders.

CREATE TABLE IF NOT EXISTS meeting_screenshots (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    recording_time REAL,
    file_path TEXT NOT NULL,
    thumbnail_path TEXT,
    display_label TEXT,
    status TEXT NOT NULL DEFAULT 'captured',
    redaction_status TEXT NOT NULL DEFAULT 'not_available',
    source TEXT NOT NULL DEFAULT 'periodic',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    metadata_json TEXT,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_meeting_screenshots_meeting_id ON meeting_screenshots(meeting_id);
CREATE INDEX IF NOT EXISTS idx_meeting_screenshots_captured_at ON meeting_screenshots(captured_at);
CREATE INDEX IF NOT EXISTS idx_meeting_screenshots_status ON meeting_screenshots(status);
