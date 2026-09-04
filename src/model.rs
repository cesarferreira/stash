//! Clipboard entry models for the interactive prototype.

use chrono::{DateTime, Utc};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContentType {
    PlainText,
    Url,
    Json,
    Jwt,
    Uuid,
    GitSha,
    FilePath,
    ShellCommand,
    StackTrace,
    GitHubUrl,
}

impl ContentType {
    pub fn label(self) -> &'static str {
        match self {
            Self::PlainText => "text",
            Self::Url => "url",
            Self::Json => "json",
            Self::Jwt => "jwt",
            Self::Uuid => "uuid",
            Self::GitSha => "sha",
            Self::FilePath => "path",
            Self::ShellCommand => "shell",
            Self::StackTrace => "trace",
            Self::GitHubUrl => "github",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Self::PlainText => "Aa",
            Self::Url | Self::GitHubUrl => "🔗",
            Self::Json => "{}",
            Self::Jwt => "JWT",
            Self::Uuid => "ID",
            Self::GitSha => "#",
            Self::FilePath => "/",
            Self::ShellCommand => "$",
            Self::StackTrace => "📝",
        }
    }
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SourceContext {
    pub app_name: String,
    pub bundle_id: Option<String>,
    pub cwd: Option<String>,
    pub git_repo: Option<String>,
    pub git_branch: Option<String>,
    pub hostname: Option<String>,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ClipboardEntry {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub last_copied_at: DateTime<Utc>,
    pub content: String,
    pub detected_types: Vec<ContentType>,
    pub source: SourceContext,
    pub copy_count: u32,
    pub pinned: bool,
}

impl ClipboardEntry {
    pub fn primary_type(&self) -> ContentType {
        self.detected_types
            .first()
            .copied()
            .unwrap_or(ContentType::PlainText)
    }

    pub fn preview_line(&self, max_chars: usize) -> String {
        let flat: String = self
            .content
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        let flat = flat.trim();
        if flat.chars().count() <= max_chars {
            flat.to_string()
        } else {
            let truncated: String = flat.chars().take(max_chars.saturating_sub(1)).collect();
            format!("{truncated}…")
        }
    }

    pub fn context_line(&self) -> Option<String> {
        let mut parts = Vec::new();
        parts.push(self.source.app_name.clone());
        if let Some(repo) = &self.source.git_repo {
            parts.push(repo.clone());
        }
        if let Some(branch) = &self.source.git_branch {
            parts.push(branch.clone());
        }
        if parts.len() > 1 {
            Some(parts.join(" · "))
        } else {
            None
        }
    }

    pub fn age_label(&self, now: DateTime<Utc>) -> String {
        let secs = (now - self.last_copied_at).num_seconds().max(0);
        if secs < 60 {
            format!("{secs}s")
        } else if secs < 3600 {
            format!("{}m", secs / 60)
        } else if secs < 86400 {
            format!("{}h", secs / 3600)
        } else {
            format!("{}d", secs / 86400)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Search,
    Actions,
    Edit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrototypeAction {
    Paste,
    Copy,
    PrettyJson,
    MinifyJson,
    OpenUrl,
    RemoveTracking,
    DecodeJwt,
    EditBeforePaste,
    Pin,
    Delete,
}

impl PrototypeAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::Paste => "Paste",
            Self::Copy => "Copy",
            Self::PrettyJson => "Pretty print JSON",
            Self::MinifyJson => "Minify JSON",
            Self::OpenUrl => "Open URL",
            Self::RemoveTracking => "Remove tracking parameters",
            Self::DecodeJwt => "Decode JWT",
            Self::EditBeforePaste => "Edit before paste",
            Self::Pin => "Pin / Unpin",
            Self::Delete => "Delete",
        }
    }

    pub fn supports(self, entry: &ClipboardEntry) -> bool {
        let types = &entry.detected_types;
        match self {
            Self::Paste | Self::Copy | Self::EditBeforePaste | Self::Pin | Self::Delete => true,
            Self::PrettyJson | Self::MinifyJson => types.contains(&ContentType::Json),
            Self::OpenUrl | Self::RemoveTracking => {
                types.contains(&ContentType::Url) || types.contains(&ContentType::GitHubUrl)
            }
            Self::DecodeJwt => types.contains(&ContentType::Jwt),
        }
    }
}

pub fn actions_for(entry: &ClipboardEntry) -> Vec<PrototypeAction> {
    [
        PrototypeAction::Paste,
        PrototypeAction::Copy,
        PrototypeAction::PrettyJson,
        PrototypeAction::MinifyJson,
        PrototypeAction::OpenUrl,
        PrototypeAction::RemoveTracking,
        PrototypeAction::DecodeJwt,
        PrototypeAction::EditBeforePaste,
        PrototypeAction::Pin,
        PrototypeAction::Delete,
    ]
    .into_iter()
    .filter(|action| action.supports(entry))
    .collect()
}
