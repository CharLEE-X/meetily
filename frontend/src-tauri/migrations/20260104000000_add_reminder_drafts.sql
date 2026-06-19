-- Local reminder draft cache.
-- Drafts are editable suggestions only; this table does not represent
-- externally-created Apple Reminders.

CREATE TABLE IF NOT EXISTS reminder_drafts (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    summary_id TEXT,
    title TEXT NOT NULL,
    notes TEXT,
    due_at TEXT,
    priority INTEGER,
    list_id TEXT,
    category TEXT NOT NULL,
    confidence REAL NOT NULL,
    source_evidence_json TEXT NOT NULL,
    dedupe_key TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (list_id) REFERENCES reminder_lists(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_drafts_meeting_dedupe
    ON reminder_drafts(meeting_id, dedupe_key);
CREATE INDEX IF NOT EXISTS idx_reminder_drafts_meeting
    ON reminder_drafts(meeting_id);
CREATE INDEX IF NOT EXISTS idx_reminder_drafts_status
    ON reminder_drafts(status);
