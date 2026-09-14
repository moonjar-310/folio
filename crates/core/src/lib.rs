//! Portable API contracts and validation. No Cloudflare or browser dependencies.
use serde::{Deserialize, Serialize};

pub const MAX_MARKDOWN_BYTES: usize = 128 * 1024;
// JSON escaping can expand one input byte to six bytes.
pub const MAX_REQUEST_BYTES: usize = MAX_MARKDOWN_BYTES * 6 + 1024;
pub const LIST_LIMIT: usize = 50;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Note {
    pub id: String,
    pub markdown: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub updated_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SaveNote {
    pub markdown: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}

pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub fn validate_note(id: &str, markdown: &str) -> Result<(), &'static str> {
    if !valid_id(id) {
        return Err("Invalid note ID");
    }
    if markdown.len() > MAX_MARKDOWN_BYTES {
        return Err("Note exceeds 128 KiB");
    }
    Ok(())
}

pub fn summarize(note: &Note, updated_at: u64) -> NoteSummary {
    let first = note.markdown.lines().find(|line| !line.trim().is_empty());
    let title = first
        .unwrap_or("Untitled note")
        .trim()
        .trim_start_matches('#')
        .trim();
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
    }
}

pub fn object_key(id: &str) -> Result<String, &'static str> {
    if !valid_id(id) {
        return Err("Invalid note ID");
    }
    Ok(format!("notes/{id}.md"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_paths_and_oversized_utf8() {
        for id in ["", "../secret", "a/b", "a\\b", "%2e%2e", ".", "a.md"] {
            assert!(!valid_id(id), "{id}");
            assert!(object_key(id).is_err());
        }
        assert!(validate_note("note-123", &"한".repeat(MAX_MARKDOWN_BYTES / 3 + 1)).is_err());
        assert!(validate_note("note-123", &"x".repeat(MAX_MARKDOWN_BYTES)).is_ok());
    }
    #[test]
    fn metadata_is_derived_without_changing_markdown() {
        let note = Note {
            id: "abc".into(),
            markdown: "# 생각\n\n- [ ] Keep Markdown".into(),
        };
        let meta = summarize(&note, 123);
        assert_eq!(meta.title, "생각");
        assert_eq!(meta.updated_at, 123);
        assert_eq!(object_key(&note.id).unwrap(), "notes/abc.md");
        assert!(note.markdown.contains("- [ ]"));
    }
}
