use serde::{Deserialize, Serialize};

pub const TEMPLATE_SCHEMA_VERSION: u32 = 1;

pub const ALLOWED_TEMPLATE_VARIABLES: &[&str] = &[
    "meeting_title",
    "transcript",
    "participants",
    "date",
    "action_items",
    "custom_instructions",
];

fn default_schema_version() -> u32 {
    TEMPLATE_SCHEMA_VERSION
}

/// Represents a single section in a meeting template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateSection {
    /// Section title (e.g., "Summary", "Action Items")
    pub title: String,

    /// Instruction for the LLM on what to extract/include
    pub instruction: String,

    /// Format type: "paragraph", "list", or "string"
    pub format: String,

    /// Optional markdown formatting hint for list items (e.g., table structure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_format: Option<String>,

    /// Alternative formatting hint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example_item_format: Option<String>,
}

/// Represents a complete meeting template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Optional stable template identifier for user-managed templates
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,

    /// Schema version for migration-safe imports/exports
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,

    /// Template display name
    pub name: String,

    /// Brief description of the template's purpose
    pub description: String,

    /// Allowed variables referenced by the template editor/generation UI
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<String>,

    /// Optional high-level guidance applied before section-specific instructions
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_instructions: Option<String>,

    /// List of sections in the template
    pub sections: Vec<TemplateSection>,
}

impl Template {
    /// Validates the template structure
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != TEMPLATE_SCHEMA_VERSION {
            return Err(format!(
                "Unsupported template schema version '{}'. Supported version is '{}'",
                self.schema_version, TEMPLATE_SCHEMA_VERSION
            ));
        }

        if let Some(id) = &self.id {
            if id.trim().is_empty() {
                return Err("Template ID cannot be empty when provided".to_string());
            }
        }

        if self.name.is_empty() {
            return Err("Template name cannot be empty".to_string());
        }

        if self.description.is_empty() {
            return Err("Template description cannot be empty".to_string());
        }

        if self.sections.is_empty() {
            return Err("Template must have at least one section".to_string());
        }

        for variable in &self.variables {
            if !ALLOWED_TEMPLATE_VARIABLES.contains(&variable.as_str()) {
                return Err(format!(
                    "Template variable '{}' is not supported. Allowed variables: {}",
                    variable,
                    ALLOWED_TEMPLATE_VARIABLES.join(", ")
                ));
            }
        }

        for (i, section) in self.sections.iter().enumerate() {
            if section.title.is_empty() {
                return Err(format!("Section {} has empty title", i));
            }

            if section.instruction.is_empty() {
                return Err(format!("Section '{}' has empty instruction", section.title));
            }

            match section.format.as_str() {
                "paragraph" | "list" | "string" => {},
                other => return Err(format!(
                    "Section '{}' has invalid format '{}'. Must be 'paragraph', 'list', or 'string'",
                    section.title, other
                )),
            }
        }

        Ok(())
    }

    /// Generates a clean markdown template structure
    pub fn to_markdown_structure(&self) -> String {
        let mut markdown = String::from("# <Add Title here>\n\n");

        for section in &self.sections {
            markdown.push_str(&format!("**{}**\n\n", section.title));
        }

        markdown
    }

    /// Generates section-specific instructions for the LLM
    pub fn to_section_instructions(&self) -> String {
        let mut instructions = String::from(
            "- **For the main title (`# [AI-Generated Title]`):** Analyze the entire transcript and create a concise, descriptive title for the meeting.\n"
        );

        if let Some(custom_instructions) = self.custom_instructions.as_ref() {
            if !custom_instructions.trim().is_empty() {
                instructions.push_str(&format!(
                    "- **Template-level guidance:** {}.\n",
                    custom_instructions.trim()
                ));
            }
        }

        for section in &self.sections {
            instructions.push_str(&format!(
                "- **For the '{}' section:** {}.\n",
                section.title, section.instruction
            ));

            // Add item format instructions if present
            let item_format = section
                .item_format
                .as_ref()
                .or(section.example_item_format.as_ref());

            if let Some(format) = item_format {
                instructions.push_str(&format!(
                    "  - Items in this section should follow the format: `{}`.\n",
                    format
                ));
            }
        }

        instructions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_template() {
        let template = Template {
            id: None,
            schema_version: TEMPLATE_SCHEMA_VERSION,
            name: "Test Template".to_string(),
            description: "A test template".to_string(),
            variables: vec!["meeting_title".to_string(), "transcript".to_string()],
            custom_instructions: None,
            sections: vec![TemplateSection {
                title: "Summary".to_string(),
                instruction: "Provide a summary".to_string(),
                format: "paragraph".to_string(),
                item_format: None,
                example_item_format: None,
            }],
        };

        assert!(template.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_name() {
        let template = Template {
            id: None,
            schema_version: TEMPLATE_SCHEMA_VERSION,
            name: "".to_string(),
            description: "A test template".to_string(),
            variables: vec![],
            custom_instructions: None,
            sections: vec![],
        };

        assert!(template.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_format() {
        let template = Template {
            id: None,
            schema_version: TEMPLATE_SCHEMA_VERSION,
            name: "Test".to_string(),
            description: "Test".to_string(),
            variables: vec![],
            custom_instructions: None,
            sections: vec![TemplateSection {
                title: "Test".to_string(),
                instruction: "Test".to_string(),
                format: "invalid".to_string(),
                item_format: None,
                example_item_format: None,
            }],
        };

        assert!(template.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_variable() {
        let template = Template {
            id: Some("test".to_string()),
            schema_version: TEMPLATE_SCHEMA_VERSION,
            name: "Test".to_string(),
            description: "Test".to_string(),
            variables: vec!["system_prompt".to_string()],
            custom_instructions: None,
            sections: vec![TemplateSection {
                title: "Summary".to_string(),
                instruction: "Summarize".to_string(),
                format: "paragraph".to_string(),
                item_format: None,
                example_item_format: None,
            }],
        };

        assert!(template.validate().is_err());
    }

    #[test]
    fn test_legacy_template_json_defaults_schema_fields() {
        let raw = r#"{
            "name": "Legacy",
            "description": "Existing bundled template",
            "sections": [
                {
                    "title": "Summary",
                    "instruction": "Summarize",
                    "format": "paragraph"
                }
            ]
        }"#;

        let template: Template = serde_json::from_str(raw).unwrap();

        assert_eq!(template.schema_version, TEMPLATE_SCHEMA_VERSION);
        assert_eq!(template.id, None);
        assert!(template.variables.is_empty());
        assert!(template.validate().is_ok());
    }
}
