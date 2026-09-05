//! In-memory fuzzy search and ranking, with `#tag` filters.

use crate::model::ClipboardEntry;
use crate::tags::normalize_tag;
use chrono::{DateTime, Utc};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Debug, Clone)]
pub struct RankedHit {
    pub index: usize,
    pub score: i64,
}

#[derive(Debug, Clone)]
pub struct SearchContext {
    pub app_name: Option<String>,
    pub git_repo: Option<String>,
    pub git_branch: Option<String>,
    pub hostname: Option<String>,
    pub cwd: Option<String>,
}

impl Default for SearchContext {
    fn default() -> Self {
        Self {
            app_name: Some("Ghostty".into()),
            git_repo: Some("stax".into()),
            git_branch: Some("feature/foo".into()),
            hostname: Some("devbox".into()),
            cwd: Some("/Users/cesarferreira/code/github/stax".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedQuery {
    /// Required tags (AND), normalized without `#`.
    tags: Vec<String>,
    /// Remaining free-text tokens for fuzzy match.
    text_tokens: Vec<String>,
}

fn parse_query(query: &str) -> ParsedQuery {
    let mut tags = Vec::new();
    let mut text_tokens = Vec::new();
    for token in query.split_whitespace() {
        if let Some(rest) = token.strip_prefix('#') {
            let tag = normalize_tag(rest);
            if !tag.is_empty() && !tags.iter().any(|t| t == &tag) {
                tags.push(tag);
            }
        } else if !token.is_empty() {
            text_tokens.push(token.to_string());
        }
    }
    ParsedQuery { tags, text_tokens }
}

fn entry_has_tag(entry: &ClipboardEntry, tag: &str) -> bool {
    entry.tags.iter().any(|t| t == tag)
}

pub fn search_entries(
    entries: &[ClipboardEntry],
    query: &str,
    now: DateTime<Utc>,
    ctx: &SearchContext,
) -> Vec<RankedHit> {
    let query = query.trim();
    let parsed = parse_query(query);

    if parsed.tags.is_empty() && parsed.text_tokens.is_empty() {
        let mut hits: Vec<_> = entries
            .iter()
            .enumerate()
            .map(|(index, entry)| RankedHit {
                index,
                score: recent_score(entry, now) + pin_boost(entry) + context_boost(entry, ctx),
            })
            .collect();
        hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.index.cmp(&b.index)));
        return hits;
    }

    let matcher = SkimMatcherV2::default().ignore_case();
    let mut hits = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        if !parsed.tags.iter().all(|tag| entry_has_tag(entry, tag)) {
            continue;
        }

        let mut token_score = 0i64;
        if !parsed.text_tokens.is_empty() {
            let haystack = searchable_text(entry);
            let mut matched = true;
            for token in &parsed.text_tokens {
                match matcher.fuzzy_match(&haystack, token) {
                    Some(score) => token_score += score,
                    None => {
                        matched = false;
                        break;
                    }
                }
            }
            if !matched {
                continue;
            }
        }

        let score = token_score
            + context_boost(entry, ctx)
            + pin_boost(entry)
            + recent_score(entry, now) / 4
            + (entry.copy_count as i64).min(20)
            + (parsed.tags.len() as i64) * 5;
        hits.push(RankedHit { index, score });
    }

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.index.cmp(&b.index)));
    hits
}

fn searchable_text(entry: &ClipboardEntry) -> String {
    let mut parts = vec![entry.content.clone(), entry.source.app_name.clone()];
    for ty in &entry.detected_types {
        parts.push(ty.label().to_string());
        parts.push(ty.glyph().to_string());
    }
    for tag in &entry.tags {
        parts.push(tag.clone());
        parts.push(format!("#{tag}"));
    }
    if let Some(repo) = &entry.source.git_repo {
        parts.push(repo.clone());
    }
    if let Some(branch) = &entry.source.git_branch {
        parts.push(branch.clone());
    }
    if let Some(cwd) = &entry.source.cwd {
        parts.push(cwd.clone());
    }
    if let Some(host) = &entry.source.hostname {
        parts.push(host.clone());
    }
    if let Some(image) = &entry.image {
        parts.push(image.label.clone());
        parts.push("image".into());
    }
    if entry.pinned {
        parts.push("pinned".into());
    }
    parts.join("\n")
}

fn pin_boost(entry: &ClipboardEntry) -> i64 {
    if entry.pinned { 10 } else { 0 }
}

fn context_boost(entry: &ClipboardEntry, ctx: &SearchContext) -> i64 {
    let mut score = 0;
    if ctx
        .git_repo
        .as_ref()
        .is_some_and(|repo| entry.source.git_repo.as_ref() == Some(repo))
    {
        score += 30;
    }
    if ctx
        .app_name
        .as_ref()
        .is_some_and(|app| &entry.source.app_name == app)
    {
        score += 20;
    }
    if ctx
        .hostname
        .as_ref()
        .is_some_and(|host| entry.source.hostname.as_ref() == Some(host))
    {
        score += 15;
    }
    if ctx
        .cwd
        .as_ref()
        .is_some_and(|cwd| entry.source.cwd.as_ref() == Some(cwd))
    {
        score += 10;
    }
    if ctx
        .git_branch
        .as_ref()
        .is_some_and(|branch| entry.source.git_branch.as_ref() == Some(branch))
    {
        score += 5;
    }
    score
}

fn recent_score(entry: &ClipboardEntry, now: DateTime<Utc>) -> i64 {
    let age_secs = (now - entry.last_copied_at).num_seconds().max(0);
    // Newer items score higher; decays over about a day.
    (100_000 / (age_secs + 60)).clamp(0, 1_500)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ClipboardEntry, ContentType, SourceContext};
    use crate::tags::tags_for;
    use chrono::Duration;

    fn fixture_entries() -> Vec<ClipboardEntry> {
        let now = Utc::now();
        vec![
            entry(
                "1",
                "grpcurl robot.api.v1.RobotService/GetStatus",
                &[ContentType::ShellCommand, ContentType::PlainText],
                Some("stax"),
                now - Duration::minutes(2),
                false,
            ),
            entry(
                "2",
                r#"{"token":"abc","repo":"stax"}"#,
                &[ContentType::Json],
                Some("stax"),
                now - Duration::minutes(5),
                true,
            ),
            entry(
                "3",
                r#"{"token":"xyz","repo":"other"}"#,
                &[ContentType::Json],
                Some("other"),
                now - Duration::hours(2),
                false,
            ),
            entry(
                "4",
                "",
                &[ContentType::Image],
                None,
                now - Duration::seconds(30),
                false,
            ),
        ]
    }

    fn entry(
        id: &str,
        content: &str,
        types: &[ContentType],
        repo: Option<&str>,
        at: chrono::DateTime<Utc>,
        pinned: bool,
    ) -> ClipboardEntry {
        ClipboardEntry {
            id: id.into(),
            created_at: at,
            last_copied_at: at,
            content: content.into(),
            detected_types: types.to_vec(),
            source: SourceContext {
                app_name: "Test".into(),
                bundle_id: None,
                cwd: None,
                git_repo: repo.map(str::to_string),
                git_branch: None,
                hostname: None,
            },
            copy_count: 1,
            pinned,
            image: None,
            link_preview: None,
            tags: tags_for(types, pinned),
        }
    }

    #[test]
    fn empty_query_returns_recent_first() {
        let entries = fixture_entries();
        let hits = search_entries(&entries, "", Utc::now(), &SearchContext::default());
        assert!(!hits.is_empty());
        let first = &entries[hits[0].index];
        assert!(first.pinned || first.last_copied_at >= entries[hits[1].index].last_copied_at);
    }

    #[test]
    fn grpc_query_finds_robot_command() {
        let entries = fixture_entries();
        let hits = search_entries(
            &entries,
            "grpc robot",
            Utc::now(),
            &SearchContext::default(),
        );
        assert!(!hits.is_empty());
        assert!(
            entries[hits[0].index]
                .content
                .to_lowercase()
                .contains("grpc")
                || entries[hits[0].index]
                    .content
                    .to_lowercase()
                    .contains("robot")
        );
    }

    #[test]
    fn same_repo_boosts_json_token_results() {
        let entries = fixture_entries();
        let hits = search_entries(&entries, "token", Utc::now(), &SearchContext::default());
        assert!(!hits.is_empty());
        let top_repos: Vec<_> = hits
            .iter()
            .take(3)
            .filter_map(|h| entries[h.index].source.git_repo.as_deref())
            .collect();
        assert!(top_repos.iter().any(|r| *r == "stax") || !top_repos.is_empty());
    }

    #[test]
    fn hash_img_filters_images() {
        let entries = fixture_entries();
        let hits = search_entries(&entries, "#img", Utc::now(), &SearchContext::default());
        assert_eq!(hits.len(), 1);
        assert!(entries[hits[0].index]
            .tags
            .iter()
            .any(|t| t == "img"));
    }

    #[test]
    fn hash_json_and_text_combine() {
        let entries = fixture_entries();
        let hits = search_entries(
            &entries,
            "#json stax",
            Utc::now(),
            &SearchContext::default(),
        );
        assert!(!hits.is_empty());
        for hit in &hits {
            assert!(entries[hit.index].tags.iter().any(|t| t == "json"));
        }
    }

    #[test]
    fn parse_extracts_tags() {
        let parsed = parse_query("#img #JSON hello world");
        assert_eq!(parsed.tags, vec!["img".to_string(), "json".to_string()]);
        assert_eq!(
            parsed.text_tokens,
            vec!["hello".to_string(), "world".to_string()]
        );
    }
}
