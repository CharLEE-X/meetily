use crate::database::repositories::{
    meeting::MeetingsRepository, summary::SummaryProcessesRepository,
};
use crate::state::AppState;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager, Runtime};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Markdown,
    Pdf,
    Docx,
}

impl ExportFormat {
    fn extension(self) -> &'static str {
        match self {
            ExportFormat::Markdown => "md",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Docx => "docx",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSections {
    pub metadata: bool,
    pub summary: bool,
    pub action_items: bool,
    pub transcript: bool,
}

impl Default for ExportSections {
    fn default() -> Self {
        Self {
            metadata: true,
            summary: true,
            action_items: true,
            transcript: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportSettings {
    pub default_format: ExportFormat,
    pub sections: ExportSections,
    pub auto_export_enabled: bool,
    pub auto_export_format: ExportFormat,
    pub destination_dir: Option<String>,
    pub file_name_template: String,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            default_format: ExportFormat::Markdown,
            sections: ExportSections::default(),
            auto_export_enabled: false,
            auto_export_format: ExportFormat::Markdown,
            destination_dir: None,
            file_name_template: "{title}-{date}".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportMeetingOptions {
    pub format: ExportFormat,
    pub sections: ExportSections,
    pub destination_dir: Option<String>,
    pub file_name: Option<String>,
    pub auto_export: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub meeting_id: String,
    pub format: ExportFormat,
    pub file_path: String,
    pub byte_size: u64,
    pub created_at: String,
    pub sections: ExportSections,
    pub auto_export: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportHistoryEntry {
    pub meeting_id: String,
    pub format: ExportFormat,
    pub file_path: String,
    pub byte_size: u64,
    pub created_at: String,
    pub auto_export: bool,
}

#[derive(Debug)]
struct ExportPayload {
    meeting_id: String,
    title: String,
    created_at: String,
    updated_at: String,
    summary: Option<Value>,
    transcripts: Vec<ExportTranscript>,
}

#[derive(Debug)]
struct ExportTranscript {
    text: String,
    timestamp: String,
    audio_start_time: Option<f64>,
}

#[tauri::command]
pub async fn export_get_settings<R: Runtime>(app: AppHandle<R>) -> Result<ExportSettings, String> {
    read_settings(&app)
}

#[tauri::command]
pub async fn export_update_settings<R: Runtime>(
    app: AppHandle<R>,
    settings: ExportSettings,
) -> Result<ExportSettings, String> {
    let sanitized = ExportSettings {
        file_name_template: if settings.file_name_template.trim().is_empty() {
            ExportSettings::default().file_name_template
        } else {
            settings.file_name_template
        },
        ..settings
    };
    write_json(&settings_path(&app)?, &sanitized)?;
    Ok(sanitized)
}

#[tauri::command]
pub async fn export_meeting<R: Runtime>(
    app: AppHandle<R>,
    state: tauri::State<'_, AppState>,
    meeting_id: String,
    options: ExportMeetingOptions,
) -> Result<ExportResult, String> {
    let payload = load_payload(&state, &meeting_id).await?;
    let default_settings = read_settings(&app).unwrap_or_default();
    let destination_dir = resolve_destination_dir(&app, options.destination_dir.as_ref())?;
    fs::create_dir_all(&destination_dir)
        .map_err(|err| format!("Failed to create export folder: {}", err))?;

    let base_name = options
        .file_name
        .as_deref()
        .unwrap_or(&default_settings.file_name_template);
    let file_name = render_file_name(base_name, &payload.title, options.format);
    let file_path = next_available_path(destination_dir.join(file_name));
    let bytes = render_export(&payload, options.format, &options.sections)?;
    fs::write(&file_path, &bytes).map_err(|err| format!("Failed to write export: {}", err))?;

    let created_at = Utc::now().to_rfc3339();
    let result = ExportResult {
        meeting_id: meeting_id.clone(),
        format: options.format,
        file_path: file_path.to_string_lossy().to_string(),
        byte_size: bytes.len() as u64,
        created_at: created_at.clone(),
        sections: options.sections,
        auto_export: options.auto_export.unwrap_or(false),
    };

    append_history(
        &app,
        ExportHistoryEntry {
            meeting_id,
            format: result.format,
            file_path: result.file_path.clone(),
            byte_size: result.byte_size,
            created_at,
            auto_export: result.auto_export,
        },
    )?;

    Ok(result)
}

#[tauri::command]
pub async fn export_get_history<R: Runtime>(
    app: AppHandle<R>,
    meeting_id: Option<String>,
) -> Result<Vec<ExportHistoryEntry>, String> {
    let history = read_history(&app)?;
    Ok(match meeting_id {
        Some(id) => history
            .into_iter()
            .filter(|entry| entry.meeting_id == id)
            .collect(),
        None => history,
    })
}

async fn load_payload(
    state: &tauri::State<'_, AppState>,
    meeting_id: &str,
) -> Result<ExportPayload, String> {
    let pool = state.db_manager.pool();
    let meeting = MeetingsRepository::get_meeting_metadata(pool, meeting_id)
        .await
        .map_err(|err| format!("Failed to load meeting metadata: {}", err))?
        .ok_or_else(|| "Meeting not found".to_string())?;

    let (transcripts, _) =
        MeetingsRepository::get_meeting_transcripts_paginated(pool, meeting_id, i64::MAX, 0)
            .await
            .map_err(|err| format!("Failed to load transcripts: {}", err))?;

    let summary = SummaryProcessesRepository::get_summary_data(pool, meeting_id)
        .await
        .map_err(|err| format!("Failed to load summary: {}", err))?
        .and_then(|process| process.result)
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok());

    Ok(ExportPayload {
        meeting_id: meeting.id,
        title: meeting.title,
        created_at: meeting.created_at.0.to_rfc3339(),
        updated_at: meeting.updated_at.0.to_rfc3339(),
        summary,
        transcripts: transcripts
            .into_iter()
            .map(|segment| ExportTranscript {
                text: segment.transcript,
                timestamp: segment.timestamp,
                audio_start_time: segment.audio_start_time,
            })
            .collect(),
    })
}

fn app_export_dir<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|err| format!("Failed to resolve app data folder: {}", err))?
        .join("exports"))
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|err| format!("Failed to resolve app data folder: {}", err))?
        .join("export_settings.json"))
}

fn history_path<R: Runtime>(app: &AppHandle<R>) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|err| format!("Failed to resolve app data folder: {}", err))?
        .join("export_history.json"))
}

fn read_settings<R: Runtime>(app: &AppHandle<R>) -> Result<ExportSettings, String> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(ExportSettings::default());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read export settings: {}", err))?;
    serde_json::from_str(&raw).map_err(|err| format!("Failed to parse export settings: {}", err))
}

fn read_history<R: Runtime>(app: &AppHandle<R>) -> Result<Vec<ExportHistoryEntry>, String> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("Failed to read export history: {}", err))?;
    serde_json::from_str(&raw).map_err(|err| format!("Failed to parse export history: {}", err))
}

fn append_history<R: Runtime>(app: &AppHandle<R>, entry: ExportHistoryEntry) -> Result<(), String> {
    let mut history = read_history(app).unwrap_or_default();
    history.insert(0, entry);
    history.truncate(50);
    write_json(&history_path(app)?, &history)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Failed to create settings folder: {}", err))?;
    }
    let json = serde_json::to_string_pretty(value)
        .map_err(|err| format!("Failed to serialize export data: {}", err))?;
    fs::write(path, json).map_err(|err| format!("Failed to write export data: {}", err))
}

fn resolve_destination_dir<R: Runtime>(
    app: &AppHandle<R>,
    configured: Option<&String>,
) -> Result<PathBuf, String> {
    if let Some(path) = configured {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }
    app_export_dir(app)
}

fn render_file_name(template: &str, title: &str, format: ExportFormat) -> String {
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let rendered = template
        .replace("{title}", title)
        .replace("{date}", &date)
        .replace("{format}", format.extension());
    let stem = sanitize_file_stem(rendered.trim_end_matches(&format!(".{}", format.extension())));
    format!("{}.{}", stem, format.extension())
}

fn sanitize_file_stem(input: &str) -> String {
    let cleaned = input
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            ch if ch.is_control() => '-',
            ch => ch,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    let trimmed = cleaned.trim_matches([' ', '.', '-']).trim();
    if trimmed.is_empty() {
        "meeting-export".to_string()
    } else {
        trimmed.to_string()
    }
}

fn next_available_path(path: PathBuf) -> PathBuf {
    if !path.exists() {
        return path;
    }

    let parent = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("meeting-export");
    let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

    for index in 1..1000 {
        let candidate = parent.join(format!("{}-{}.{}", stem, index, extension));
        if !candidate.exists() {
            return candidate;
        }
    }

    parent.join(format!("{}-{}.{}", stem, Utc::now().timestamp(), extension))
}

fn render_export(
    payload: &ExportPayload,
    format: ExportFormat,
    sections: &ExportSections,
) -> Result<Vec<u8>, String> {
    match format {
        ExportFormat::Markdown => Ok(render_markdown(payload, sections).into_bytes()),
        ExportFormat::Pdf => Ok(render_pdf(payload, sections)),
        ExportFormat::Docx => render_docx(payload, sections),
    }
}

fn render_markdown(payload: &ExportPayload, sections: &ExportSections) -> String {
    let mut lines = vec![format!("# {}", payload.title), String::new()];

    if sections.metadata {
        lines.extend([
            "## Metadata".to_string(),
            format!("- Meeting ID: {}", payload.meeting_id),
            format!("- Created: {}", payload.created_at),
            format!("- Updated: {}", payload.updated_at),
            String::new(),
        ]);
    }

    if sections.summary {
        lines.push("## Summary".to_string());
        lines.push(summary_text(payload.summary.as_ref(), true));
        lines.push(String::new());
    }

    if sections.action_items {
        let action_items = summary_action_items(payload.summary.as_ref());
        if !action_items.is_empty() {
            lines.push("## Action Items".to_string());
            lines.extend(action_items.into_iter().map(|item| format!("- {}", item)));
            lines.push(String::new());
        }
    }

    if sections.transcript {
        lines.push("## Transcript".to_string());
        for segment in &payload.transcripts {
            lines.push(format!(
                "- [{}] {}",
                segment
                    .audio_start_time
                    .map(format_seconds)
                    .unwrap_or_else(|| segment.timestamp.clone()),
                segment.text.trim()
            ));
        }
        lines.push(String::new());
    }

    lines.join("\n")
}

fn render_pdf(payload: &ExportPayload, sections: &ExportSections) -> Vec<u8> {
    let markdown = render_markdown(payload, sections);
    let lines = wrap_lines(&markdown.replace('#', ""), 92);
    let pages = lines
        .chunks(52)
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();
    let page_count = pages.len().max(1);
    let mut objects = Vec::new();

    objects.push("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string());
    let kids = (0..page_count)
        .map(|idx| format!("{} 0 R", 3 + idx * 2))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!(
        "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        kids, page_count
    ));

    for idx in 0..page_count {
        let page_obj = 3 + idx * 2;
        let content_obj = page_obj + 1;
        let content = pdf_content_stream(pages.get(idx).cloned().unwrap_or_default());
        objects.push(format!(
            "{} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents {} 0 R >>\nendobj\n",
            page_obj, content_obj
        ));
        objects.push(format!(
            "{} 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
            content_obj,
            content.as_bytes().len(),
            content
        ));
    }

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize];
    for object in &objects {
        offsets.push(pdf.len());
        pdf.extend_from_slice(object.as_bytes());
    }
    let xref_start = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).as_bytes());
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{:010} 00000 n \n", offset).as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            offsets.len(),
            xref_start
        )
        .as_bytes(),
    );
    pdf
}

fn pdf_content_stream(lines: Vec<String>) -> String {
    let mut stream = "BT\n/F1 11 Tf\n50 748 Td\n14 TL\n".to_string();
    for line in lines {
        stream.push_str(&format!("({}) Tj\nT*\n", escape_pdf_text(&line)));
    }
    stream.push_str("ET");
    stream
}

fn render_docx(payload: &ExportPayload, sections: &ExportSections) -> Result<Vec<u8>, String> {
    let markdown = render_markdown(payload, sections);
    let mut buffer = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(&mut buffer);
    let options = SimpleFileOptions::default();

    zip.start_file("[Content_Types].xml", options)
        .map_err(|err| err.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#).map_err(|err| err.to_string())?;

    zip.add_directory("_rels/", options)
        .map_err(|err| err.to_string())?;
    zip.start_file("_rels/.rels", options)
        .map_err(|err| err.to_string())?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#).map_err(|err| err.to_string())?;

    zip.add_directory("word/", options)
        .map_err(|err| err.to_string())?;
    zip.start_file("word/document.xml", options)
        .map_err(|err| err.to_string())?;
    zip.write_all(docx_document_xml(&markdown).as_bytes())
        .map_err(|err| err.to_string())?;
    zip.finish().map_err(|err| err.to_string())?;
    Ok(buffer.into_inner())
}

fn docx_document_xml(markdown: &str) -> String {
    let mut body = String::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            body.push_str("<w:p/>");
            continue;
        }

        let (style, text) = if let Some(text) = trimmed.strip_prefix("# ") {
            ("Title", text)
        } else if let Some(text) = trimmed.strip_prefix("## ") {
            ("Heading1", text)
        } else if let Some(text) = trimmed.strip_prefix("- ") {
            ("ListParagraph", text)
        } else {
            ("Normal", trimmed)
        };

        body.push_str(&format!(
            r#"<w:p><w:pPr><w:pStyle w:val="{}"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
            style,
            escape_xml(text)
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{}<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440"/></w:sectPr></w:body></w:document>"#,
        body
    )
}

fn summary_text(summary: Option<&Value>, exclude_action_items: bool) -> String {
    let Some(summary) = summary else {
        return "No summary is available yet.".to_string();
    };

    if let Some(markdown) = summary.get("markdown").and_then(Value::as_str) {
        return markdown.to_string();
    }

    if let Some(text) = summary.get("raw_summary").and_then(Value::as_str) {
        return text.to_string();
    }

    let mut lines = Vec::new();
    if let Some(order) = summary.get("_section_order").and_then(Value::as_array) {
        for key in order.iter().filter_map(Value::as_str) {
            if exclude_action_items && key.to_lowercase().contains("action") {
                continue;
            }
            append_summary_section(summary, key, &mut lines);
        }
    } else if let Some(map) = summary.as_object() {
        for key in map.keys() {
            if key.starts_with('_') || key == "MeetingName" || key == "summary_json" {
                continue;
            }
            if exclude_action_items && key.to_lowercase().contains("action") {
                continue;
            }
            append_summary_section(summary, key, &mut lines);
        }
    }

    if lines.is_empty() {
        "No summary is available yet.".to_string()
    } else {
        lines.join("\n")
    }
}

fn append_summary_section(summary: &Value, key: &str, lines: &mut Vec<String>) {
    let Some(section) = summary.get(key) else {
        return;
    };
    let title = section
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or(key)
        .replace('_', " ");
    lines.push(format!("### {}", title));

    if let Some(blocks) = section.get("blocks").and_then(Value::as_array) {
        for block in blocks {
            if let Some(content) = block.get("content").and_then(Value::as_str) {
                lines.push(format!("- {}", content));
            }
        }
    } else if let Some(text) = section.as_str() {
        lines.push(text.to_string());
    }
    lines.push(String::new());
}

fn summary_action_items(summary: Option<&Value>) -> Vec<String> {
    let Some(summary) = summary else {
        return Vec::new();
    };
    let mut items = Vec::new();

    if let Some(section) = summary
        .get("action_items")
        .or_else(|| summary.get("Action Items"))
    {
        if let Some(blocks) = section.get("blocks").and_then(Value::as_array) {
            for block in blocks {
                if let Some(content) = block.get("content").and_then(Value::as_str) {
                    items.push(content.to_string());
                }
            }
        } else if let Some(text) = section.as_str() {
            items.push(text.to_string());
        }
    }

    items
}

fn wrap_lines(input: &str, max_chars: usize) -> Vec<String> {
    let mut output = Vec::new();
    for line in input.lines() {
        let mut current = String::new();
        for word in line.split_whitespace() {
            if current.len() + word.len() + 1 > max_chars && !current.is_empty() {
                output.push(current);
                current = String::new();
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        output.push(current);
    }
    output
}

fn escape_pdf_text(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

fn escape_xml(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn format_seconds(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    format!(
        "{:02}:{:02}:{:02}",
        total / 3600,
        (total % 3600) / 60,
        total % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload() -> ExportPayload {
        ExportPayload {
            meeting_id: "meeting-1".to_string(),
            title: "Design Review".to_string(),
            created_at: "2026-06-18T10:00:00Z".to_string(),
            updated_at: "2026-06-18T10:30:00Z".to_string(),
            summary: Some(serde_json::json!({
                "action_items": {
                    "title": "Action Items",
                    "blocks": [{ "content": "Open a Linear follow-up" }]
                },
                "key_points": {
                    "title": "Key Points",
                    "blocks": [{ "content": "Ship exports locally" }]
                },
                "_section_order": ["key_points", "action_items"]
            })),
            transcripts: vec![ExportTranscript {
                text: "We should export the summary.".to_string(),
                timestamp: "10:01:00".to_string(),
                audio_start_time: Some(12.0),
            }],
        }
    }

    #[test]
    fn sanitizes_file_name() {
        assert_eq!(
            render_file_name("{title}/export", "A:B?", ExportFormat::Markdown),
            "A-B--export.md"
        );
    }

    #[test]
    fn renders_markdown_sections() {
        let markdown = render_markdown(&payload(), &ExportSections::default());
        assert!(markdown.contains("# Design Review"));
        assert!(markdown.contains("Open a Linear follow-up"));
        assert!(markdown.contains("[00:00:12]"));
    }

    #[test]
    fn renders_pdf_header() {
        let pdf = render_pdf(&payload(), &ExportSections::default());
        assert!(pdf.starts_with(b"%PDF-1.4"));
        assert!(pdf.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn renders_docx_archive() {
        let docx = render_docx(&payload(), &ExportSections::default()).unwrap();
        let cursor = Cursor::new(docx);
        let mut archive = zip::ZipArchive::new(cursor).unwrap();
        assert!(archive.by_name("word/document.xml").is_ok());
    }
}
