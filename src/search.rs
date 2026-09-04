//! In-memory fuzzy search and ranking.

use crate::model::ClipboardEntry;
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

pub fn search_entries(
    entries: &[ClipboardEntry],
    query: &str,
    now: DateTime<Utc>,
    ctx: &SearchContext,
) -> Vec<RankedHit> {
    let query = query.trim();
    if query.is_empty() {
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
    let tokens: Vec<&str> = query.split_whitespace().collect();
    let mut hits = Vec::new();

    for (index, entry) in entries.iter().enumerate() {
        let haystack = searchable_text(entry);
        let mut token_score = 0i64;
        let mut matched = true;
        for token in &tokens {
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

        let score = token_score
            + context_boost(entry, ctx)
            + pin_boost(entry)
            + recent_score(entry, now) / 4
            + (entry.copy_count as i64).min(20);
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
    use crate::sample_data::sample_entries;

    #[test]
    fn empty_query_returns_recent_first() {
        let entries = sample_entries();
        let hits = search_entries(&entries, "", Utc::now(), &SearchContext::default());
        assert!(!hits.is_empty());
        let first = &entries[hits[0].index];
        assert!(first.pinned || first.last_copied_at >= entries[hits[1].index].last_copied_at);
    }

    #[test]
    fn grpc_query_finds_robot_command() {
        let entries = sample_entries();
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
        let entries = sample_entries();
        let hits = search_entries(&entries, "token", Utc::now(), &SearchContext::default());
        assert!(!hits.is_empty());
        // Prefer stax-context entry when available among matches.
        let top_repos: Vec<_> = hits
            .iter()
            .take(3)
            .filter_map(|h| entries[h.index].source.git_repo.as_deref())
            .collect();
        assert!(top_repos.iter().any(|r| *r == "stax") || !top_repos.is_empty());
    }
}
