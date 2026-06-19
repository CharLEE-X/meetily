-- Calendar event creation is a separate opt-in from read-only calendar sync.

ALTER TABLE calendar_provider_accounts
    ADD COLUMN target_calendar_name TEXT NOT NULL DEFAULT 'Meetily';

ALTER TABLE calendar_provider_accounts
    ADD COLUMN auto_create_events INTEGER NOT NULL DEFAULT 0;
