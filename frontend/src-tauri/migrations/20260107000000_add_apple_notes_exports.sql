-- Apple Notes export is opt-in and stores only app-managed destination/status
-- metadata. External Notes content is not copied back into Meetily.

CREATE TABLE IF NOT EXISTS apple_notes_provider_accounts (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    account_label TEXT NOT NULL,
    status TEXT NOT NULL,
    root_folder_name TEXT NOT NULL DEFAULT 'Meetily',
    grouping_mode TEXT NOT NULL DEFAULT 'none',
    auto_export_enabled INTEGER NOT NULL DEFAULT 0,
    confirmed_destination_hash TEXT,
    last_export_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_apple_notes_provider_accounts_provider
    ON apple_notes_provider_accounts(provider);

CREATE TABLE IF NOT EXISTS apple_notes_exports (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    provider TEXT NOT NULL,
    account_id TEXT,
    account_name TEXT,
    folder_id TEXT,
    folder_name TEXT,
    provider_note_id TEXT,
    note_title TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    status TEXT NOT NULL,
    last_error TEXT,
    exported_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_apple_notes_exports_meeting_provider
    ON apple_notes_exports(meeting_id, provider);
CREATE INDEX IF NOT EXISTS idx_apple_notes_exports_meeting
    ON apple_notes_exports(meeting_id);
CREATE INDEX IF NOT EXISTS idx_apple_notes_exports_status
    ON apple_notes_exports(status);
CREATE INDEX IF NOT EXISTS idx_apple_notes_exports_exported_at
    ON apple_notes_exports(exported_at);
