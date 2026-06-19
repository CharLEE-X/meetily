-- Tracks Apple Reminders created from Meetily drafts.
-- External reminders are not deleted or modified when the integration is disconnected.

CREATE TABLE IF NOT EXISTS reminder_created_links (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    draft_id TEXT,
    dedupe_key TEXT NOT NULL,
    provider TEXT NOT NULL,
    provider_reminder_id TEXT NOT NULL,
    list_id TEXT,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_error TEXT,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (draft_id) REFERENCES reminder_drafts(id) ON DELETE SET NULL,
    FOREIGN KEY (list_id) REFERENCES reminder_lists(id) ON DELETE SET NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_created_links_meeting_dedupe
    ON reminder_created_links(meeting_id, dedupe_key);
CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_created_links_provider_id
    ON reminder_created_links(provider, provider_reminder_id);
CREATE INDEX IF NOT EXISTS idx_reminder_created_links_meeting
    ON reminder_created_links(meeting_id);
