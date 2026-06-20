# Meeting Summary Templates

This directory contains bundled template definitions for meeting summary generation.
Users normally manage editable templates from **Settings → Templates**; these
bundled files are the built-in defaults and should remain safe fallbacks.

## Available Templates

### 1. `daily_standup.json`
Time-boxed daily updates template designed for engineering/product teams.

**Sections:**
- Date
- Attendees
- Yesterday (completed work)
- Today (planned work)
- Blockers
- Notes

### 2. `standard_meeting.json`
General-purpose meeting notes template focusing on key outcomes and actions.

**Sections:**
- Summary
- Key Decisions
- Action Items
- Discussion Highlights

## Template Structure

Each template JSON file follows this schema:

```json
{
  "id": "optional_stable_id",
  "schema_version": 1,
  "name": "Template Name",
  "description": "Brief description of the template's purpose",
  "variables": ["meeting_title", "transcript", "custom_instructions"],
  "custom_instructions": "Optional template-level guidance",
  "sections": [
    {
      "title": "Section Title",
      "instruction": "Instructions for the LLM on what to extract/include",
      "format": "paragraph|list|string",
      "item_format": "Optional: Markdown table format for list items"
    }
  ]
}
```

## Custom Templates

Users can add custom templates to the application data directory:

- **macOS**: `~/Library/Application Support/Meetily/summary-templates/`
- **Windows**: `%APPDATA%\Meetily\summary-templates\`
- **Linux**: `~/.local/share/Meetily/summary-templates/`

Custom templates override built-in templates with the same ID. Legacy loose JSON
files under the older `templates/` app-data directory are still read for
migration compatibility, but new saves and imports go to `summary-templates/`.

## Template Fields

### Root Level
- `name` (required): Display name for the template
- `schema_version` (required for new templates): Current supported version is `1`
- `description` (required): Brief explanation of the template's use case
- `variables` (optional): Allowed variables exposed in the editor
- `custom_instructions` (optional): Template-level model guidance
- `sections` (required): Array of section definitions

### Section Object
- `title` (required): Section heading text
- `instruction` (required): LLM guidance for this section
- `format` (required): One of `"paragraph"`, `"list"`, or `"string"`
- `item_format` (optional): Markdown formatting hint for list items (e.g., table structure)
- `example_item_format` (optional): Alternative formatting hint

## Usage in Code

Templates are loaded using the `templates` module:

```rust
use crate::summary::templates;

// Get a specific template
let template = templates::get_template("daily_standup")?;

// List available templates
let available = templates::list_templates();

// Validate custom template JSON
let custom_json = std::fs::read_to_string("custom.json")?;
let validated = templates::validate_and_parse_template(&custom_json)?;
```
