//! Attachment helpers shared by the GUI, CLI and MCP server.
//!
//! An attachment is either a path on disk or an `http(s)` web link. The two
//! live in the same list so a task or project carries one collection, and the
//! helpers here keep the "is this a link?" rule in one place instead of each
//! front end deciding for itself.

/// Whether an attachment is a web link rather than a file path.
pub fn is_web_link(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower
        .strip_prefix("http://")
        .or_else(|| lower.strip_prefix("https://"))
        .is_some_and(|rest| !rest.trim().is_empty())
}

/// The URL part of an attachment. A named link stores `url<TAB>name`.
pub fn attachment_url(value: &str) -> &str {
    value.split_once('\t').map(|(url, _)| url).unwrap_or(value)
}

/// Store a link with an optional display name. An empty name keeps the link
/// exactly as it was before naming existed.
pub fn link_with_name(url: &str, name: &str) -> String {
    let name = name.trim();
    if name.is_empty() {
        url.trim().to_string()
    } else {
        format!("{}\t{name}", url.trim())
    }
}

/// The short label drawn in an attachment row. File paths show their file
/// name; links use their optional name, or drop the scheme and `www.` so a
/// long address still starts with something recognisable.
pub fn attachment_name(value: &str) -> String {
    if let Some((_, name)) = value.split_once('\t') {
        let name = name.trim();
        if !name.is_empty() {
            return name.to_string();
        }
    }
    if is_web_link(value) {
        let trimmed = value.trim();
        let without_scheme = trimmed
            .split_once("://")
            .map(|(_, rest)| rest)
            .unwrap_or(trimmed);
        let without_www = without_scheme
            .strip_prefix("www.")
            .unwrap_or(without_scheme);
        let trimmed_slash = without_www.trim_end_matches('/');
        if trimmed_slash.is_empty() {
            without_scheme.to_string()
        } else {
            trimmed_slash.to_string()
        }
    } else {
        std::path::Path::new(value)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| value.to_string())
    }
}

/// Turn a typed address into a stored link. A bare host such as
/// `example.com/page` gets `https://`; anything that still does not look like
/// an http(s) link is rejected so a stray word never becomes a broken row.
pub fn normalize_web_link(value: &str) -> Option<String> {
    let (value, name) = match value.split_once('\t') {
        Some((url, name)) => (url.trim(), Some(name)),
        None => (value.trim(), None),
    };
    if value.is_empty() {
        return None;
    }
    let candidate = if is_web_link(value) {
        value.to_string()
    } else if !value.contains(char::is_whitespace) && value.contains('.') {
        format!("https://{value}")
    } else {
        return None;
    };
    // A scheme with nothing after it is not a link.
    is_web_link(&candidate).then(|| link_with_name(&candidate, name.unwrap_or("")))
}
