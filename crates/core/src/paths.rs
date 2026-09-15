//! Portable vault paths, excluding traversal and Windows path aliases.
pub fn valid_segment(s: &str) -> bool {
    if s.is_empty()
        || s.len() > 180
        || s.trim() != s
        || s.ends_with('.')
        || s == "."
        || s == ".."
        || s.chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c))
    {
        return false;
    }
    let base = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    !matches!(
        base.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    )
}
pub fn valid_path(path: &str) -> bool {
    !path.is_empty() && path.len() <= 900 && path.split('/').all(valid_segment)
}
pub fn note_path(path: &str) -> bool {
    valid_path(path) && path.ends_with(".md") && !path.split('/').any(|s| s.starts_with('.'))
}
pub fn path_id(path: &str) -> String {
    format!(
        "p-{}",
        path.as_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    )
}
pub fn id_path(id: &str) -> Option<String> {
    let hex = id.strip_prefix("p-")?;
    if hex.len() > 1800 || !hex.len().is_multiple_of(2) {
        return None;
    }
    let bytes = hex
        .as_bytes()
        .chunks(2)
        .map(|c| {
            std::str::from_utf8(c)
                .ok()
                .and_then(|s| u8::from_str_radix(s, 16).ok())
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok().filter(|s| note_path(s))
}
pub fn valid_note_id(id: &str) -> bool {
    super::valid_id(id) || id_path(id).is_some()
}
pub fn folder(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(p, _)| p.to_string())
        .unwrap_or_default()
}
pub fn filename(title: &str) -> String {
    let clean: String = title
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .scan(0, |n, c| {
            *n += c.len_utf8();
            if *n <= 140 { Some(c) } else { None }
        })
        .collect();
    let clean = clean.trim().trim_end_matches('.').trim_start_matches('.');
    let clean = if valid_segment(clean) {
        clean
    } else {
        "Untitled note"
    };
    format!("{clean}.md")
}
