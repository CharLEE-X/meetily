-- Provider-neutral calendar integration cache.
-- Calendar sync is opt-in and stores only the minimum metadata needed for
-- meeting detection and recording setup.

CREATE TABLE IF NOT EXISTS calendar_provider_accounts (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    account_label TEXT NOT NULL,
    status TEXT NOT NULL,
    last_sync_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_calendar_provider_accounts_provider
    ON calendar_provider_accounts(provider);

CREATE TABLE IF NOT EXISTS calendar_sources (
    id TEXT PRIMARY KEY,
    provider_account_id TEXT NOT NULL,
    provider_calendar_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT,
    selected INTEGER NOT NULL DEFAULT 1,
    read_only INTEGER NOT NULL DEFAULT 1,
    last_sync_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (provider_account_id) REFERENCES calendar_provider_accounts(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_calendar_sources_provider_calendar
    ON calendar_sources(provider_account_id, provider_calendar_id);
CREATE INDEX IF NOT EXISTS idx_calendar_sources_selected
    ON calendar_sources(selected);

CREATE TABLE IF NOT EXISTS calendar_events (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    provider_event_id TEXT NOT NULL,
    calendar_source_id TEXT NOT NULL,
    title TEXT NOT NULL,
    starts_at TEXT NOT NULL,
    ends_at TEXT NOT NULL,
    timezone TEXT,
    location TEXT,
    meeting_url TEXT,
    meeting_provider TEXT,
    attendee_count INTEGER,
    attendee_names_json TEXT,
    organizer_name TEXT,
    description_excerpt TEXT,
    content_hash TEXT NOT NULL,
    sync_status TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (calendar_source_id) REFERENCES calendar_sources(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_calendar_events_provider_event_source
    ON calendar_events(provider, provider_event_id, calendar_source_id);
CREATE INDEX IF NOT EXISTS idx_calendar_events_starts_at
    ON calendar_events(starts_at);
CREATE INDEX IF NOT EXISTS idx_calendar_events_meeting_url
    ON calendar_events(meeting_url);
CREATE INDEX IF NOT EXISTS idx_calendar_events_sync_status
    ON calendar_events(sync_status);

CREATE TABLE IF NOT EXISTS meeting_calendar_links (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    calendar_event_id TEXT NOT NULL,
    link_source TEXT NOT NULL,
    confidence REAL,
    apple_event_identifier TEXT,
    notes_export_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (calendar_event_id) REFERENCES calendar_events(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_meeting_calendar_links_meeting_event
    ON meeting_calendar_links(meeting_id, calendar_event_id);
CREATE INDEX IF NOT EXISTS idx_meeting_calendar_links_meeting
    ON meeting_calendar_links(meeting_id);
CREATE INDEX IF NOT EXISTS idx_meeting_calendar_links_event
    ON meeting_calendar_links(calendar_event_id);
