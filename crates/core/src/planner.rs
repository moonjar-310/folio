use serde::{Deserialize, Serialize};
pub fn default_folder() -> String {
    "Personal".into()
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub completed: bool,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub source_note_id: Option<String>,
    pub source_line: Option<usize>,
    pub created_at: u64,
    pub updated_at: u64,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Goal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub position: i64,
    pub status: String,
    pub created_at: u64,
    pub updated_at: u64,
}
pub fn valid_folder(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 240
        && path
            .split('/')
            .all(|p| crate::paths::valid_segment(p) && !p.starts_with('.'))
}
pub fn valid_date(date: &str) -> bool {
    let b = date.as_bytes();
    if b.len() != 10
        || b[4] != b'-'
        || b[7] != b'-'
        || !b
            .iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return false;
    }
    let y: u32 = date[..4].parse().unwrap_or(0);
    let m: usize = date[5..7].parse().unwrap_or(0);
    let d: u32 = date[8..].parse().unwrap_or(0);
    let days = [
        0,
        31,
        if y.is_multiple_of(400) || (y.is_multiple_of(4) && !y.is_multiple_of(100)) {
            29
        } else {
            28
        },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        31,
        30,
        31,
    ];
    y > 0 && m > 0 && m <= 12 && d > 0 && d <= days[m]
}
pub fn valid_time(time: &str) -> bool {
    let b = time.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 2 || c.is_ascii_digit())
        && time[..2].parse::<u8>().is_ok_and(|n| n < 24)
        && time[3..].parse::<u8>().is_ok_and(|n| n < 60)
}
pub fn checkboxes(markdown: &str) -> Vec<(usize, String, bool)> {
    let mut fence: Option<char> = None;
    markdown
        .lines()
        .enumerate()
        .filter_map(|(line, text)| {
            let text = text.trim_start();
            if text.starts_with("```") || text.starts_with("~~~") {
                let marker = text.chars().next().unwrap();
                if fence == Some(marker) {
                    fence = None;
                } else if fence.is_none() {
                    fence = Some(marker);
                }
                return None;
            }
            if fence.is_some() {
                return None;
            }
            let item = text
                .strip_prefix("- ")
                .or_else(|| text.strip_prefix("* "))
                .or_else(|| text.strip_prefix("+ "))?;
            let completed = if item.starts_with("[ ] ") {
                false
            } else if item.starts_with("[x] ") || item.starts_with("[X] ") {
                true
            } else {
                return None;
            };
            Some((line, item[4..].trim().to_string(), completed))
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_dates_paths_and_fences() {
        assert!(valid_date("2024-02-29"));
        assert!(!valid_date("2025-02-29"));
        assert!(!valid_date("2026-13-01"));
        assert!(!valid_folder("a/../b"));
        assert!(valid_folder("Projects/새 계획"));
        assert!(!valid_time("한ab"));
        assert_eq!(
            checkboxes("```md\n- [ ] example\n```\n- [x] real"),
            vec![(3, "real".into(), true)]
        );
    }
}
