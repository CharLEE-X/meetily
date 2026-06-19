-- Configurable follow-up presets for programmer-oriented Apple Reminders drafts.
-- These settings affect local draft generation only; externally-created reminders are not modified.

CREATE TABLE IF NOT EXISTS reminder_workflow_settings (
    provider TEXT PRIMARY KEY,
    global_priority INTEGER NOT NULL DEFAULT 5 CHECK (global_priority IN (1, 5, 9)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reminder_workflow_presets (
    category TEXT PRIMARY KEY CHECK (
        category IN (
            'pr_review',
            'linear_follow_up',
            'deploy_alert_check',
            'docs_update',
            'implementation_task',
            'experiment_revisit',
            'clarification_follow_up'
        )
    ),
    enabled INTEGER NOT NULL DEFAULT 1,
    default_list_id TEXT,
    default_priority INTEGER CHECK (default_priority IS NULL OR default_priority IN (1, 5, 9)),
    due_preset TEXT NOT NULL CHECK (due_preset IN ('none', 'in_2_hours', 'tomorrow_morning', 'in_2_days', 'next_week')),
    updated_at TEXT NOT NULL,
    FOREIGN KEY (default_list_id) REFERENCES reminder_lists(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_reminder_workflow_presets_enabled
    ON reminder_workflow_presets(enabled);
