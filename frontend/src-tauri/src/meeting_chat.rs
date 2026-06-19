use crate::database::repositories::{meeting::MeetingsRepository, setting::SettingsRepository};
use crate::state::AppState;
use crate::summary::llm_client::{generate_summary, LLMProvider};
use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};
use std::path::PathBuf;
use std::sync::LazyLock;
use tauri::{AppHandle, Manager, State};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const MAX_CONTEXT_CHUNKS: usize = 12;
const MAX_CHUNK_CHARS: usize = 900;
const MAX_CONTEXT_CHARS: usize = 8_000;

static MEETING_CHAT_CANCEL_TOKEN: LazyLock<tokio::sync::Mutex<Option<CancellationToken>>> =
    LazyLock::new(|| tokio::sync::Mutex::new(None));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MeetingChatRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MeetingChatStatus {
    Pending,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingChatCitation {
    pub id: String,
    pub transcript_id: String,
    pub timestamp: String,
    pub audio_start_time: Option<f64>,
    pub audio_end_time: Option<f64>,
    pub excerpt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingChatMessage {
    pub id: String,
    pub meeting_id: String,
    pub role: MeetingChatRole,
    pub content: String,
    pub status: MeetingChatStatus,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub citations: Vec<MeetingChatCitation>,
    pub error: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskMeetingChatRequest {
    pub meeting_id: String,
    pub question: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskMeetingChatResponse {
    pub user_message: MeetingChatMessage,
    pub assistant_message: MeetingChatMessage,
}

#[derive(Debug, FromRow)]
struct MeetingChatMessageRow {
    id: String,
    meeting_id: String,
    role: String,
    content: String,
    status: String,
    provider: Option<String>,
    model: Option<String>,
    citations: Option<String>,
    error: Option<String>,
    created_at: String,
}

#[derive(Debug, FromRow)]
struct TranscriptContextRow {
    id: String,
    transcript: String,
    timestamp: String,
    audio_start_time: Option<f64>,
    audio_end_time: Option<f64>,
}

impl From<MeetingChatMessageRow> for MeetingChatMessage {
    fn from(row: MeetingChatMessageRow) -> Self {
        let citations = row
            .citations
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Vec<MeetingChatCitation>>(raw).ok())
            .unwrap_or_default();

        Self {
            id: row.id,
            meeting_id: row.meeting_id,
            role: match row.role.as_str() {
                "assistant" => MeetingChatRole::Assistant,
                _ => MeetingChatRole::User,
            },
            content: row.content,
            status: match row.status.as_str() {
                "completed" => MeetingChatStatus::Completed,
                "failed" => MeetingChatStatus::Failed,
                "canceled" => MeetingChatStatus::Canceled,
                _ => MeetingChatStatus::Pending,
            },
            provider: row.provider,
            model: row.model,
            citations,
            error: row.error,
            created_at: row.created_at,
        }
    }
}

pub struct MeetingChatRepository;

impl MeetingChatRepository {
    pub async fn list_messages(
        pool: &SqlitePool,
        meeting_id: &str,
    ) -> Result<Vec<MeetingChatMessage>, sqlx::Error> {
        let rows = sqlx::query_as::<_, MeetingChatMessageRow>(
            "SELECT id, meeting_id, role, content, status, provider, model, citations, error, created_at
             FROM meeting_chat_messages
             WHERE meeting_id = ?
             ORDER BY created_at ASC",
        )
        .bind(meeting_id)
        .fetch_all(pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn insert_message(
        pool: &SqlitePool,
        message: &MeetingChatMessage,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO meeting_chat_messages
             (id, meeting_id, role, content, status, provider, model, citations, error, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&message.id)
        .bind(&message.meeting_id)
        .bind(role_str(&message.role))
        .bind(&message.content)
        .bind(status_str(&message.status))
        .bind(&message.provider)
        .bind(&message.model)
        .bind(serde_json::to_string(&message.citations).unwrap_or_else(|_| "[]".to_string()))
        .bind(&message.error)
        .bind(&message.created_at)
        .execute(pool)
        .await?;

        Ok(())
    }
}

#[tauri::command]
pub async fn meeting_chat_list_messages(
    state: State<'_, AppState>,
    meeting_id: String,
) -> Result<Vec<MeetingChatMessage>, String> {
    MeetingChatRepository::list_messages(state.db_manager.pool(), &meeting_id)
        .await
        .map_err(|error| format!("Failed to load meeting chat history: {}", error))
}

#[tauri::command]
pub async fn meeting_chat_cancel() -> Result<(), String> {
    let mut guard = MEETING_CHAT_CANCEL_TOKEN.lock().await;
    if let Some(token) = guard.take() {
        token.cancel();
    }
    Ok(())
}

#[tauri::command]
pub async fn meeting_chat_ask(
    app: AppHandle,
    state: State<'_, AppState>,
    request: AskMeetingChatRequest,
) -> Result<AskMeetingChatResponse, String> {
    let question = request.question.trim();
    if question.is_empty() {
        return Err("Ask a question before sending.".to_string());
    }

    let pool = state.db_manager.pool();
    let meeting = MeetingsRepository::get_meeting_metadata(pool, &request.meeting_id)
        .await
        .map_err(|error| format!("Failed to load meeting: {}", error))?
        .ok_or_else(|| "Meeting not found.".to_string())?;

    let now = Utc::now().to_rfc3339();
    let user_message = MeetingChatMessage {
        id: format!("chat-{}", Uuid::new_v4()),
        meeting_id: request.meeting_id.clone(),
        role: MeetingChatRole::User,
        content: question.to_string(),
        status: MeetingChatStatus::Completed,
        provider: None,
        model: None,
        citations: Vec::new(),
        error: None,
        created_at: now.clone(),
    };
    MeetingChatRepository::insert_message(pool, &user_message)
        .await
        .map_err(|error| format!("Failed to save chat question: {}", error))?;

    let context = build_meeting_chat_context(pool, &request.meeting_id, question)
        .await
        .map_err(|error| format!("Failed to build meeting context: {}", error))?;

    let assistant_id = format!("chat-{}", Uuid::new_v4());
    let provider_settings = load_provider_settings(pool).await?;
    let cancellation_token = CancellationToken::new();
    {
        let mut guard = MEETING_CHAT_CANCEL_TOKEN.lock().await;
        if let Some(existing) = guard.take() {
            existing.cancel();
        }
        *guard = Some(cancellation_token.clone());
    }

    let answer_result = generate_chat_answer(
        &app,
        &provider_settings,
        &meeting.title,
        question,
        &context,
        &cancellation_token,
    )
    .await;

    {
        let mut guard = MEETING_CHAT_CANCEL_TOKEN.lock().await;
        *guard = None;
    }

    let (content, status, error) = match answer_result {
        Ok(answer) => (answer, MeetingChatStatus::Completed, None),
        Err(error) if error.to_lowercase().contains("cancel") => (
            "Meeting chat answer was canceled.".to_string(),
            MeetingChatStatus::Canceled,
            Some("Canceled by user.".to_string()),
        ),
        Err(error) => (
            fallback_cited_answer(question, &context),
            MeetingChatStatus::Failed,
            Some(error),
        ),
    };

    let assistant_message = MeetingChatMessage {
        id: assistant_id,
        meeting_id: request.meeting_id,
        role: MeetingChatRole::Assistant,
        content,
        status,
        provider: Some(provider_settings.provider),
        model: Some(provider_settings.model),
        citations: context.citations,
        error,
        created_at: Utc::now().to_rfc3339(),
    };
    MeetingChatRepository::insert_message(pool, &assistant_message)
        .await
        .map_err(|error| format!("Failed to save chat answer: {}", error))?;

    Ok(AskMeetingChatResponse {
        user_message,
        assistant_message,
    })
}

#[derive(Debug, Clone)]
struct ProviderSettings {
    provider: String,
    model: String,
    api_key: String,
    ollama_endpoint: Option<String>,
    custom_openai_endpoint: Option<String>,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    top_p: Option<f32>,
}

#[derive(Debug, Clone)]
struct MeetingChatContext {
    citations: Vec<MeetingChatCitation>,
    prompt_context: String,
}

async fn load_provider_settings(pool: &SqlitePool) -> Result<ProviderSettings, String> {
    let setting = SettingsRepository::get_model_config(pool)
        .await
        .map_err(|error| format!("Failed to load model settings: {}", error))?
        .ok_or_else(|| "Configure an AI model before using meeting chat.".to_string())?;
    let custom_config = setting.get_custom_openai_config();
    let provider = setting.provider;
    let model = custom_config
        .as_ref()
        .map(|config| config.model.clone())
        .unwrap_or_else(|| setting.model.clone());
    let api_key = SettingsRepository::get_api_key(pool, &provider)
        .await
        .map_err(|error| format!("Failed to load provider credentials: {}", error))?
        .or_else(|| {
            custom_config
                .as_ref()
                .and_then(|config| config.api_key.clone())
        })
        .unwrap_or_default();

    Ok(ProviderSettings {
        provider,
        model,
        api_key,
        ollama_endpoint: setting.ollama_endpoint,
        custom_openai_endpoint: custom_config.as_ref().map(|config| config.endpoint.clone()),
        max_tokens: custom_config
            .as_ref()
            .and_then(|config| config.max_tokens)
            .and_then(|value| u32::try_from(value).ok()),
        temperature: custom_config.as_ref().and_then(|config| config.temperature),
        top_p: custom_config.as_ref().and_then(|config| config.top_p),
    })
}

async fn generate_chat_answer(
    app: &AppHandle,
    settings: &ProviderSettings,
    meeting_title: &str,
    question: &str,
    context: &MeetingChatContext,
    cancellation_token: &CancellationToken,
) -> Result<String, String> {
    if context.citations.is_empty() {
        return Ok("I could not find transcript context for this meeting yet.".to_string());
    }

    let provider = LLMProvider::from_str(&settings.provider)?;
    let system_prompt = "You answer questions about one Meetily meeting. Use only the supplied context. Cite transcript evidence with citation ids like [T1]. If context is insufficient, say what is missing.";
    let user_prompt = format!(
        "Meeting: {meeting_title}\nQuestion: {question}\n\nContext excerpts:\n{context}\n\nReturn a concise answer with citations.",
        context = context.prompt_context
    );
    let app_data_dir: Option<PathBuf> = app.path().app_data_dir().ok();
    let client = Client::new();

    generate_summary(
        &client,
        &provider,
        &settings.model,
        &settings.api_key,
        system_prompt,
        &user_prompt,
        settings.ollama_endpoint.as_deref(),
        settings.custom_openai_endpoint.as_deref(),
        settings.max_tokens,
        settings.temperature,
        settings.top_p,
        app_data_dir.as_ref(),
        Some(cancellation_token),
    )
    .await
}

async fn build_meeting_chat_context(
    pool: &SqlitePool,
    meeting_id: &str,
    question: &str,
) -> Result<MeetingChatContext, sqlx::Error> {
    let transcripts = load_transcripts(pool, meeting_id).await?;
    Ok(build_context_from_transcripts(&transcripts, question))
}

async fn load_transcripts(
    pool: &SqlitePool,
    meeting_id: &str,
) -> Result<Vec<TranscriptContextRow>, sqlx::Error> {
    sqlx::query_as::<_, TranscriptContextRow>(
        "SELECT id, transcript, timestamp, audio_start_time, audio_end_time
         FROM transcripts
         WHERE meeting_id = ?
         ORDER BY COALESCE(audio_start_time, 999999999), timestamp ASC",
    )
    .bind(meeting_id)
    .fetch_all(pool)
    .await
}

fn build_context_from_transcripts(
    transcripts: &[TranscriptContextRow],
    question: &str,
) -> MeetingChatContext {
    let query_terms = query_terms(question);
    let mut scored: Vec<(usize, &TranscriptContextRow)> = transcripts
        .iter()
        .enumerate()
        .map(|(index, row)| (context_score(&row.transcript, &query_terms, index), row))
        .collect();
    scored.sort_by(|(left_score, _), (right_score, _)| right_score.cmp(left_score));

    let mut citations = Vec::new();
    let mut prompt_parts = Vec::new();
    let mut used_chars = 0usize;

    for (_, row) in scored.into_iter().take(MAX_CONTEXT_CHUNKS) {
        let excerpt = truncate_chars(row.transcript.trim(), MAX_CHUNK_CHARS);
        if excerpt.is_empty() {
            continue;
        }
        if used_chars + excerpt.len() > MAX_CONTEXT_CHARS {
            break;
        }
        let citation_id = format!("T{}", citations.len() + 1);
        prompt_parts.push(format!("[{}] {}: {}", citation_id, row.timestamp, excerpt));
        citations.push(MeetingChatCitation {
            id: citation_id,
            transcript_id: row.id.clone(),
            timestamp: row.timestamp.clone(),
            audio_start_time: row.audio_start_time,
            audio_end_time: row.audio_end_time,
            excerpt,
        });
        used_chars = prompt_parts.iter().map(|part| part.len()).sum();
    }

    MeetingChatContext {
        citations,
        prompt_context: prompt_parts.join("\n"),
    }
}

fn query_terms(question: &str) -> Vec<String> {
    question
        .split(|ch: char| !ch.is_alphanumeric())
        .map(|term| term.trim().to_lowercase())
        .filter(|term| term.len() >= 3)
        .filter(|term| {
            !matches!(
                term.as_str(),
                "the" | "and" | "for" | "with" | "that" | "what" | "when" | "where" | "about"
            )
        })
        .collect()
}

fn context_score(text: &str, query_terms: &[String], index: usize) -> usize {
    let lower = text.to_lowercase();
    let matches = query_terms
        .iter()
        .filter(|term| lower.contains(term.as_str()))
        .count();
    matches * 1000usize + (100usize.saturating_sub(index.min(100)))
}

fn fallback_cited_answer(question: &str, context: &MeetingChatContext) -> String {
    if context.citations.is_empty() {
        return "I could not find transcript context for this meeting yet.".to_string();
    }
    let cited = context
        .citations
        .iter()
        .take(3)
        .map(|citation| format!("[{}] {}", citation.id, citation.excerpt))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "I could not complete model routing for this question, but I found relevant meeting context for: \"{}\".\n\n{}",
        question, cited
    )
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut output = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        output.push_str("...");
    }
    output
}

fn role_str(role: &MeetingChatRole) -> &'static str {
    match role {
        MeetingChatRole::User => "user",
        MeetingChatRole::Assistant => "assistant",
    }
}

fn status_str(status: &MeetingChatStatus) -> &'static str {
    match status {
        MeetingChatStatus::Pending => "pending",
        MeetingChatStatus::Completed => "completed",
        MeetingChatStatus::Failed => "failed",
        MeetingChatStatus::Canceled => "canceled",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, text: &str, timestamp: &str) -> TranscriptContextRow {
        TranscriptContextRow {
            id: id.to_string(),
            transcript: text.to_string(),
            timestamp: timestamp.to_string(),
            audio_start_time: None,
            audio_end_time: None,
        }
    }

    #[test]
    fn context_prefers_query_matches_and_cites_timestamps() {
        let context = build_context_from_transcripts(
            &[
                row("t1", "We talked about lunch and travel.", "00:01"),
                row(
                    "t2",
                    "The production alert needs an observability fix.",
                    "00:02",
                ),
                row(
                    "t3",
                    "The observability dashboard should page the platform team.",
                    "00:03",
                ),
            ],
            "What observability actions did we discuss?",
        );

        assert_eq!(context.citations[0].transcript_id, "t2");
        assert!(context.prompt_context.contains("[T1] 00:02"));
        assert!(context.prompt_context.contains("[T2] 00:03"));
    }

    #[test]
    fn context_is_bounded_for_large_meetings() {
        let transcripts = (0..80)
            .map(|index| {
                row(
                    &format!("t{}", index),
                    &format!("observability {}", "long text ".repeat(200)),
                    &format!("00:{index:02}"),
                )
            })
            .collect::<Vec<_>>();

        let context = build_context_from_transcripts(&transcripts, "observability");

        assert!(context.citations.len() <= MAX_CONTEXT_CHUNKS);
        assert!(context.prompt_context.len() <= MAX_CONTEXT_CHARS + MAX_CHUNK_CHARS);
    }
}
