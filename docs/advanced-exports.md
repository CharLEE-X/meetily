# Advanced Exports

Meetily can export a completed meeting from the meeting details screen as Markdown, PDF, or DOCX.

Exports are local-first:

* Manual exports are triggered from the meeting summary toolbar.
* The default destination is the app-managed `exports/` folder under Meetily app data.
* A custom destination folder can be entered in export preferences.
* Export history stores file paths, format, timestamp, size, and whether the export was automatic. It does not store exported meeting content.

## Export Options

Each export can include or exclude:

* Meeting metadata.
* Summary.
* Action items.
* Transcript with recording-relative timestamps when available.

The file name template supports:

* `{title}` for the meeting title.
* `{date}` for the export date.
* `{format}` for the export extension.

## Auto-Export

Auto-export is disabled by default. When enabled, Meetily exports once after the meeting summary reaches the completed state in the meeting details view. Auto-export uses the saved format, section selection, destination, and file name template.

## Packaging Notes

Markdown and PDF rendering are implemented in the Tauri app without extra native binaries. DOCX rendering uses the existing Rust ZIP dependency to create a minimal Office Open XML document. No Python, Docker, FastAPI, or external document service is required.
