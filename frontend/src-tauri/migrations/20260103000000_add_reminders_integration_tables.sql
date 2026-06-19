-- Provider-neutral reminder integration cache.
-- This slice stores only provider connection state and list metadata.
-- Reminder body/title creation is introduced in later follow-up issues.

CREATE TABLE IF NOT EXISTS reminder_provider_accounts (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    account_label TEXT NOT NULL,
    status TEXT NOT NULL,
    default_list_id TEXT,
    last_sync_at TEXT,
    last_error TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_provider_accounts_provider
    ON reminder_provider_accounts(provider);

CREATE TABLE IF NOT EXISTS reminder_lists (
    id TEXT PRIMARY KEY,
    provider_account_id TEXT NOT NULL,
    provider_list_id TEXT NOT NULL,
    name TEXT NOT NULL,
    color TEXT,
    selected INTEGER NOT NULL DEFAULT 1,
    is_default INTEGER NOT NULL DEFAULT 0,
    last_seen_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (provider_account_id) REFERENCES reminder_provider_accounts(id) ON DELETE CASCADE
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reminder_lists_provider_list
    ON reminder_lists(provider_account_id, provider_list_id);
CREATE INDEX IF NOT EXISTS idx_reminder_lists_selected
    ON reminder_lists(selected);
CREATE INDEX IF NOT EXISTS idx_reminder_lists_default
    ON reminder_lists(is_default);
