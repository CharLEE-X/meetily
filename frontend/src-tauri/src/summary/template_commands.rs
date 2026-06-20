use crate::summary::templates;
use crate::summary::templates::storage::{self, StoredTemplate, TemplateExportBundle};
use serde::{Deserialize, Serialize};
use tauri::{Manager, Runtime};
use tracing::{info, warn};

/// Template metadata for UI display
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateInfo {
    /// Template identifier (e.g., "daily_standup", "standard_meeting")
    pub id: String,

    /// Display name for the template
    pub name: String,

    /// Brief description of the template's purpose
    pub description: String,

    /// Source of the template: builtIn or custom
    pub source: String,
}

/// Detailed template structure for preview/debugging
#[derive(Debug, Serialize, Deserialize)]
pub struct TemplateDetails {
    /// Template identifier
    pub id: String,

    /// Display name
    pub name: String,

    /// Description
    pub description: String,

    /// List of section titles in order
    pub sections: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateRecord {
    pub id: String,
    pub source: String,
    pub template: templates::Template,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveTemplateRequest {
    pub id: String,
    pub template: templates::Template,
}

/// Lists all available templates
///
/// Returns templates from both built-in (embedded) and custom (user data directory) sources.
/// Templates are automatically discovered - no code changes needed to add new templates.
///
/// # Returns
/// Vector of TemplateInfo with id, name, and description for each template
#[tauri::command]
pub async fn api_list_templates<R: Runtime>(
    _app: tauri::AppHandle<R>,
) -> Result<Vec<TemplateInfo>, String> {
    info!("api_list_templates called");

    let stored_templates = templates::list_stored_templates();

    let template_infos: Vec<TemplateInfo> = stored_templates
        .into_iter()
        .map(|stored| TemplateInfo {
            id: stored.id,
            name: stored.template.name,
            description: stored.template.description,
            source: match stored.source {
                storage::TemplateSource::BuiltIn => "builtIn".to_string(),
                storage::TemplateSource::Custom => "custom".to_string(),
            },
        })
        .collect();

    info!("Found {} available templates", template_infos.len());

    Ok(template_infos)
}

#[tauri::command]
pub async fn api_save_custom_template<R: Runtime>(
    app: tauri::AppHandle<R>,
    request: SaveTemplateRequest,
) -> Result<StoredTemplate, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {}", error))?;

    storage::save_custom_template_to_dir(&app_data_dir, &request.id, request.template)
}

#[tauri::command]
pub async fn api_delete_custom_template<R: Runtime>(
    app: tauri::AppHandle<R>,
    template_id: String,
) -> Result<(), String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {}", error))?;

    storage::delete_custom_template_from_dir(&app_data_dir, &template_id)
}

#[tauri::command]
pub async fn api_duplicate_template<R: Runtime>(
    app: tauri::AppHandle<R>,
    template_id: String,
    new_template_id: String,
    new_name: String,
) -> Result<StoredTemplate, String> {
    let mut template = templates::get_template(&template_id)?;
    template.name = new_name;
    template.id = Some(new_template_id.clone());

    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {}", error))?;

    storage::save_custom_template_to_dir(&app_data_dir, &new_template_id, template)
}

#[tauri::command]
pub async fn api_export_custom_templates<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<TemplateExportBundle, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {}", error))?;

    storage::export_custom_templates_from_dir(&app_data_dir)
}

#[tauri::command]
pub async fn api_import_custom_templates<R: Runtime>(
    app: tauri::AppHandle<R>,
    bundle_json: String,
) -> Result<Vec<StoredTemplate>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {}", error))?;

    storage::import_custom_templates_to_dir(&app_data_dir, &bundle_json)
}

#[tauri::command]
pub async fn api_restore_default_templates<R: Runtime>(
    app: tauri::AppHandle<R>,
) -> Result<Vec<StoredTemplate>, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Failed to resolve app data directory: {}", error))?;
    let mut restored = Vec::new();

    for id in [
        "daily_standup",
        "sales_marketing_client_call",
        "psychiatric_session",
        "retrospective",
        "project_sync",
        "standard_meeting",
    ] {
        let mut template = templates::get_template(id)?;
        let new_id = storage::next_available_template_id(&app_data_dir, &format!("{}_copy", id))?;
        template.id = Some(new_id.clone());
        restored.push(storage::save_custom_template_to_dir(
            &app_data_dir,
            &new_id,
            template,
        )?);
    }

    Ok(restored)
}

/// Gets detailed information about a specific template
///
/// # Arguments
/// * `template_id` - Template identifier (e.g., "daily_standup")
///
/// # Returns
/// TemplateDetails with full template structure
#[tauri::command]
pub async fn api_get_template_details<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
) -> Result<TemplateDetails, String> {
    info!(
        "api_get_template_details called for template_id: {}",
        template_id
    );

    let template = templates::get_template(&template_id)?;

    let section_titles: Vec<String> = template
        .sections
        .iter()
        .map(|section| section.title.clone())
        .collect();

    let details = TemplateDetails {
        id: template_id,
        name: template.name,
        description: template.description,
        sections: section_titles,
    };

    info!("Retrieved template details for '{}'", details.name);

    Ok(details)
}

#[tauri::command]
pub async fn api_get_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_id: String,
) -> Result<TemplateRecord, String> {
    let template = templates::get_template(&template_id)?;
    let source = templates::list_stored_templates()
        .into_iter()
        .find(|stored| stored.id == template_id)
        .map(|stored| match stored.source {
            storage::TemplateSource::BuiltIn => "builtIn".to_string(),
            storage::TemplateSource::Custom => "custom".to_string(),
        })
        .unwrap_or_else(|| "builtIn".to_string());

    Ok(TemplateRecord {
        id: template_id,
        source,
        template,
    })
}

/// Validates a custom template JSON string
///
/// Useful for template editor UI or validation before saving custom templates
///
/// # Arguments
/// * `template_json` - Raw JSON string of the template
///
/// # Returns
/// Ok(template_name) if valid, Err(error_message) if invalid
#[tauri::command]
pub async fn api_validate_template<R: Runtime>(
    _app: tauri::AppHandle<R>,
    template_json: String,
) -> Result<String, String> {
    info!("api_validate_template called");

    match templates::validate_and_parse_template(&template_json) {
        Ok(template) => {
            info!("Template '{}' validated successfully", template.name);
            Ok(template.name)
        }
        Err(e) => {
            warn!("Template validation failed: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_templates() {
        // This test requires the templates to be embedded/available
        // In a real test environment, you might want to mock the templates module

        // For now, just verify the function compiles and runs
        // You can expand this with more specific assertions
    }

    #[tokio::test]
    async fn test_validate_template_valid() {
        let valid_json = r#"
        {
            "schema_version": 1,
            "name": "Test Template",
            "description": "A test template",
            "sections": [
                {
                    "title": "Summary",
                    "instruction": "Provide a summary",
                    "format": "paragraph"
                }
            ]
        }"#;

        // Mock app handle would be needed for actual testing
        // For now, test the validation logic directly
        let result = templates::validate_and_parse_template(valid_json);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_validate_template_invalid() {
        let invalid_json = "invalid json";

        let result = templates::validate_and_parse_template(invalid_json);
        assert!(result.is_err());
    }
}
