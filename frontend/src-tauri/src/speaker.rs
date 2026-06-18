use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::collections::BTreeMap;
use tauri::Runtime;
use uuid::Uuid;

const SOURCE_HEURISTIC: &str = "heuristic";
const SOURCE_LEGACY: &str = "legacy";
const STATUS_DETECTED: &str = "detected";
const CONFIDENCE_LEGACY_SOURCE: f64 = 0.9;
const CONFIDENCE_TIMING_HEURISTIC: f64 = 0.45;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerLabel {
    pub id: String,
    pub meeting_id: String,
    pub display_name: String,
    pub source: String,
    pub status: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSpeakerSegment {
    pub id: String,
    pub meeting_id: String,
    pub transcript_id: String,
    pub speaker_label_id: String,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub source: String,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerLabelingResult {
    pub meeting_id: String,
    pub labels: Vec<SpeakerLabel>,
    pub segments: Vec<TranscriptSpeakerSegment>,
    pub strategy: String,
}

#[derive(Debug, Clone)]
struct TranscriptForLabeling {
    id: String,
    audio_start_time: Option<f64>,
    audio_end_time: Option<f64>,
    legacy_speaker: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct SpeakerAssignment {
    transcript_id: String,
    display_name: String,
    source: String,
    confidence: f64,
    start_time: Option<f64>,
    end_time: Option<f64>,
}

#[tauri::command]
pub async fn run_speaker_labeling<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<SpeakerLabelingResult, String> {
    run_speaker_labeling_for_meeting(state.inner(), &meeting_id).await
}

#[tauri::command]
pub async fn get_speaker_labels<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<SpeakerLabelingResult, String> {
    load_speaker_labeling_result(state.inner(), &meeting_id, "stored").await
}

#[tauri::command]
pub async fn clear_speaker_labels<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    include_confirmed: Option<bool>,
) -> Result<(), String> {
    let pool = state.db_manager.pool();
    let now = Utc::now().to_rfc3339();
    let include_confirmed = include_confirmed.unwrap_or(false);
    let label_scope = if include_confirmed {
        "SELECT id FROM speaker_labels WHERE meeting_id = ?"
    } else {
        "SELECT id FROM speaker_labels WHERE meeting_id = ? AND status != 'confirmed'"
    };

    sqlx::query(&format!(
        "DELETE FROM transcript_speaker_segments WHERE meeting_id = ? AND speaker_label_id IN ({})",
        label_scope
    ))
    .bind(&meeting_id)
    .bind(&meeting_id)
    .execute(pool)
    .await
    .map_err(|err| format!("Failed to clear speaker segments: {}", err))?;

    let label_update = if include_confirmed {
        "UPDATE speaker_labels SET status = 'deleted', deleted_at = ?, updated_at = ? WHERE meeting_id = ?"
    } else {
        "UPDATE speaker_labels SET status = 'deleted', deleted_at = ?, updated_at = ? WHERE meeting_id = ? AND status != 'confirmed'"
    };

    sqlx::query(label_update)
        .bind(&now)
        .bind(&now)
        .bind(&meeting_id)
        .execute(pool)
        .await
        .map_err(|err| format!("Failed to clear speaker labels: {}", err))?;

    Ok(())
}

async fn run_speaker_labeling_for_meeting(
    state: &AppState,
    meeting_id: &str,
) -> Result<SpeakerLabelingResult, String> {
    let pool = state.db_manager.pool();
    let transcripts = load_transcripts_for_labeling(pool, meeting_id).await?;
    let assignments = derive_speaker_assignments(&transcripts);
    let now = Utc::now().to_rfc3339();

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| format!("Failed to start speaker labeling transaction: {}", err))?;

    sqlx::query(
        r#"
        DELETE FROM transcript_speaker_segments
        WHERE meeting_id = ?
          AND speaker_label_id IN (
            SELECT id FROM speaker_labels
            WHERE meeting_id = ?
              AND source IN ('heuristic', 'legacy')
              AND status != 'confirmed'
          )
        "#,
    )
    .bind(meeting_id)
    .bind(meeting_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("Failed to reset detected speaker segments: {}", err))?;

    sqlx::query(
        r#"
        DELETE FROM speaker_labels
        WHERE meeting_id = ?
          AND source IN ('heuristic', 'legacy')
          AND status != 'confirmed'
        "#,
    )
    .bind(meeting_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("Failed to reset detected speaker labels: {}", err))?;

    let mut label_ids_by_name: BTreeMap<String, (String, String, f64, bool)> = BTreeMap::new();
    let existing_label_rows = sqlx::query(
        r#"
        SELECT id, display_name, source, confidence
        FROM speaker_labels
        WHERE meeting_id = ? AND deleted_at IS NULL
        "#,
    )
    .bind(meeting_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|err| format!("Failed to load existing speaker labels: {}", err))?;

    for row in existing_label_rows {
        let display_name: String = row.get("display_name");
        let id: String = row.get("id");
        let source: String = row.get("source");
        let confidence: Option<f64> = row.try_get("confidence").ok();
        label_ids_by_name.insert(display_name, (id, source, confidence.unwrap_or(1.0), true));
    }

    for assignment in &assignments {
        label_ids_by_name
            .entry(assignment.display_name.clone())
            .or_insert_with(|| {
                (
                    Uuid::new_v4().to_string(),
                    assignment.source.clone(),
                    assignment.confidence,
                    false,
                )
            });
    }

    for (display_name, (label_id, source, confidence, is_existing)) in &label_ids_by_name {
        if *is_existing {
            continue;
        }

        sqlx::query(
            r#"
            INSERT INTO speaker_labels
                (id, meeting_id, display_name, source, status, confidence, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(label_id)
        .bind(meeting_id)
        .bind(display_name)
        .bind(source)
        .bind(STATUS_DETECTED)
        .bind(confidence)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Failed to store speaker label: {}", err))?;
    }

    for assignment in &assignments {
        let Some((label_id, _, _, _)) = label_ids_by_name.get(&assignment.display_name) else {
            continue;
        };
        sqlx::query(
            r#"
            INSERT INTO transcript_speaker_segments
                (id, meeting_id, transcript_id, speaker_label_id, start_time, end_time, source, confidence, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(meeting_id)
        .bind(&assignment.transcript_id)
        .bind(label_id)
        .bind(assignment.start_time)
        .bind(assignment.end_time)
        .bind(&assignment.source)
        .bind(assignment.confidence)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|err| format!("Failed to store speaker segment: {}", err))?;
    }

    tx.commit()
        .await
        .map_err(|err| format!("Failed to commit speaker labels: {}", err))?;

    load_speaker_labeling_result(state, meeting_id, "local_timing_and_source").await
}

async fn load_transcripts_for_labeling(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Vec<TranscriptForLabeling>, String> {
    let rows = sqlx::query(
        r#"
        SELECT id, audio_start_time, audio_end_time, speaker
        FROM transcripts
        WHERE meeting_id = ?
        ORDER BY COALESCE(audio_start_time, 0), timestamp, id
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load transcripts for speaker labeling: {}", err))?;

    Ok(rows
        .into_iter()
        .map(|row| TranscriptForLabeling {
            id: row.get("id"),
            audio_start_time: row.try_get("audio_start_time").ok(),
            audio_end_time: row.try_get("audio_end_time").ok(),
            legacy_speaker: row.try_get("speaker").ok(),
        })
        .collect())
}

async fn load_speaker_labeling_result(
    state: &AppState,
    meeting_id: &str,
    strategy: &str,
) -> Result<SpeakerLabelingResult, String> {
    let pool = state.db_manager.pool();

    let label_rows = sqlx::query(
        r#"
        SELECT id, meeting_id, display_name, source, status, confidence
        FROM speaker_labels
        WHERE meeting_id = ? AND status != 'deleted'
        ORDER BY display_name
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load speaker labels: {}", err))?;

    let segment_rows = sqlx::query(
        r#"
        SELECT id, meeting_id, transcript_id, speaker_label_id, start_time, end_time, source, confidence
        FROM transcript_speaker_segments
        WHERE meeting_id = ?
        ORDER BY COALESCE(start_time, 0), transcript_id
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load speaker segments: {}", err))?;

    Ok(SpeakerLabelingResult {
        meeting_id: meeting_id.to_string(),
        labels: label_rows
            .into_iter()
            .map(|row| SpeakerLabel {
                id: row.get("id"),
                meeting_id: row.get("meeting_id"),
                display_name: row.get("display_name"),
                source: row.get("source"),
                status: row.get("status"),
                confidence: row.try_get("confidence").ok(),
            })
            .collect(),
        segments: segment_rows
            .into_iter()
            .map(|row| TranscriptSpeakerSegment {
                id: row.get("id"),
                meeting_id: row.get("meeting_id"),
                transcript_id: row.get("transcript_id"),
                speaker_label_id: row.get("speaker_label_id"),
                start_time: row.try_get("start_time").ok(),
                end_time: row.try_get("end_time").ok(),
                source: row.get("source"),
                confidence: row.try_get("confidence").ok(),
            })
            .collect(),
        strategy: strategy.to_string(),
    })
}

fn derive_speaker_assignments(transcripts: &[TranscriptForLabeling]) -> Vec<SpeakerAssignment> {
    transcripts
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            if let Some(legacy_speaker) = segment
                .legacy_speaker
                .as_deref()
                .and_then(normalize_legacy_speaker)
            {
                return SpeakerAssignment {
                    transcript_id: segment.id.clone(),
                    display_name: legacy_speaker,
                    source: SOURCE_LEGACY.to_string(),
                    confidence: CONFIDENCE_LEGACY_SOURCE,
                    start_time: segment.audio_start_time,
                    end_time: segment.audio_end_time,
                };
            }

            let display_name = timing_based_speaker_name(transcripts, index);
            SpeakerAssignment {
                transcript_id: segment.id.clone(),
                display_name,
                source: SOURCE_HEURISTIC.to_string(),
                confidence: CONFIDENCE_TIMING_HEURISTIC,
                start_time: segment.audio_start_time,
                end_time: segment.audio_end_time,
            }
        })
        .collect()
}

fn normalize_legacy_speaker(value: &str) -> Option<String> {
    let normalized = value.trim().to_lowercase();
    match normalized.as_str() {
        "mic" | "microphone" => Some("Microphone".to_string()),
        "system" | "system_audio" | "speaker" => Some("System Audio".to_string()),
        "" | "unknown" | "none" | "null" => None,
        other => sanitize_legacy_speaker_label(other).map(|label| format!("Speaker {}", label)),
    }
}

fn sanitize_legacy_speaker_label(value: &str) -> Option<String> {
    let cleaned = value
        .chars()
        .filter(|ch| {
            ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || *ch == '-' || *ch == '_'
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let truncated = cleaned.chars().take(40).collect::<String>();
    if truncated.is_empty() {
        None
    } else {
        Some(truncated)
    }
}

fn timing_based_speaker_name(transcripts: &[TranscriptForLabeling], index: usize) -> String {
    if index == 0 {
        return "Speaker 1".to_string();
    }

    let current = &transcripts[index];
    let previous = &transcripts[index - 1];
    let gap = match (previous.audio_end_time, current.audio_start_time) {
        (Some(previous_end), Some(current_start)) => current_start - previous_end,
        _ => 0.0,
    };

    if gap >= 2.5 && index % 2 == 1 {
        "Speaker 2".to_string()
    } else if gap >= 5.0 && index % 3 == 2 {
        "Speaker 3".to_string()
    } else {
        "Speaker 1".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transcript(
        id: &str,
        start: Option<f64>,
        end: Option<f64>,
        legacy_speaker: Option<&str>,
    ) -> TranscriptForLabeling {
        TranscriptForLabeling {
            id: id.to_string(),
            audio_start_time: start,
            audio_end_time: end,
            legacy_speaker: legacy_speaker.map(str::to_string),
        }
    }

    #[test]
    fn uses_legacy_audio_source_when_available() {
        let assignments = derive_speaker_assignments(&[
            transcript("a", Some(0.0), Some(1.0), Some("mic")),
            transcript("b", Some(1.2), Some(2.0), Some("system")),
        ]);

        assert_eq!(assignments[0].display_name, "Microphone");
        assert_eq!(assignments[0].source, SOURCE_LEGACY);
        assert_eq!(assignments[1].display_name, "System Audio");
    }

    #[test]
    fn falls_back_to_timing_based_detected_labels() {
        let assignments = derive_speaker_assignments(&[
            transcript("a", Some(0.0), Some(1.0), None),
            transcript("b", Some(4.0), Some(5.0), None),
            transcript("c", Some(5.2), Some(6.0), None),
        ]);

        assert_eq!(assignments[0].display_name, "Speaker 1");
        assert_eq!(assignments[1].display_name, "Speaker 2");
        assert_eq!(assignments[2].display_name, "Speaker 1");
        assert!(assignments
            .iter()
            .all(|assignment| assignment.source == SOURCE_HEURISTIC));
    }

    #[test]
    fn empty_transcripts_produce_no_assignments() {
        assert!(derive_speaker_assignments(&[]).is_empty());
    }

    #[test]
    fn normalizes_unknown_and_unsafe_legacy_values() {
        assert_eq!(normalize_legacy_speaker("unknown"), None);
        assert_eq!(
            normalize_legacy_speaker("Guest <script>"),
            Some("Speaker guest script".to_string())
        );
    }
}
