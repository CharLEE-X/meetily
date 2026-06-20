use super::types::Template;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTemplate {
    pub id: String,
    pub template: Template,
    pub source: TemplateSource,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TemplateSource {
    BuiltIn,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateExportBundle {
    pub schema_version: u32,
    pub exported_at: String,
    pub templates: Vec<StoredTemplate>,
}

pub const TEMPLATE_EXPORT_SCHEMA_VERSION: u32 = 1;

pub fn custom_templates_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("summary-templates")
}

pub fn legacy_custom_templates_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("templates")
}

pub fn sanitize_template_id(input: &str) -> Result<String, String> {
    let id = input.trim().to_lowercase().replace([' ', '-'], "_");

    if id.is_empty() {
        return Err("Template ID cannot be empty".to_string());
    }

    if id.len() > 80 {
        return Err("Template ID must be 80 characters or fewer".to_string());
    }

    if !id
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    {
        return Err(
            "Template ID can only use lowercase letters, numbers, underscores, spaces, or hyphens"
                .to_string(),
        );
    }

    Ok(id)
}

pub fn load_custom_templates_from_dir(app_data_dir: &Path) -> Result<Vec<StoredTemplate>, String> {
    let mut templates: Vec<StoredTemplate> = Vec::new();

    for dir in [
        custom_templates_dir(app_data_dir),
        legacy_custom_templates_dir(app_data_dir),
    ] {
        if !dir.exists() {
            continue;
        }

        for entry in fs::read_dir(&dir)
            .map_err(|error| format!("Failed to read template directory: {}", error))?
        {
            let entry =
                entry.map_err(|error| format!("Failed to read template entry: {}", error))?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }

            let raw = fs::read_to_string(&path).map_err(|error| {
                format!(
                    "Failed to read template file '{}': {}",
                    path.display(),
                    error
                )
            })?;
            let id = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .ok_or_else(|| format!("Template file '{}' has no valid file name", path.display()))
                .and_then(sanitize_template_id)?;
            let mut stored = parse_stored_template(&raw, &id).map_err(|error| {
                format!(
                    "Failed to parse template file '{}': {}",
                    path.display(),
                    error
                )
            })?;
            stored.id = id.clone();
            stored.template.id = Some(id);
            stored.template.validate()?;
            templates.retain(|existing| existing.id != stored.id);
            templates.push(stored);
        }
    }

    templates.sort_by(|a, b| a.template.name.cmp(&b.template.name));
    Ok(templates)
}

fn parse_stored_template(raw: &str, fallback_id: &str) -> Result<StoredTemplate, String> {
    if let Ok(stored) = serde_json::from_str::<StoredTemplate>(raw) {
        return Ok(stored);
    }

    let template: Template = serde_json::from_str(raw)
        .map_err(|error| format!("Failed to parse template JSON: {}", error))?;
    let now = Utc::now().to_rfc3339();
    Ok(StoredTemplate {
        id: fallback_id.to_string(),
        template,
        source: TemplateSource::Custom,
        created_at: now.clone(),
        updated_at: now,
    })
}

pub fn next_available_template_id(app_data_dir: &Path, base_id: &str) -> Result<String, String> {
    let base_id = sanitize_template_id(base_id)?;
    let existing: std::collections::HashSet<String> = load_custom_templates_from_dir(app_data_dir)?
        .into_iter()
        .map(|template| template.id)
        .collect();

    if !existing.contains(&base_id) {
        return Ok(base_id);
    }

    for index in 2..=999 {
        let candidate = format!("{}_{}", base_id, index);
        if !existing.contains(&candidate) {
            return Ok(candidate);
        }
    }

    Err(format!(
        "Could not find an available template ID for '{}'",
        base_id
    ))
}

pub fn save_custom_template_to_dir(
    app_data_dir: &Path,
    id: &str,
    mut template: Template,
) -> Result<StoredTemplate, String> {
    let id = sanitize_template_id(id)?;
    template.id = Some(id.clone());
    template.validate()?;

    let dir = custom_templates_dir(app_data_dir);
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Failed to create template directory: {}", error))?;

    let path = dir.join(format!("{}.json", id));
    let now = Utc::now().to_rfc3339();
    let created_at = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<StoredTemplate>(&raw).ok())
        .map(|stored| stored.created_at)
        .unwrap_or_else(|| now.clone());

    let stored = StoredTemplate {
        id,
        template,
        source: TemplateSource::Custom,
        created_at,
        updated_at: now,
    };
    let raw = serde_json::to_string_pretty(&stored)
        .map_err(|error| format!("Failed to serialize template: {}", error))?;
    fs::write(&path, raw).map_err(|error| {
        format!(
            "Failed to save template file '{}': {}",
            path.display(),
            error
        )
    })?;

    Ok(stored)
}

pub fn delete_custom_template_from_dir(app_data_dir: &Path, id: &str) -> Result<(), String> {
    let id = sanitize_template_id(id)?;
    let path = custom_templates_dir(app_data_dir).join(format!("{}.json", id));
    if path.exists() {
        fs::remove_file(&path)
            .map_err(|error| format!("Failed to delete template '{}': {}", id, error))?;
    }
    Ok(())
}

pub fn export_custom_templates_from_dir(
    app_data_dir: &Path,
) -> Result<TemplateExportBundle, String> {
    Ok(TemplateExportBundle {
        schema_version: TEMPLATE_EXPORT_SCHEMA_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        templates: load_custom_templates_from_dir(app_data_dir)?,
    })
}

pub fn import_custom_templates_to_dir(
    app_data_dir: &Path,
    raw_bundle: &str,
) -> Result<Vec<StoredTemplate>, String> {
    let bundle: TemplateExportBundle = serde_json::from_str(raw_bundle)
        .map_err(|error| format!("Failed to parse template import JSON: {}", error))?;

    if bundle.schema_version != TEMPLATE_EXPORT_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported template export schema version '{}'. Supported version is '{}'",
            bundle.schema_version, TEMPLATE_EXPORT_SCHEMA_VERSION
        ));
    }

    let mut imported = Vec::new();
    for stored in bundle.templates {
        if stored.source != TemplateSource::Custom {
            continue;
        }
        imported.push(save_custom_template_to_dir(
            app_data_dir,
            &stored.id,
            stored.template,
        )?);
    }

    Ok(imported)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_template(name: &str) -> Template {
        Template {
            id: None,
            schema_version: super::super::types::TEMPLATE_SCHEMA_VERSION,
            name: name.to_string(),
            description: "A reusable meeting template".to_string(),
            variables: vec!["meeting_title".to_string(), "transcript".to_string()],
            custom_instructions: None,
            sections: vec![super::super::types::TemplateSection {
                title: "Summary".to_string(),
                instruction: "Summarize the meeting".to_string(),
                format: "paragraph".to_string(),
                item_format: None,
                example_item_format: None,
            }],
        }
    }

    #[test]
    fn sanitize_template_id_normalizes_safe_input() {
        assert_eq!(
            sanitize_template_id("Client Sales Call").unwrap(),
            "client_sales_call"
        );
        assert_eq!(
            sanitize_template_id("retro-template-2026").unwrap(),
            "retro_template_2026"
        );
    }

    #[test]
    fn sanitize_template_id_rejects_path_traversal() {
        assert!(sanitize_template_id("../secret").is_err());
        assert!(sanitize_template_id("template.json").is_err());
        assert!(sanitize_template_id("").is_err());
    }

    #[test]
    fn export_bundle_round_trips_custom_template() {
        let stored = StoredTemplate {
            id: "project_sync_custom".to_string(),
            template: test_template("Project Sync Custom"),
            source: TemplateSource::Custom,
            created_at: "2026-06-20T08:00:00Z".to_string(),
            updated_at: "2026-06-20T08:00:00Z".to_string(),
        };
        let bundle = TemplateExportBundle {
            schema_version: TEMPLATE_EXPORT_SCHEMA_VERSION,
            exported_at: "2026-06-20T08:01:00Z".to_string(),
            templates: vec![stored],
        };

        let raw = serde_json::to_string_pretty(&bundle).unwrap();
        let parsed: TemplateExportBundle = serde_json::from_str(&raw).unwrap();

        assert_eq!(parsed.schema_version, TEMPLATE_EXPORT_SCHEMA_VERSION);
        assert_eq!(parsed.templates[0].id, "project_sync_custom");
        assert_eq!(parsed.templates[0].template.name, "Project Sync Custom");
    }

    #[test]
    fn custom_templates_dir_is_app_data_scoped() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            custom_templates_dir(dir.path()),
            dir.path().join("summary-templates")
        );
        assert_eq!(
            legacy_custom_templates_dir(dir.path()),
            dir.path().join("templates")
        );
    }

    #[test]
    fn saves_lists_and_deletes_custom_template() {
        let dir = tempfile::tempdir().unwrap();

        save_custom_template_to_dir(
            dir.path(),
            "Project Sync Custom",
            test_template("Project Sync Custom"),
        )
        .unwrap();

        let templates = load_custom_templates_from_dir(dir.path()).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, "project_sync_custom");
        assert_eq!(
            templates[0].template.id.as_deref(),
            Some("project_sync_custom")
        );

        delete_custom_template_from_dir(dir.path(), "project_sync_custom").unwrap();
        assert!(load_custom_templates_from_dir(dir.path())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn imports_only_custom_templates_from_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = TemplateExportBundle {
            schema_version: TEMPLATE_EXPORT_SCHEMA_VERSION,
            exported_at: "2026-06-20T08:01:00Z".to_string(),
            templates: vec![
                StoredTemplate {
                    id: "custom_one".to_string(),
                    template: test_template("Custom One"),
                    source: TemplateSource::Custom,
                    created_at: "2026-06-20T08:00:00Z".to_string(),
                    updated_at: "2026-06-20T08:00:00Z".to_string(),
                },
                StoredTemplate {
                    id: "built_in".to_string(),
                    template: test_template("Built In"),
                    source: TemplateSource::BuiltIn,
                    created_at: "2026-06-20T08:00:00Z".to_string(),
                    updated_at: "2026-06-20T08:00:00Z".to_string(),
                },
            ],
        };
        let raw = serde_json::to_string(&bundle).unwrap();

        let imported = import_custom_templates_to_dir(dir.path(), &raw).unwrap();
        let stored = load_custom_templates_from_dir(dir.path()).unwrap();

        assert_eq!(imported.len(), 1);
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, "custom_one");
    }

    #[test]
    fn loads_legacy_loose_template_files() {
        let dir = tempfile::tempdir().unwrap();
        let legacy_dir = legacy_custom_templates_dir(dir.path());
        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(
            legacy_dir.join("legacy_template.json"),
            serde_json::to_string(&test_template("Legacy Template")).unwrap(),
        )
        .unwrap();

        let stored = load_custom_templates_from_dir(dir.path()).unwrap();

        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].id, "legacy_template");
        assert_eq!(stored[0].source, TemplateSource::Custom);
    }

    #[test]
    fn next_available_template_id_does_not_overwrite_existing_copy() {
        let dir = tempfile::tempdir().unwrap();
        save_custom_template_to_dir(dir.path(), "daily_standup_copy", test_template("Copy"))
            .unwrap();

        assert_eq!(
            next_available_template_id(dir.path(), "daily_standup_copy").unwrap(),
            "daily_standup_copy_2"
        );
    }
}
