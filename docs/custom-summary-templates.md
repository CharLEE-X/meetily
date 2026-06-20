# Custom Summary Templates

Meetily supports local custom summary templates for users who want different
meeting outputs for standups, project reviews, retrospectives, customer calls,
or personal workflows.

## Where to manage templates

Open **Settings → Templates**.

From there you can:

- create a new custom template;
- duplicate a built-in template into an editable copy;
- edit template name, description, variables, guidance, sections, formats, and
  optional item formats;
- validate a template before saving;
- preview the markdown structure that the model will fill;
- import and export custom templates as JSON;
- delete custom templates;
- restore editable copies of the built-in defaults.

Built-in templates are protected. To change one, duplicate it first and edit the
custom copy.

## Per-meeting selection

Meeting details include a template picker in the summary toolbar. The selected
template is remembered per meeting and reused for later generation or
regeneration. When templates are imported, deleted, or restored, open meeting
views refresh their template lists automatically.

## Storage and migration behavior

Custom templates are stored locally in the Meetily app data directory:

- macOS: `~/Library/Application Support/Meetily/summary-templates/`
- Windows: `%APPDATA%\Meetily\summary-templates\`
- Linux: `~/.local/share/Meetily/summary-templates/`

The supported schema version is `1`. New saves and imports use the
`summary-templates/` directory and wrap templates with source metadata for safe
export/import. Legacy loose JSON templates from the older `templates/` directory
are still read so existing local custom templates keep working.

Generated summaries store the selected template ID, schema version, display
name, and template fingerprint in local summary metadata. This lets Meetily
detect when cached output came from a different template without storing extra
meeting content outside the existing summary record.

## Template JSON shape

```json
{
  "id": "engineering_sync",
  "schema_version": 1,
  "name": "Engineering Sync",
  "description": "Status, blockers, decisions, and follow-ups for engineering meetings.",
  "variables": ["meeting_title", "transcript", "participants", "custom_instructions"],
  "custom_instructions": "Prefer concrete owner/action/due-date language.",
  "sections": [
    {
      "title": "Summary",
      "instruction": "Summarize the main engineering context and outcome.",
      "format": "paragraph"
    },
    {
      "title": "Action Items",
      "instruction": "List follow-ups with owner and due date when available.",
      "format": "list",
      "item_format": "Owner | Task | Due | Evidence"
    }
  ]
}
```

Allowed variables are:

- `meeting_title`
- `transcript`
- `participants`
- `date`
- `action_items`
- `custom_instructions`

## QA matrix

| Flow | Expected result |
| --- | --- |
| Default template generation | A built-in template can be selected and used for summary generation. |
| Custom template save | Valid custom templates save locally and appear in the meeting template picker. |
| Invalid template validation | Empty names, empty sections, unsupported variables, and invalid formats fail before model invocation. |
| Import/export | Exported custom-template JSON imports back into the app without changing built-in templates. |
| Delete custom template | Deleted custom templates disappear from Settings and meeting pickers; built-ins remain available. |
| Per-meeting persistence | A meeting remembers its selected template across reloads. |
