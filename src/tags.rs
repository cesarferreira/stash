//! Auto-derived tags for clipboard entries (multi-tag; `#tag` search).

use crate::model::ContentType;

/// Normalize a tag token (strips leading `#`, lowercases).
pub fn normalize_tag(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('#')
        .to_ascii_lowercase()
}

/// Tags derived from detected types (+ pin). Includes short aliases so `#img` and `#image` both work.
pub fn tags_for(types: &[ContentType], pinned: bool) -> Vec<String> {
    let mut tags = Vec::new();
    for ty in types {
        for tag in ty.auto_tags() {
            push_unique(&mut tags, tag);
        }
    }
    if types.is_empty() {
        for tag in ContentType::PlainText.auto_tags() {
            push_unique(&mut tags, tag);
        }
    }
    if pinned {
        push_unique(&mut tags, "pin");
        push_unique(&mut tags, "pinned");
    }
    tags
}

fn push_unique(tags: &mut Vec<String>, tag: &str) {
    if !tags.iter().any(|t| t == tag) {
        tags.push(tag.to_string());
    }
}

impl ContentType {
    /// Searchable tag aliases for this type (without `#`).
    pub fn auto_tags(self) -> &'static [&'static str] {
        match self {
            Self::PlainText => &["text", "txt"],
            Self::Url => &["url", "link"],
            Self::Json => &["json"],
            Self::Jwt => &["jwt", "token"],
            Self::Uuid => &["uuid", "id"],
            Self::GitSha => &["sha", "git", "commit"],
            Self::FilePath => &["path", "file"],
            Self::ShellCommand => &["shell", "cmd", "bash", "cli"],
            Self::StackTrace => &["trace", "error", "err", "stack"],
            Self::GitHubUrl => &["github", "gh", "url", "link"],
            Self::Image => &["image", "img", "png", "pic", "photo"],
        }
    }
}

/// Format tags for display: `#img #image #png`
pub fn format_tags(tags: &[String]) -> String {
    tags.iter()
        .map(|t| format!("#{t}"))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_gets_img_and_image() {
        let tags = tags_for(&[ContentType::Image], false);
        assert!(tags.contains(&"img".into()));
        assert!(tags.contains(&"image".into()));
        assert!(tags.contains(&"png".into()));
    }

    #[test]
    fn pin_adds_pin_tags() {
        let tags = tags_for(&[ContentType::PlainText], true);
        assert!(tags.contains(&"pin".into()));
        assert!(tags.contains(&"txt".into()));
    }

    #[test]
    fn normalize_strips_hash() {
        assert_eq!(normalize_tag("#IMG"), "img");
        assert_eq!(normalize_tag("Json"), "json");
    }
}
