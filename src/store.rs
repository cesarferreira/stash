//! SQLite-backed clipboard history. Survives app restarts and reboots.

use crate::clipboard::{Capture, CaptureKind, accent_from_png};
use crate::detect::detect_types;
use crate::model::{ClipboardEntry, ContentType, ImageMeta, SourceContext};
use crate::paths::{blob_path_for_hash, db_path, ensure_data_dirs};
use crate::tags::tags_for;
use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open() -> Result<Self, String> {
        ensure_data_dirs().map_err(|e| e.to_string())?;
        Self::open_at(&db_path())
    }

    pub fn open_at(path: &std::path::Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            PRAGMA journal_mode=WAL;
            PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                last_copied_at INTEGER NOT NULL,
                content_type TEXT NOT NULL,
                detected_types TEXT NOT NULL,
                text_content TEXT,
                blob_path TEXT,
                content_hash TEXT NOT NULL,
                source_app TEXT,
                source_bundle_id TEXT,
                image_width INTEGER,
                image_height INTEGER,
                image_label TEXT,
                image_accent INTEGER,
                copy_count INTEGER NOT NULL DEFAULT 1,
                pinned INTEGER NOT NULL DEFAULT 0,
                tags TEXT NOT NULL DEFAULT '[]'
            );
            CREATE INDEX IF NOT EXISTS idx_entries_hash ON entries(content_hash);
            CREATE INDEX IF NOT EXISTS idx_entries_last ON entries(last_copied_at DESC);
            ",
        )
        .map_err(|e| e.to_string())?;
        // Existing DBs created before tags existed.
        let _ = conn.execute(
            "ALTER TABLE entries ADD COLUMN tags TEXT NOT NULL DEFAULT '[]'",
            [],
        );
        Ok(Self { conn })
    }

    pub fn list_entries(&self) -> Result<Vec<ClipboardEntry>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, created_at, last_copied_at, content_type, detected_types,
                        text_content, blob_path, content_hash, source_app, source_bundle_id,
                        image_width, image_height, image_label, image_accent,
                        copy_count, pinned, tags
                 FROM entries
                 ORDER BY pinned DESC, last_copied_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], row_to_entry)
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        Ok(rows)
    }

    /// Insert or bump an existing hash. Returns true if the visible history changed.
    pub fn record(&mut self, capture: Capture) -> Result<bool, String> {
        let now = Utc::now().timestamp();
        if let Some(existing_id) = self.latest_id_for_hash(&capture.content_hash)? {
            self.conn
                .execute(
                    "UPDATE entries
                     SET last_copied_at = ?1,
                         copy_count = copy_count + 1,
                         source_app = ?2,
                         source_bundle_id = ?3
                     WHERE id = ?4",
                    params![
                        now,
                        capture.source.app_name,
                        capture.source.bundle_id,
                        existing_id
                    ],
                )
                .map_err(|e| e.to_string())?;
            return Ok(true);
        }

        let id = Uuid::new_v4().to_string();
        match capture.kind {
            CaptureKind::Text(text) => {
                let types = detect_types(&text);
                let primary = types.first().copied().unwrap_or(ContentType::PlainText);
                let tags = tags_for(&types, false);
                self.conn
                    .execute(
                        "INSERT INTO entries (
                            id, created_at, last_copied_at, content_type, detected_types,
                            text_content, blob_path, content_hash, source_app, source_bundle_id,
                            copy_count, pinned, tags
                         ) VALUES (?1,?2,?2,?3,?4,?5,NULL,?6,?7,?8,1,0,?9)",
                        params![
                            id,
                            now,
                            primary.label(),
                            types_to_json(&types),
                            text,
                            capture.content_hash,
                            capture.source.app_name,
                            capture.source.bundle_id,
                            tags_to_json(&tags),
                        ],
                    )
                    .map_err(|e| e.to_string())?;
            }
            CaptureKind::Image { png, width, height } => {
                let path = self.write_blob(&capture.content_hash, &png)?;
                let accent = accent_from_png(&png) as i64;
                let label = format!("PNG {width}×{height}");
                let types = vec![ContentType::Image];
                let tags = tags_for(&types, false);
                self.conn
                    .execute(
                        "INSERT INTO entries (
                            id, created_at, last_copied_at, content_type, detected_types,
                            text_content, blob_path, content_hash, source_app, source_bundle_id,
                            image_width, image_height, image_label, image_accent,
                            copy_count, pinned, tags
                         ) VALUES (?1,?2,?2,?3,?4,NULL,?5,?6,?7,?8,?9,?10,?11,?12,1,0,?13)",
                        params![
                            id,
                            now,
                            ContentType::Image.label(),
                            types_to_json(&types),
                            path.to_string_lossy(),
                            capture.content_hash,
                            capture.source.app_name,
                            capture.source.bundle_id,
                            width as i64,
                            height as i64,
                            label,
                            accent,
                            tags_to_json(&tags),
                        ],
                    )
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(true)
    }

    pub fn set_pinned(&mut self, id: &str, pinned: bool) -> Result<(), String> {
        let types_json: String = self
            .conn
            .query_row(
                "SELECT detected_types FROM entries WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;
        let types = types_from_json(&types_json);
        let tags = tags_for(&types, pinned);
        self.conn
            .execute(
                "UPDATE entries SET pinned = ?1, tags = ?2 WHERE id = ?3",
                params![pinned as i64, tags_to_json(&tags), id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let blob: Option<String> = self
            .conn
            .query_row(
                "SELECT blob_path FROM entries WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .flatten();

        self.conn
            .execute("DELETE FROM entries WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;

        if let Some(path) = blob {
            let still_used: i64 = self
                .conn
                .query_row(
                    "SELECT COUNT(*) FROM entries WHERE blob_path = ?1",
                    params![path],
                    |row| row.get(0),
                )
                .unwrap_or(0);
            if still_used == 0 {
                let _ = fs::remove_file(&path);
            }
        }
        Ok(())
    }

    /// Attach a website preview thumbnail to a URL entry (reuses image blob columns).
    pub fn attach_link_preview(
        &mut self,
        id: &str,
        preview: &crate::link_preview::LinkPreviewImage,
    ) -> Result<(), String> {
        let path = self.write_blob(&preview.content_hash, &preview.png)?;
        let label = format!("preview · {}", preview.label);
        self.conn
            .execute(
                "UPDATE entries
                 SET blob_path = ?1,
                     image_width = ?2,
                     image_height = ?3,
                     image_label = ?4,
                     image_accent = ?5
                 WHERE id = ?6",
                params![
                    path.to_string_lossy(),
                    preview.width as i64,
                    preview.height as i64,
                    label,
                    preview.accent as i64,
                    id,
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// URL entries that still need a website preview fetched.
    pub fn urls_missing_preview(&self) -> Result<Vec<(String, String)>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, text_content, detected_types, blob_path
                 FROM entries
                 WHERE text_content IS NOT NULL
                 ORDER BY last_copied_at DESC
                 LIMIT 40",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let text: String = row.get(1)?;
                let types: String = row.get(2)?;
                let blob: Option<String> = row.get(3)?;
                Ok((id, text, types, blob))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;

        Ok(rows
            .into_iter()
            .filter(|(_, text, types, blob)| {
                blob.is_none() && {
                    let types = types_from_json(types);
                    crate::link_preview::is_previewable_url(text, &types)
                }
            })
            .map(|(id, text, _, _)| (id, text.trim().to_string()))
            .collect())
    }

    fn latest_id_for_hash(&self, hash: &str) -> Result<Option<String>, String> {
        self.conn
            .query_row(
                "SELECT id FROM entries WHERE content_hash = ?1
                 ORDER BY last_copied_at DESC LIMIT 1",
                params![hash],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    fn write_blob(&self, content_hash: &str, png: &[u8]) -> Result<PathBuf, String> {
        let path = blob_path_for_hash(content_hash);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        if !path.exists() {
            fs::write(&path, png).map_err(|e| e.to_string())?;
        }
        Ok(path)
    }
}

fn tags_to_json(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".into())
}

fn tags_from_json(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn types_to_json(types: &[ContentType]) -> String {
    let labels: Vec<&str> = types.iter().map(|t| t.label()).collect();
    serde_json::to_string(&labels).unwrap_or_else(|_| "[\"text\"]".into())
}

fn types_from_json(raw: &str) -> Vec<ContentType> {
    let Ok(labels) = serde_json::from_str::<Vec<String>>(raw) else {
        return vec![ContentType::PlainText];
    };
    let mut out: Vec<_> = labels
        .iter()
        .filter_map(|l| ContentType::from_label(l))
        .collect();
    if out.is_empty() {
        out.push(ContentType::PlainText);
    }
    out
}

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardEntry> {
    let id: String = row.get(0)?;
    let created_at: i64 = row.get(1)?;
    let last_copied_at: i64 = row.get(2)?;
    let _content_type: String = row.get(3)?;
    let detected_types: String = row.get(4)?;
    let text_content: Option<String> = row.get(5)?;
    let blob_path: Option<String> = row.get(6)?;
    let _content_hash: String = row.get(7)?;
    let source_app: Option<String> = row.get(8)?;
    let source_bundle_id: Option<String> = row.get(9)?;
    let image_width: Option<i64> = row.get(10)?;
    let image_height: Option<i64> = row.get(11)?;
    let image_label: Option<String> = row.get(12)?;
    let image_accent: Option<i64> = row.get(13)?;
    let copy_count: i64 = row.get(14)?;
    let pinned: i64 = row.get(15)?;
    let tags_raw: String = row.get(16).unwrap_or_else(|_| "[]".into());

    let image_fields = match (blob_path, image_width, image_height) {
        (Some(path), Some(w), Some(h)) => Some(ImageMeta {
            width: w as u32,
            height: h as u32,
            label: image_label.unwrap_or_else(|| "image".into()),
            accent: image_accent.unwrap_or(0x7aa2f7) as u32,
            path: PathBuf::from(path),
        }),
        _ => None,
    };

    let detected_types = types_from_json(&detected_types);
    let is_url = detected_types.contains(&ContentType::Url)
        || detected_types.contains(&ContentType::GitHubUrl);
    let is_image = detected_types.contains(&ContentType::Image);
    let (image, link_preview) = if is_image {
        (image_fields, None)
    } else if is_url {
        (None, image_fields)
    } else {
        (None, None)
    };

    let pinned = pinned != 0;
    let mut tags = tags_from_json(&tags_raw);
    if tags.is_empty() {
        tags = tags_for(&detected_types, pinned);
    }

    Ok(ClipboardEntry {
        id,
        created_at: secs_to_utc(created_at),
        last_copied_at: secs_to_utc(last_copied_at),
        content: text_content.unwrap_or_default(),
        detected_types,
        source: SourceContext {
            app_name: source_app.unwrap_or_else(|| "Unknown".into()),
            bundle_id: source_bundle_id,
            cwd: None,
            git_repo: None,
            git_branch: None,
            hostname: None,
        },
        copy_count: copy_count.max(1) as u32,
        pinned,
        image,
        link_preview,
        tags,
    })
}

fn secs_to_utc(secs: i64) -> DateTime<Utc> {
    Utc.timestamp_opt(secs, 0).single().unwrap_or_else(Utc::now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard::CaptureKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_store() -> Store {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("stash-test-{nanos}"));
        fs::create_dir_all(&dir).unwrap();
        Store::open_at(&dir.join("clipboard.sqlite")).unwrap()
    }

    #[test]
    fn dedupes_identical_text() {
        let mut store = temp_store();
        let capture = Capture {
            kind: CaptureKind::Text("hello".into()),
            content_hash: "abc".into(),
            source: SourceContext {
                app_name: "Terminal".into(),
                bundle_id: Some("com.apple.Terminal".into()),
                cwd: None,
                git_repo: None,
                git_branch: None,
                hostname: None,
            },
        };
        store.record(capture.clone()).unwrap();
        store.record(capture).unwrap();
        let entries = store.list_entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].copy_count, 2);
        assert_eq!(entries[0].content, "hello");
    }
}
