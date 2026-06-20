use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::Row;
use std::collections::{BTreeMap, HashMap};
use tauri::{AppHandle, Runtime};
use tauri_plugin_store::StoreExt;
use uuid::Uuid;

const SPEAKER_LABELING_STORE: &str = "speaker_labeling_preferences.json";
const SPEAKER_LABELING_STORE_KEY: &str = "preferences";
const SOURCE_HEURISTIC: &str = "heuristic";
const SOURCE_LEGACY: &str = "legacy";
const SOURCE_SCREENSHOT_NAME: &str = "screenshot_name";
const STATUS_DETECTED: &str = "detected";
const CONFIDENCE_LEGACY_SOURCE: f64 = 0.9;
const CONFIDENCE_TIMING_HEURISTIC: f64 = 0.45;
const CONFIDENCE_SCREENSHOT_NAME: f64 = 0.82;
const VISUAL_CUE_ALIGNMENT_WINDOW_SECONDS: f64 = 2.0;
const VISUAL_CUE_DISTANCE_PENALTY: f64 = 0.08;
const VISUAL_CUE_MIDPOINT_PENALTY: f64 = 0.01;
const VISUAL_CUE_DOMINANT_MARGIN: f64 = 0.05;
const VISUAL_CUE_REVIEW_CONFIDENCE: f64 = 0.7;
const VISUAL_CUE_AUTO_APPLY_CONFIDENCE: f64 = 0.9;

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
    pub visual_suggestions: Vec<SpeakerLabelSuggestion>,
    pub strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerLabelSuggestion {
    pub transcript_id: String,
    pub display_name: String,
    pub confidence: f64,
    pub start_time: Option<f64>,
    pub end_time: Option<f64>,
    pub source: String,
    pub snapshot_id: String,
    pub provider: Option<String>,
    pub active_marker: String,
    pub auto_applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SpeakerLabelingPreferences {
    pub auto_apply_visual_suggestions: bool,
}

impl Default for SpeakerLabelingPreferences {
    fn default() -> Self {
        Self {
            auto_apply_visual_suggestions: true,
        }
    }
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

#[derive(Debug, Clone, PartialEq)]
struct VisualSpeakerCue {
    snapshot_id: String,
    provider: Option<String>,
    recording_time: f64,
    extracted_name: String,
    active_marker: String,
    confidence: f64,
}

#[tauri::command]
pub async fn run_speaker_labeling<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<SpeakerLabelingResult, String> {
    let preferences = load_speaker_labeling_preferences(&app)?;
    run_speaker_labeling_for_meeting(state.inner(), &meeting_id, &preferences).await
}

#[tauri::command]
pub async fn get_speaker_labels<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
) -> Result<SpeakerLabelingResult, String> {
    load_speaker_labeling_result(state.inner(), &meeting_id, "stored").await
}

#[tauri::command]
pub async fn clear_speaker_labels<R: Runtime>(
    _app: AppHandle<R>,
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

#[tauri::command]
pub async fn update_speaker_label<R: Runtime>(
    _app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    label_id: String,
    display_name: String,
) -> Result<SpeakerLabel, String> {
    let display_name = normalize_display_name(&display_name)?;
    let pool = state.db_manager.pool();
    let now = Utc::now().to_rfc3339();

    let existing = sqlx::query(
        r#"
        SELECT id, meeting_id, display_name, source, status, confidence
        FROM speaker_labels
        WHERE id = ? AND deleted_at IS NULL
        "#,
    )
    .bind(&label_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to load speaker label: {}", err))?
    .ok_or_else(|| "Speaker label not found".to_string())?;

    let meeting_id: String = existing.get("meeting_id");
    let previous_name: String = existing.get("display_name");
    if previous_name == display_name {
        return Ok(row_to_speaker_label(existing));
    }

    let duplicate = sqlx::query(
        r#"
        SELECT id
        FROM speaker_labels
        WHERE meeting_id = ? AND display_name = ? AND id != ? AND deleted_at IS NULL
        "#,
    )
    .bind(&meeting_id)
    .bind(&display_name)
    .bind(&label_id)
    .fetch_optional(pool)
    .await
    .map_err(|err| format!("Failed to validate speaker label: {}", err))?;

    if duplicate.is_some() {
        return Err("A speaker with that name already exists".to_string());
    }

    let correction_id = Uuid::new_v4().to_string();
    let before_json = serde_json::json!({ "displayName": previous_name });
    let after_json = serde_json::json!({ "displayName": display_name });

    let mut tx = pool
        .begin()
        .await
        .map_err(|err| format!("Failed to start speaker correction transaction: {}", err))?;

    sqlx::query(
        r#"
        INSERT INTO speaker_corrections (id, meeting_id, action, before_json, after_json, created_at)
        VALUES (?, ?, 'rename', ?, ?, ?)
        "#,
    )
    .bind(&correction_id)
    .bind(&meeting_id)
    .bind(before_json.to_string())
    .bind(after_json.to_string())
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("Failed to store speaker correction: {}", err))?;

    sqlx::query(
        r#"
        UPDATE speaker_labels
        SET display_name = ?, status = 'confirmed', source = 'manual', updated_at = ?
        WHERE id = ?
        "#,
    )
    .bind(&display_name)
    .bind(&now)
    .bind(&label_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("Failed to update speaker label: {}", err))?;

    sqlx::query(
        r#"
        UPDATE transcript_speaker_segments
        SET source = 'manual', correction_id = ?, updated_at = ?
        WHERE speaker_label_id = ?
        "#,
    )
    .bind(&correction_id)
    .bind(&now)
    .bind(&label_id)
    .execute(&mut *tx)
    .await
    .map_err(|err| format!("Failed to update speaker segments: {}", err))?;

    tx.commit()
        .await
        .map_err(|err| format!("Failed to commit speaker correction: {}", err))?;

    let updated = sqlx::query(
        r#"
        SELECT id, meeting_id, display_name, source, status, confidence
        FROM speaker_labels
        WHERE id = ?
        "#,
    )
    .bind(&label_id)
    .fetch_one(pool)
    .await
    .map_err(|err| format!("Failed to load updated speaker label: {}", err))?;

    Ok(row_to_speaker_label(updated))
}

fn load_speaker_labeling_preferences<R: Runtime>(
    app: &AppHandle<R>,
) -> Result<SpeakerLabelingPreferences, String> {
    let store = app
        .store(SPEAKER_LABELING_STORE)
        .map_err(|err| format!("Failed to open speaker labeling preferences: {}", err))?;
    let Some(value) = store.get(SPEAKER_LABELING_STORE_KEY) else {
        return Ok(SpeakerLabelingPreferences::default());
    };
    match serde_json::from_value(value) {
        Ok(preferences) => Ok(preferences),
        Err(err) => {
            log::warn!(
                "Failed to read speaker labeling preferences; using defaults: {}",
                err
            );
            Ok(SpeakerLabelingPreferences::default())
        }
    }
}

#[tauri::command]
pub fn get_speaker_labeling_preferences<R: Runtime>(
    app: AppHandle<R>,
) -> Result<SpeakerLabelingPreferences, String> {
    load_speaker_labeling_preferences(&app)
}

#[tauri::command]
pub async fn set_speaker_labeling_preferences<R: Runtime>(
    app: AppHandle<R>,
    preferences: SpeakerLabelingPreferences,
) -> Result<SpeakerLabelingPreferences, String> {
    let saved_preferences = preferences.clone();
    tokio::task::spawn_blocking(move || -> Result<(), String> {
        let store = app
            .store(SPEAKER_LABELING_STORE)
            .map_err(|err| format!("Failed to open speaker labeling preferences: {}", err))?;
        store.set(
            SPEAKER_LABELING_STORE_KEY,
            serde_json::to_value(&saved_preferences).map_err(|err| {
                format!("Failed to serialize speaker labeling preferences: {}", err)
            })?,
        );
        store
            .save()
            .map_err(|err| format!("Failed to save speaker labeling preferences: {}", err))?;
        Ok(())
    })
    .await
    .map_err(|err| format!("Failed to save speaker labeling preferences: {}", err))??;
    Ok(preferences)
}

async fn run_speaker_labeling_for_meeting(
    state: &AppState,
    meeting_id: &str,
    preferences: &SpeakerLabelingPreferences,
) -> Result<SpeakerLabelingResult, String> {
    let pool = state.db_manager.pool();
    let transcripts = load_transcripts_for_labeling(pool, meeting_id).await?;
    let visible_name_hint = load_visible_speaker_name_hint(pool, meeting_id).await?;
    let visual_cues = load_visual_speaker_cues(pool, meeting_id).await?;
    let assignments = derive_speaker_assignments(
        &transcripts,
        visible_name_hint.as_deref(),
        &visual_cues,
        preferences,
    );
    let visual_suggestions =
        derive_visual_speaker_suggestions(&transcripts, &visual_cues, &assignments);
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
              AND source IN ('heuristic', 'legacy', 'screenshot_name')
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
          AND source IN ('heuristic', 'legacy', 'screenshot_name')
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

    let mut result =
        load_speaker_labeling_result(state, meeting_id, "local_timing_and_source").await?;
    result.visual_suggestions = visual_suggestions;
    Ok(result)
}

fn row_to_speaker_label(row: sqlx::sqlite::SqliteRow) -> SpeakerLabel {
    SpeakerLabel {
        id: row.get("id"),
        meeting_id: row.get("meeting_id"),
        display_name: row.get("display_name"),
        source: row.get("source"),
        status: row.get("status"),
        confidence: row.try_get("confidence").ok(),
    }
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

async fn load_visible_speaker_name_hint(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Option<String>, String> {
    let rows = sqlx::query(
        r#"
        SELECT metadata_json
        FROM meeting_screenshots
        WHERE meeting_id = ?
          AND deleted_at IS NULL
          AND status = 'captured'
          AND metadata_json IS NOT NULL
        ORDER BY COALESCE(recording_time, 0), captured_at
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load screenshot speaker hints: {}", err))?;

    let mut counts: HashMap<String, usize> = HashMap::new();
    for row in rows {
        let raw: String = row.get("metadata_json");
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let Some(names) = value
            .get("analysis")
            .and_then(|analysis| analysis.get("visibleNames"))
            .and_then(Value::as_array)
        else {
            continue;
        };

        for name in names {
            let Some(name) = name.as_str().and_then(normalize_screenshot_speaker_name) else {
                continue;
            };
            *counts.entry(name).or_insert(0) += 1;
        }
    }

    Ok(select_stable_visible_name(counts))
}

async fn load_visual_speaker_cues(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
) -> Result<Vec<VisualSpeakerCue>, String> {
    let rows = sqlx::query(
        r#"
        SELECT metadata_json
        FROM meeting_screenshots
        WHERE meeting_id = ?
          AND deleted_at IS NULL
          AND status = 'captured'
          AND metadata_json IS NOT NULL
        ORDER BY COALESCE(recording_time, 0), captured_at
        "#,
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
    .map_err(|err| format!("Failed to load visual speaker cues: {}", err))?;

    let mut cues = Vec::new();
    for row in rows {
        let raw: String = row.get("metadata_json");
        let Ok(value) = serde_json::from_str::<Value>(&raw) else {
            continue;
        };
        let Some(items) = value.get("speakerCues").and_then(Value::as_array) else {
            continue;
        };
        for item in items {
            let snapshot_id = item
                .get("snapshot_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let Some(recording_time) = item.get("recording_time").and_then(Value::as_f64) else {
                continue;
            };
            let Some(extracted_name) = item
                .get("extracted_name")
                .and_then(Value::as_str)
                .and_then(normalize_screenshot_speaker_name)
            else {
                continue;
            };
            let active_marker = item
                .get("active_marker")
                .and_then(Value::as_str)
                .unwrap_or("visible-name")
                .to_string();
            let provider = item
                .get("provider")
                .and_then(Value::as_str)
                .map(str::to_string);
            let confidence = item
                .get("confidence")
                .and_then(Value::as_f64)
                .unwrap_or(CONFIDENCE_SCREENSHOT_NAME)
                .clamp(0.0, 1.0);
            cues.push(VisualSpeakerCue {
                snapshot_id,
                provider,
                recording_time,
                extracted_name,
                active_marker,
                confidence,
            });
        }
    }

    cues.sort_by(|left, right| {
        left.recording_time
            .partial_cmp(&right.recording_time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(cues)
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

    let labels = label_rows
        .into_iter()
        .map(|row| SpeakerLabel {
            id: row.get("id"),
            meeting_id: row.get("meeting_id"),
            display_name: row.get("display_name"),
            source: row.get("source"),
            status: row.get("status"),
            confidence: row.try_get("confidence").ok(),
        })
        .collect::<Vec<_>>();
    let segments = segment_rows
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
        .collect::<Vec<_>>();
    let visual_suggestions =
        load_visual_speaker_suggestions(pool, meeting_id, &labels, &segments).await?;

    Ok(SpeakerLabelingResult {
        meeting_id: meeting_id.to_string(),
        labels,
        segments,
        visual_suggestions,
        strategy: strategy.to_string(),
    })
}

async fn load_visual_speaker_suggestions(
    pool: &sqlx::SqlitePool,
    meeting_id: &str,
    labels: &[SpeakerLabel],
    segments: &[TranscriptSpeakerSegment],
) -> Result<Vec<SpeakerLabelSuggestion>, String> {
    let transcripts = load_transcripts_for_labeling(pool, meeting_id).await?;
    let visual_cues = load_visual_speaker_cues(pool, meeting_id).await?;
    let labels_by_id = labels
        .iter()
        .map(|label| (label.id.as_str(), label))
        .collect::<HashMap<_, _>>();
    let assignments = segments
        .iter()
        .filter_map(|segment| {
            let label = labels_by_id.get(segment.speaker_label_id.as_str())?;
            Some(SpeakerAssignment {
                transcript_id: segment.transcript_id.clone(),
                display_name: label.display_name.clone(),
                source: segment.source.clone(),
                confidence: segment.confidence.unwrap_or(0.0),
                start_time: segment.start_time,
                end_time: segment.end_time,
            })
        })
        .collect::<Vec<_>>();
    Ok(derive_visual_speaker_suggestions(
        &transcripts,
        &visual_cues,
        &assignments,
    ))
}

fn derive_speaker_assignments(
    transcripts: &[TranscriptForLabeling],
    visible_name_hint: Option<&str>,
    visual_cues: &[VisualSpeakerCue],
    preferences: &SpeakerLabelingPreferences,
) -> Vec<SpeakerAssignment> {
    transcripts
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            let visual_cue = best_visual_cue_for_segment(segment, visual_cues);
            if let Some(cue) = visual_cue
                .filter(|_| preferences.auto_apply_visual_suggestions)
                .filter(|cue| effective_visual_cue_confidence(cue) >= VISUAL_CUE_REVIEW_CONFIDENCE)
                .filter(|cue| should_apply_visual_cue(segment, cue))
            {
                return SpeakerAssignment {
                    transcript_id: segment.id.clone(),
                    display_name: cue.extracted_name.clone(),
                    source: SOURCE_SCREENSHOT_NAME.to_string(),
                    confidence: effective_visual_cue_confidence(cue),
                    start_time: segment.audio_start_time,
                    end_time: segment.audio_end_time,
                };
            }

            if let Some(legacy_speaker) = segment
                .legacy_speaker
                .as_deref()
                .and_then(normalize_legacy_speaker)
            {
                if is_microphone_source(&legacy_speaker) {
                    if let Some(display_name) =
                        visible_name_hint.and_then(normalize_screenshot_speaker_name)
                    {
                        return SpeakerAssignment {
                            transcript_id: segment.id.clone(),
                            display_name,
                            source: SOURCE_SCREENSHOT_NAME.to_string(),
                            confidence: CONFIDENCE_SCREENSHOT_NAME,
                            start_time: segment.audio_start_time,
                            end_time: segment.audio_end_time,
                        };
                    }
                }

                return SpeakerAssignment {
                    transcript_id: segment.id.clone(),
                    display_name: legacy_speaker,
                    source: SOURCE_LEGACY.to_string(),
                    confidence: CONFIDENCE_LEGACY_SOURCE,
                    start_time: segment.audio_start_time,
                    end_time: segment.audio_end_time,
                };
            }

            if let Some(display_name) =
                visible_name_hint.and_then(normalize_screenshot_speaker_name)
            {
                return SpeakerAssignment {
                    transcript_id: segment.id.clone(),
                    display_name,
                    source: SOURCE_SCREENSHOT_NAME.to_string(),
                    confidence: CONFIDENCE_SCREENSHOT_NAME,
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

fn derive_visual_speaker_suggestions(
    transcripts: &[TranscriptForLabeling],
    visual_cues: &[VisualSpeakerCue],
    assignments: &[SpeakerAssignment],
) -> Vec<SpeakerLabelSuggestion> {
    let assignments_by_transcript_id = assignments
        .iter()
        .map(|assignment| (assignment.transcript_id.as_str(), assignment))
        .collect::<HashMap<_, _>>();

    transcripts
        .iter()
        .filter_map(|segment| {
            let cue = best_visual_cue_for_segment(segment, visual_cues)?;
            let confidence = effective_visual_cue_confidence(cue);
            if confidence < VISUAL_CUE_REVIEW_CONFIDENCE {
                return None;
            }
            let auto_applied = assignments_by_transcript_id
                .get(segment.id.as_str())
                .map(|assignment| {
                    assignment.source == SOURCE_SCREENSHOT_NAME
                        && assignment.display_name == cue.extracted_name
                })
                .unwrap_or(false);
            Some(SpeakerLabelSuggestion {
                transcript_id: segment.id.clone(),
                display_name: cue.extracted_name.clone(),
                confidence,
                start_time: segment.audio_start_time,
                end_time: segment.audio_end_time,
                source: SOURCE_SCREENSHOT_NAME.to_string(),
                snapshot_id: cue.snapshot_id.clone(),
                provider: cue.provider.clone(),
                active_marker: cue.active_marker.clone(),
                auto_applied,
            })
        })
        .collect()
}

fn best_visual_cue_for_segment<'a>(
    segment: &TranscriptForLabeling,
    visual_cues: &'a [VisualSpeakerCue],
) -> Option<&'a VisualSpeakerCue> {
    let start = segment.audio_start_time?;
    let end = segment.audio_end_time.unwrap_or(start).max(start);
    let midpoint = (start + end) / 2.0;

    let mut ranked = visual_cues
        .iter()
        .filter_map(|cue| {
            let distance = if cue.recording_time < start {
                start - cue.recording_time
            } else if cue.recording_time > end {
                cue.recording_time - end
            } else {
                0.0
            };
            if distance > VISUAL_CUE_ALIGNMENT_WINDOW_SECONDS {
                return None;
            }
            // Screenshot cue recording_time and transcript audio times are both
            // recording-relative seconds from the same session start.
            let midpoint_distance = (cue.recording_time - midpoint).abs();
            let score = (effective_visual_cue_confidence(cue)
                - distance * VISUAL_CUE_DISTANCE_PENALTY
                - midpoint_distance * VISUAL_CUE_MIDPOINT_PENALTY)
                .max(0.0);
            Some((cue, score, midpoint_distance))
        })
        .collect::<Vec<_>>();

    ranked.sort_by(
        |(_, left_score, left_distance), (_, right_score, right_distance)| {
            right_score
                .partial_cmp(left_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left_distance
                        .partial_cmp(right_distance)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        },
    );

    let (best, best_score, _) = ranked.first().copied()?;
    let best_name = best.extracted_name.to_lowercase();
    let has_near_tie = ranked.iter().skip(1).any(|(cue, score, _)| {
        cue.extracted_name.to_lowercase() != best_name
            && (best_score - score).abs() < VISUAL_CUE_DOMINANT_MARGIN
    });
    if has_near_tie {
        return None;
    }

    Some(best)
}

fn should_apply_visual_cue(segment: &TranscriptForLabeling, cue: &VisualSpeakerCue) -> bool {
    let Some(legacy_speaker) = segment
        .legacy_speaker
        .as_deref()
        .and_then(normalize_legacy_speaker)
    else {
        return true;
    };

    is_microphone_source(&legacy_speaker)
        || (cue.active_marker != "visible-name"
            && effective_visual_cue_confidence(cue) >= VISUAL_CUE_AUTO_APPLY_CONFIDENCE)
}

fn effective_visual_cue_confidence(cue: &VisualSpeakerCue) -> f64 {
    (cue.confidence * visual_provider_reliability(cue.provider.as_deref())).clamp(0.0, 1.0)
}

fn visual_provider_reliability(provider: Option<&str>) -> f64 {
    match provider
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
        .as_deref()
    {
        Some(
            "google-meet" | "zoom" | "microsoft-teams" | "teams" | "facetime" | "webex" | "slack",
        ) => 1.0,
        Some(_) => 0.9,
        None => 0.85,
    }
}

fn select_stable_visible_name(counts: HashMap<String, usize>) -> Option<String> {
    let mut counts = counts.into_iter().collect::<Vec<_>>();
    counts.sort_by(|(left_name, left_count), (right_name, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_name.cmp(right_name))
    });

    let (name, count) = counts.first()?;
    if *count == 0 {
        return None;
    }

    if counts
        .get(1)
        .map(|(_, second_count)| second_count == count)
        .unwrap_or(false)
    {
        return None;
    }

    Some(name.clone())
}

fn normalize_screenshot_speaker_name(value: &str) -> Option<String> {
    let cleaned = value
        .chars()
        .filter(|ch| {
            ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || *ch == '-' || *ch == '\''
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let truncated = cleaned.chars().take(48).collect::<String>();
    if truncated.is_empty() {
        None
    } else {
        Some(truncated)
    }
}

fn is_microphone_source(display_name: &str) -> bool {
    matches!(
        display_name.trim().to_lowercase().as_str(),
        "microphone" | "mic"
    )
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

fn normalize_display_name(value: &str) -> Result<String, String> {
    let cleaned = value
        .chars()
        .filter(|ch| {
            ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || *ch == '-' || *ch == '_'
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let truncated = cleaned.chars().take(48).collect::<String>();

    if truncated.is_empty() {
        Err("Speaker name cannot be empty".to_string())
    } else {
        Ok(truncated)
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

    fn visual_cue(name: &str, time: f64, confidence: f64) -> VisualSpeakerCue {
        VisualSpeakerCue {
            snapshot_id: format!("shot-{time}"),
            provider: Some("google-meet".to_string()),
            recording_time: time,
            extracted_name: name.to_string(),
            active_marker: "caption-label".to_string(),
            confidence,
        }
    }

    #[test]
    fn uses_legacy_audio_source_when_available() {
        let assignments = derive_speaker_assignments(
            &[
                transcript("a", Some(0.0), Some(1.0), Some("mic")),
                transcript("b", Some(1.2), Some(2.0), Some("system")),
            ],
            None,
            &[],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Microphone");
        assert_eq!(assignments[0].source, SOURCE_LEGACY);
        assert_eq!(assignments[1].display_name, "System Audio");
    }

    #[test]
    fn falls_back_to_timing_based_detected_labels() {
        let assignments = derive_speaker_assignments(
            &[
                transcript("a", Some(0.0), Some(1.0), None),
                transcript("b", Some(4.0), Some(5.0), None),
                transcript("c", Some(5.2), Some(6.0), None),
            ],
            None,
            &[],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Speaker 1");
        assert_eq!(assignments[1].display_name, "Speaker 2");
        assert_eq!(assignments[2].display_name, "Speaker 1");
        assert!(assignments
            .iter()
            .all(|assignment| assignment.source == SOURCE_HEURISTIC));
    }

    #[test]
    fn empty_transcripts_produce_no_assignments() {
        assert!(
            derive_speaker_assignments(&[], None, &[], &SpeakerLabelingPreferences::default())
                .is_empty()
        );
    }

    #[test]
    fn normalizes_unknown_and_unsafe_legacy_values() {
        assert_eq!(normalize_legacy_speaker("unknown"), None);
        assert_eq!(
            normalize_legacy_speaker("Guest <script>"),
            Some("Speaker guest script".to_string())
        );
    }

    #[test]
    fn visible_screenshot_name_overrides_generic_microphone_label() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(0.0), Some(1.0), Some("mic"))],
            Some("Adrian Witaszak"),
            &[],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Adrian Witaszak");
        assert_eq!(assignments[0].source, SOURCE_SCREENSHOT_NAME);
        assert_eq!(assignments[0].confidence, CONFIDENCE_SCREENSHOT_NAME);
    }

    #[test]
    fn visual_cues_align_before_during_and_after_segments() {
        let assignments = derive_speaker_assignments(
            &[
                transcript("a", Some(5.0), Some(8.0), None),
                transcript("b", Some(10.0), Some(12.0), None),
                transcript("c", Some(15.0), Some(17.0), None),
            ],
            None,
            &[
                visual_cue("Before Speaker", 4.0, 0.9),
                visual_cue("During Speaker", 11.0, 0.91),
                visual_cue("After Speaker", 18.0, 0.92),
            ],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Before Speaker");
        assert_eq!(assignments[1].display_name, "During Speaker");
        assert_eq!(assignments[2].display_name, "After Speaker");
        assert!(assignments
            .iter()
            .all(|assignment| assignment.source == SOURCE_SCREENSHOT_NAME));
    }

    #[test]
    fn multiple_visual_cues_rank_by_confidence_and_preserve_ambiguity() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), None)],
            None,
            &[
                visual_cue("Lower Confidence", 2.0, 0.7),
                visual_cue("Higher Confidence", 2.2, 0.9),
            ],
            &SpeakerLabelingPreferences::default(),
        );
        assert_eq!(assignments[0].display_name, "Higher Confidence");

        let ambiguous = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), None)],
            None,
            &[
                visual_cue("First Speaker", 2.0, 0.9),
                visual_cue("Second Speaker", 2.0, 0.91),
            ],
            &SpeakerLabelingPreferences::default(),
        );
        assert_eq!(ambiguous[0].source, SOURCE_HEURISTIC);
    }

    #[test]
    fn same_speaker_visual_cue_bursts_are_not_ambiguous() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), None)],
            None,
            &[
                visual_cue("Same Speaker", 1.9, 0.9),
                visual_cue("Same Speaker", 2.1, 0.9),
            ],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Same Speaker");
        assert_eq!(assignments[0].source, SOURCE_SCREENSHOT_NAME);
    }

    #[test]
    fn missing_visual_cues_leave_existing_label_fallback_unchanged() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), Some("mic"))],
            Some("Stable Name"),
            &[],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Stable Name");
        assert_eq!(assignments[0].source, SOURCE_SCREENSHOT_NAME);
    }

    #[test]
    fn weak_visual_cues_do_not_override_system_audio_labels() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), Some("system"))],
            None,
            &[VisualSpeakerCue {
                snapshot_id: "shot-1".to_string(),
                provider: Some("google-meet".to_string()),
                recording_time: 2.0,
                extracted_name: "Visible Person".to_string(),
                active_marker: "visible-name".to_string(),
                confidence: 0.65,
            }],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "System Audio");
        assert_eq!(assignments[0].source, SOURCE_LEGACY);
    }

    #[test]
    fn review_only_preferences_do_not_apply_visual_suggestions() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), None)],
            None,
            &[visual_cue("Visible Person", 2.0, 0.96)],
            &SpeakerLabelingPreferences {
                auto_apply_visual_suggestions: false,
            },
        );

        assert_eq!(assignments[0].display_name, "Speaker 1");
        assert_eq!(assignments[0].source, SOURCE_HEURISTIC);
    }

    #[test]
    fn visual_suggestions_survive_review_only_mode() {
        let transcripts = vec![transcript("a", Some(1.0), Some(3.0), None)];
        let cues = vec![visual_cue("Visible Person", 2.0, 0.96)];
        let assignments = derive_speaker_assignments(
            &transcripts,
            None,
            &cues,
            &SpeakerLabelingPreferences {
                auto_apply_visual_suggestions: false,
            },
        );
        let suggestions = derive_visual_speaker_suggestions(&transcripts, &cues, &assignments);

        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].display_name, "Visible Person");
        assert!(!suggestions[0].auto_applied);
        assert_eq!(suggestions[0].transcript_id, "a");
    }

    #[test]
    fn provider_reliability_can_demote_low_quality_visual_cues() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), None)],
            None,
            &[VisualSpeakerCue {
                snapshot_id: "shot-1".to_string(),
                provider: Some("unknown-window".to_string()),
                recording_time: 2.0,
                extracted_name: "Visible Person".to_string(),
                active_marker: "caption-label".to_string(),
                confidence: 0.74,
            }],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Speaker 1");
        assert_eq!(assignments[0].source, SOURCE_HEURISTIC);
    }

    #[test]
    fn high_confidence_visual_cues_override_generic_legacy_labels() {
        let assignments = derive_speaker_assignments(
            &[transcript("a", Some(1.0), Some(3.0), Some("system"))],
            None,
            &[visual_cue("Visible Person", 2.0, 0.95)],
            &SpeakerLabelingPreferences::default(),
        );

        assert_eq!(assignments[0].display_name, "Visible Person");
        assert_eq!(assignments[0].source, SOURCE_SCREENSHOT_NAME);
        assert!(assignments[0].confidence >= VISUAL_CUE_AUTO_APPLY_CONFIDENCE);
    }

    #[test]
    fn stable_visible_name_requires_no_top_count_tie() {
        let mut counts = HashMap::new();
        counts.insert("Adrian Witaszak".to_string(), 2);
        counts.insert("Kriszi Balla".to_string(), 2);
        assert_eq!(select_stable_visible_name(counts), None);

        let mut counts = HashMap::new();
        counts.insert("Adrian Witaszak".to_string(), 3);
        counts.insert("Kriszi Balla".to_string(), 1);
        assert_eq!(
            select_stable_visible_name(counts),
            Some("Adrian Witaszak".to_string())
        );
    }
}
