//! PRD-derived in-memory sample clipboard history.

use crate::model::{ClipboardEntry, ContentType, SourceContext};
use chrono::{Duration, Utc};

pub fn sample_entries() -> Vec<ClipboardEntry> {
    let now = Utc::now();

    vec![
        entry(
            "e01",
            now - Duration::seconds(12),
            "grpcurl localhost:50505/robot.v1.RobotService/GetStatus",
            &[ContentType::ShellCommand, ContentType::PlainText],
            ghostty("stax", "feature/grpc-robot"),
            4,
            false,
        ),
        entry(
            "e02",
            now - Duration::minutes(3),
            "UNAVAILABLE: io exception\n  at robot.grpc.Client.call(Client.kt:88)\n  at robot.ui.StatusFetcher.fetch(StatusFetcher.kt:41)",
            &[ContentType::StackTrace, ContentType::PlainText],
            android_studio("android-robot", "main"),
            1,
            false,
        ),
        entry(
            "e03",
            now - Duration::minutes(8),
            r#"{"grpc_port":50505,"robot":"alpha","enabled":true}"#,
            &[ContentType::Json],
            ghostty("stax", "feature/grpc-robot"),
            2,
            true,
        ),
        entry(
            "e04",
            now - Duration::minutes(23),
            "https://grpc.io/docs/languages/rust/quickstart/",
            &[ContentType::Url],
            arc(None, None),
            1,
            false,
        ),
        entry(
            "e05",
            now - Duration::hours(2),
            "adb reverse tcp:50505 tcp:50505",
            &[ContentType::ShellCommand, ContentType::PlainText],
            ghostty("android-robot", "main"),
            6,
            false,
        ),
        entry(
            "e06",
            now - Duration::minutes(2),
            r#"{"user_id":123,"enabled":true,"role":"admin"}"#,
            &[ContentType::Json],
            ghostty("stax", "feature/foo"),
            3,
            false,
        ),
        entry(
            "e07",
            now - Duration::minutes(9),
            r#"{"name":"foo","version":"1.2"}"#,
            &[ContentType::Json],
            chrome(),
            1,
            false,
        ),
        entry(
            "e08",
            now - Duration::hours(1),
            r#"{"device":"pixel"}"#,
            &[ContentType::Json],
            android_studio("android-robot", "pixel-debug"),
            2,
            false,
        ),
        entry(
            "e09",
            now - Duration::minutes(15),
            "https://github.com/foo/bar/commit/a3c87f9e2b1d4c6f8a0e5b7d9c1f3a5e7b9d1c3f",
            &[
                ContentType::GitHubUrl,
                ContentType::Url,
                ContentType::GitSha,
            ],
            arc(Some("stax"), Some("main")),
            1,
            false,
        ),
        entry(
            "e10",
            now - Duration::minutes(41),
            "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiZXhwIjoxNzE2MjM5MDIyLCJyb2xlIjoiYWRtaW4ifQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
            &[ContentType::Jwt, ContentType::PlainText],
            vscode("stax", "feature/auth"),
            2,
            true,
        ),
        entry(
            "e11",
            now - Duration::hours(3),
            "550e8400-e29b-41d4-a716-446655440000",
            &[ContentType::Uuid, ContentType::PlainText],
            ghostty("stax", "main"),
            1,
            false,
        ),
        entry(
            "e12",
            now - Duration::hours(4),
            "a3c87f9e2b1d4c6f8a0e5b7d9c1f3a5e7b9d1c3f",
            &[ContentType::GitSha, ContentType::PlainText],
            ghostty("stax", "feature/foo"),
            3,
            false,
        ),
        entry(
            "e13",
            now - Duration::hours(5),
            "/Users/cesarferreira/code/github/stax/crates/cli/src/main.rs",
            &[ContentType::FilePath, ContentType::PlainText],
            vscode("stax", "feature/foo"),
            2,
            false,
        ),
        entry(
            "e14",
            now - Duration::hours(6),
            "https://example.com/foo?id=123&utm_source=twitter&utm_campaign=launch",
            &[ContentType::Url],
            safari(),
            1,
            false,
        ),
        entry(
            "e15",
            now - Duration::hours(8),
            "cargo test -p clip-core -- detection",
            &[ContentType::ShellCommand, ContentType::PlainText],
            ghostty("stash", "main"),
            5,
            false,
        ),
        entry(
            "e16",
            now - Duration::days(1),
            "pixel adb devices\nList of devices attached\nemulator-5554\tdevice",
            &[ContentType::PlainText, ContentType::ShellCommand],
            ghostty("android-robot", "main"),
            1,
            false,
        ),
        entry(
            "e17",
            now - Duration::days(2),
            r#"{"token":"sk_live_example_do_not_use","exp":1716239022}"#,
            &[ContentType::Json],
            chrome(),
            1,
            false,
        ),
        entry(
            "e18",
            now - Duration::minutes(55),
            "IllegalStateException: Robot channel closed\n\tat com.wayve.robot.Channel.send(Channel.kt:22)",
            &[ContentType::StackTrace, ContentType::PlainText],
            android_studio("android-robot", "crash-repro"),
            2,
            false,
        ),
    ]
}

fn entry(
    id: &str,
    at: chrono::DateTime<Utc>,
    content: &str,
    types: &[ContentType],
    source: SourceContext,
    copy_count: u32,
    pinned: bool,
) -> ClipboardEntry {
    ClipboardEntry {
        id: id.to_string(),
        created_at: at,
        last_copied_at: at,
        content: content.to_string(),
        detected_types: types.to_vec(),
        source,
        copy_count,
        pinned,
    }
}

fn ghostty(repo: &str, branch: &str) -> SourceContext {
    SourceContext {
        app_name: "Ghostty".into(),
        bundle_id: Some("com.mitchellh.ghostty".into()),
        cwd: Some(format!("/Users/cesarferreira/code/github/{repo}")),
        git_repo: Some(repo.into()),
        git_branch: Some(branch.into()),
        hostname: Some("devbox".into()),
    }
}

fn android_studio(repo: &str, branch: &str) -> SourceContext {
    SourceContext {
        app_name: "Android Studio".into(),
        bundle_id: Some("com.google.android.studio".into()),
        cwd: Some(format!("/Users/cesarferreira/code/github/{repo}")),
        git_repo: Some(repo.into()),
        git_branch: Some(branch.into()),
        hostname: Some("devbox".into()),
    }
}

fn vscode(repo: &str, branch: &str) -> SourceContext {
    SourceContext {
        app_name: "VS Code".into(),
        bundle_id: Some("com.microsoft.VSCode".into()),
        cwd: Some(format!("/Users/cesarferreira/code/github/{repo}")),
        git_repo: Some(repo.into()),
        git_branch: Some(branch.into()),
        hostname: Some("devbox".into()),
    }
}

fn arc(repo: Option<&str>, branch: Option<&str>) -> SourceContext {
    SourceContext {
        app_name: "Arc".into(),
        bundle_id: Some("company.thebrowser.Browser".into()),
        cwd: None,
        git_repo: repo.map(str::to_string),
        git_branch: branch.map(str::to_string),
        hostname: Some("devbox".into()),
    }
}

fn chrome() -> SourceContext {
    SourceContext {
        app_name: "Chrome".into(),
        bundle_id: Some("com.google.Chrome".into()),
        cwd: None,
        git_repo: None,
        git_branch: None,
        hostname: Some("devbox".into()),
    }
}

fn safari() -> SourceContext {
    SourceContext {
        app_name: "Safari".into(),
        bundle_id: Some("com.apple.Safari".into()),
        cwd: None,
        git_repo: None,
        git_branch: None,
        hostname: Some("devbox".into()),
    }
}
