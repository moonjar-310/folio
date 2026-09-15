//! Portable contracts, validation and Markdown task extraction.
use serde::{Deserialize, Serialize};
pub mod paths;
pub mod planner;
pub mod storage;
pub use planner::*;
pub const MAX_MARKDOWN_BYTES: usize = 128 * 1024;
pub const MAX_REQUEST_BYTES: usize = MAX_MARKDOWN_BYTES * 6 + 2048;
pub const LIST_LIMIT: usize = 50;
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub markdown: String,
    #[serde(default = "default_folder")]
    pub folder: String,
    #[serde(default)]
    pub revision: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub updated_at: u64,
    #[serde(default = "default_folder")]
    pub folder: String,
    #[serde(default)]
    pub revision: String,
    #[serde(default)]
    pub archived: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SaveNote {
    pub markdown: String,
    #[serde(default = "default_folder")]
    pub folder: String,
    #[serde(default)]
    pub revision: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
pub fn validate_note(id: &str, markdown: &str) -> Result<(), &'static str> {
    if !paths::valid_note_id(id) {
        return Err("Invalid note ID");
    }
    if markdown.len() > MAX_MARKDOWN_BYTES {
        return Err("Note exceeds 128 KiB");
    }
    Ok(())
}
pub fn summarize(note: &Note, updated_at: u64) -> NoteSummary {
    let first = note
        .markdown
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Untitled note");
    let title = first.trim().trim_start_matches('#').trim();
    NoteSummary {
        id: note.id.clone(),
        title: if title.is_empty() {
            "Untitled note".into()
        } else {
            title.chars().take(120).collect()
        },
        preview: note
            .markdown
            .lines()
            .skip(1)
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(160)
            .collect(),
        updated_at,
        folder: note.folder.clone(),
        revision: note.revision.clone(),
        archived: note.folder == "Archive" || note.folder.starts_with("Archive/"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_paths_and_oversized_utf8() {
        for id in ["", "../secret", "a/b", "a\\b", "%2e%2e", ".", "a.md"] {
            assert!(!valid_id(id));
        }
        assert!(validate_note("note-123", &"한".repeat(MAX_MARKDOWN_BYTES / 3 + 1)).is_err());
        assert!(validate_note("note-123", &"x".repeat(MAX_MARKDOWN_BYTES)).is_ok());
    }
}

pub fn rename_title(markdown: &str, title: &str) -> String {
    let lines: Vec<&str> = markdown.split_inclusive('\n').collect();
    if let Some(index) = lines.iter().position(|line| !line.trim().is_empty())
        && lines[index].trim_start().starts_with("# ")
    {
        let ending = if lines[index].ends_with("\r\n") {
            "\r\n"
        } else if lines[index].ends_with('\n') {
            "\n"
        } else {
            ""
        };
        return format!(
            "{}# {title}{ending}{}",
            lines[..index].concat(),
            lines[index + 1..].concat()
        );
    }
    format!("# {title}\n\n{markdown}")
}
#[cfg(test)]
mod title_tests {
    #[test]
    fn renaming_preserves_body_and_line_endings() {
        assert_eq!(
            super::rename_title("A paragraph\nKeep me", "Title"),
            "# Title\n\nA paragraph\nKeep me"
        );
        assert_eq!(
            super::rename_title("\r\n# Old\r\nBody\r\n", "New"),
            "\r\n# New\r\nBody\r\n"
        );
    }
}
