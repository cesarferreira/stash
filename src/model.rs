//! Clipboard entry models.

use chrono::{DateTime, Utc};
use std::fmt;
use std::path::PathBuf;

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
    Image,
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
            Self::Image => "image",
        }
    }

    pub fn glyph(self) -> &'static str {
        match self {
            Self::PlainText => "txt",
            Self::Url | Self::GitHubUrl => "url",
            Self::Json => "{}",
            Self::Jwt => "jwt",
            Self::Uuid => "id ",
            Self::GitSha => "sha",
            Self::FilePath => " / ",
            Self::ShellCommand => " $ ",
            Self::StackTrace => "err",
            Self::Image => "img",
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Some(match label {
            "text" => Self::PlainText,
            "url" => Self::Url,
            "json" => Self::Json,
            "jwt" => Self::Jwt,
            "uuid" => Self::Uuid,
            "sha" => Self::GitSha,
            "path" => Self::FilePath,
            "shell" => Self::ShellCommand,
            "trace" => Self::StackTrace,
            "github" => Self::GitHubUrl,
            "image" => Self::Image,
            _ => return None,
        })
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
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
    pub label: String,
    pub accent: u32,
    pub path: PathBuf,
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
    pub image: Option<ImageMeta>,
    /// Website thumbnail for URL entries (not pasted as an image).
    pub link_preview: Option<ImageMeta>,
    /// Searchable tags without `#` (e.g. `img`, `json`). An entry may have many.
    pub tags: Vec<String>,
}

impl ClipboardEntry {
    pub fn primary_type(&self) -> ContentType {
        self.detected_types
            .first()
            .copied()
            .unwrap_or(ContentType::PlainText)
    }

    pub fn is_image(&self) -> bool {
        self.detected_types.contains(&ContentType::Image)
    }

    pub fn preview_line(&self, max_chars: usize) -> String {
        if self.is_image() {
            if let Some(image) = &self.image {
                return format!("[{} · {}×{}]", image.label, image.width, image.height);
            }
        }
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

    pub fn age_ago_label(&self, now: DateTime<Utc>) -> String {
        format!("{} ago", self.age_label(now))
    }

    pub fn format_timestamp(&self, at: DateTime<Utc>) -> String {
        at.format("%d %b %Y at %H:%M").to_string()
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
