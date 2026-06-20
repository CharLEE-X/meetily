/// Embedded default templates using compile-time inclusion
///
/// These templates are bundled into the binary and serve as fallbacks
/// when custom templates are not available.

/// Daily standup template for engineering/product teams
pub const DAILY_STANDUP: &str = include_str!("../../../templates/daily_standup.json");

/// Client/sales call template
pub const SALES_MARKETING_CLIENT_CALL: &str =
    include_str!("../../../templates/sales_marketing_client_call.json");

/// Therapy/session notes template
pub const PSYCHIATRIC_SESSION: &str = include_str!("../../../templates/psychatric_session.json");

/// Agile retrospective template
pub const RETROSPECTIVE: &str = include_str!("../../../templates/retrospective.json");

/// Project sync/status update template
pub const PROJECT_SYNC: &str = include_str!("../../../templates/project_sync.json");

/// Standard meeting notes template
pub const STANDARD_MEETING: &str = include_str!("../../../templates/standard_meeting.json");

/// Registry of all built-in templates
///
/// Maps template identifiers to their embedded JSON content
pub fn get_builtin_templates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("daily_standup", DAILY_STANDUP),
        ("sales_marketing_client_call", SALES_MARKETING_CLIENT_CALL),
        ("psychiatric_session", PSYCHIATRIC_SESSION),
        ("retrospective", RETROSPECTIVE),
        ("project_sync", PROJECT_SYNC),
        ("standard_meeting", STANDARD_MEETING),
    ]
}

/// Get a built-in template by identifier
///
/// # Arguments
/// * `id` - Template identifier (e.g., "daily_standup", "standard_meeting")
///
/// # Returns
/// The template JSON content if found, None otherwise
pub fn get_builtin_template(id: &str) -> Option<&'static str> {
    match id {
        "daily_standup" => Some(DAILY_STANDUP),
        "sales_marketing_client_call" => Some(SALES_MARKETING_CLIENT_CALL),
        "psychiatric_session" => Some(PSYCHIATRIC_SESSION),
        "retrospective" => Some(RETROSPECTIVE),
        "project_sync" => Some(PROJECT_SYNC),
        "standard_meeting" => Some(STANDARD_MEETING),
        _ => None,
    }
}

/// List all built-in template identifiers
pub fn list_builtin_template_ids() -> Vec<&'static str> {
    vec![
        "daily_standup",
        "sales_marketing_client_call",
        "psychiatric_session",
        "retrospective",
        "project_sync",
        "standard_meeting",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_templates_valid_json() {
        for (id, content) in get_builtin_templates() {
            let result = serde_json::from_str::<serde_json::Value>(content);
            assert!(
                result.is_ok(),
                "Built-in template '{}' contains invalid JSON: {:?}",
                id,
                result.err()
            );
        }
    }

    #[test]
    fn test_get_builtin_template() {
        assert!(get_builtin_template("daily_standup").is_some());
        assert!(get_builtin_template("standard_meeting").is_some());
        assert!(get_builtin_template("nonexistent").is_none());
    }

    #[test]
    fn test_builtin_templates_include_required_defaults() {
        let ids = list_builtin_template_ids();

        for expected in [
            "daily_standup",
            "sales_marketing_client_call",
            "psychiatric_session",
            "retrospective",
            "project_sync",
            "standard_meeting",
        ] {
            assert!(
                ids.contains(&expected),
                "missing required default template: {}",
                expected
            );
            assert!(get_builtin_template(expected).is_some());
        }
    }
}
