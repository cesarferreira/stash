//! Heuristic content-type detection for captured clipboard text.

use crate::model::ContentType;

fn looks_like_jwt(text: &str) -> bool {
    let parts: Vec<_> = text.trim().split('.').collect();
    parts.len() == 3
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
        && text.len() > 40
}

fn looks_like_uuid(text: &str) -> bool {
    let t = text.trim();
    if t.len() != 36 {
        return false;
    }
    let bytes = t.as_bytes();
    let hex = |i: usize| bytes[i].is_ascii_hexdigit();
    let dash = |i: usize| bytes[i] == b'-';
    (0..8).all(hex)
        && dash(8)
        && (9..13).all(hex)
        && dash(13)
        && (14..18).all(hex)
        && dash(18)
        && (19..23).all(hex)
        && dash(23)
        && (24..36).all(hex)
}

fn looks_like_git_sha(text: &str) -> bool {
    let t = text.trim();
    (t.len() == 7 || t.len() == 40) && t.chars().all(|c| c.is_ascii_hexdigit())
}

fn looks_like_url(text: &str) -> bool {
    let t = text.trim();
    t.starts_with("http://") || t.starts_with("https://")
}

fn looks_like_github_url(text: &str) -> bool {
    looks_like_url(text)
        && (text.contains("github.com/") || text.contains("githubusercontent.com/"))
}

fn looks_like_json(text: &str) -> bool {
    let t = text.trim();
    (t.starts_with('{') && t.ends_with('}')) || (t.starts_with('[') && t.ends_with(']'))
}

fn looks_like_file_path(text: &str) -> bool {
    let t = text.trim();
    if t.contains('\n') || t.contains(' ') {
        return false;
    }
    t.starts_with('/') || t.starts_with("~/") || t.starts_with("./")
}

fn looks_like_shell(text: &str) -> bool {
    let t = text.trim();
    if t.contains('\n') {
        return false;
    }
    const PREFIXES: &[&str] = &[
        "git ", "cargo ", "npm ", "pnpm ", "yarn ", "brew ", "docker ", "kubectl ", "ssh ",
        "curl ", "wget ", "cd ", "ls ", "rg ", "fd ", "make ", "python ", "node ",
    ];
    PREFIXES.iter().any(|p| t.starts_with(p))
}

fn looks_like_stack_trace(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("traceback (most recent call last)")
        || lower.contains("exception in thread")
        || (lower.contains("at ") && lower.contains(".java:"))
        || text.contains("panic!")
        || text.contains("stack backtrace:")
}

/// Ordered detectors — first match becomes the primary type.
pub fn detect_types(text: &str) -> Vec<ContentType> {
    let mut types = Vec::new();
    let push = |types: &mut Vec<ContentType>, ty: ContentType| {
        if !types.contains(&ty) {
            types.push(ty);
        }
    };

    if looks_like_jwt(text) {
        push(&mut types, ContentType::Jwt);
    }
    if looks_like_uuid(text) {
        push(&mut types, ContentType::Uuid);
    }
    if looks_like_git_sha(text) {
        push(&mut types, ContentType::GitSha);
    }
    if looks_like_github_url(text) {
        push(&mut types, ContentType::GitHubUrl);
        push(&mut types, ContentType::Url);
    } else if looks_like_url(text) {
        push(&mut types, ContentType::Url);
    }
    if looks_like_json(text) && serde_json::from_str::<serde_json::Value>(text.trim()).is_ok() {
        push(&mut types, ContentType::Json);
    }
    if looks_like_stack_trace(text) {
        push(&mut types, ContentType::StackTrace);
    }
    if looks_like_file_path(text) {
        push(&mut types, ContentType::FilePath);
    }
    if looks_like_shell(text) {
        push(&mut types, ContentType::ShellCommand);
    }
    if types.is_empty() {
        types.push(ContentType::PlainText);
    } else if let Some(
        ContentType::Jwt | ContentType::Uuid | ContentType::GitSha | ContentType::ShellCommand,
    ) = types.first().copied()
    {
        push(&mut types, ContentType::PlainText);
    }
    types
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_url_and_json() {
        assert!(detect_types("https://example.com").contains(&ContentType::Url));
        assert!(detect_types(r#"{"a":1}"#).contains(&ContentType::Json));
        assert!(
            detect_types("550e8400-e29b-41d4-a716-446655440000").contains(&ContentType::Uuid)
        );
    }
}
