-- Speaker identification metadata.
-- This migration is additive and keeps transcripts.speaker as a legacy compatibility field.

CREATE TABLE IF NOT EXISTS speaker_labels (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    display_name TEXT NOT NULL,
    source TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'detected',
    confidence REAL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT,
    metadata_json TEXT,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_speaker_labels_meeting_id ON speaker_labels(meeting_id);
CREATE INDEX IF NOT EXISTS idx_speaker_labels_status ON speaker_labels(status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_speaker_labels_active_name
    ON speaker_labels(meeting_id, display_name)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS speaker_corrections (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    action TEXT NOT NULL,
    before_json TEXT,
    after_json TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_speaker_corrections_meeting_id ON speaker_corrections(meeting_id);

CREATE TABLE IF NOT EXISTS transcript_speaker_segments (
    id TEXT PRIMARY KEY,
    meeting_id TEXT NOT NULL,
    transcript_id TEXT NOT NULL,
    speaker_label_id TEXT NOT NULL,
    start_time REAL,
    end_time REAL,
    source TEXT NOT NULL,
    confidence REAL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    correction_id TEXT,
    FOREIGN KEY (meeting_id) REFERENCES meetings(id) ON DELETE CASCADE,
    FOREIGN KEY (transcript_id) REFERENCES transcripts(id) ON DELETE CASCADE,
    FOREIGN KEY (speaker_label_id) REFERENCES speaker_labels(id) ON DELETE CASCADE,
    FOREIGN KEY (correction_id) REFERENCES speaker_corrections(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_transcript_speaker_segments_meeting_id ON transcript_speaker_segments(meeting_id);
CREATE INDEX IF NOT EXISTS idx_transcript_speaker_segments_transcript_id ON transcript_speaker_segments(transcript_id);
CREATE INDEX IF NOT EXISTS idx_transcript_speaker_segments_label_id ON transcript_speaker_segments(speaker_label_id);
