-- Skipped screenshot captures keep local metadata only and must not create dummy image files.

DROP TABLE IF EXISTS meeting_screenshots_next;

CREATE TABLE meeting_screenshots_next (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    recording_time REAL,
    file_path TEXT,
    thumbnail_path TEXT,
    display_label TEXT,
    status TEXT NOT NULL DEFAULT 'captured',
    redaction_status TEXT NOT NULL DEFAULT 'not_available',
    source TEXT NOT NULL DEFAULT 'periodic',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    metadata_json TEXT
);

INSERT INTO meeting_screenshots_next (
    id, meeting_id, captured_at, recording_time, file_path, thumbnail_path,
    display_label, status, redaction_status, source, created_at, updated_at,
    deleted_at, metadata_json
)
SELECT
    id, meeting_id, captured_at, recording_time, file_path, thumbnail_path,
    display_label, status, redaction_status, source, created_at, updated_at,
    deleted_at, metadata_json
FROM meeting_screenshots;

DROP TABLE meeting_screenshots;
ALTER TABLE meeting_screenshots_next RENAME TO meeting_screenshots;

CREATE INDEX IF NOT EXISTS idx_meeting_screenshots_meeting_id ON meeting_screenshots(meeting_id);
CREATE INDEX IF NOT EXISTS idx_meeting_screenshots_captured_at ON meeting_screenshots(captured_at);
CREATE INDEX IF NOT EXISTS idx_meeting_screenshots_status ON meeting_screenshots(status);
