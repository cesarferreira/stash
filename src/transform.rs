//! Content transformations used by the action palette.

use crate::model::{ClipboardEntry, ContentType};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::Value;
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformError {
    Message(String),
}

impl TransformError {
    pub fn message(&self) -> &str {
        match self {
            Self::Message(msg) => msg,
        }
    }
}

pub fn pretty_json(content: &str) -> Result<String, TransformError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|err| TransformError::Message(format!("invalid JSON: {err}")))?;
    serde_json::to_string_pretty(&value)
        .map_err(|err| TransformError::Message(format!("pretty print failed: {err}")))
}

pub fn minify_json(content: &str) -> Result<String, TransformError> {
    let value: Value = serde_json::from_str(content)
        .map_err(|err| TransformError::Message(format!("invalid JSON: {err}")))?;
    serde_json::to_string(&value)
        .map_err(|err| TransformError::Message(format!("minify failed: {err}")))
}

pub fn remove_tracking_params(content: &str) -> Result<String, TransformError> {
    let mut url = Url::parse(content.trim())
        .map_err(|err| TransformError::Message(format!("invalid URL: {err}")))?;
    let filtered: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(key, _)| {
            let key = key.to_ascii_lowercase();
            !(key.starts_with("utm_")
                || key == "fbclid"
                || key == "gclid"
                || key == "mc_cid"
                || key == "mc_eid")
        })
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

    if filtered.is_empty() {
        url.set_query(None);
    } else {
        let query = filtered
            .into_iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        url.set_query(Some(&query));
    }
    Ok(url.to_string())
}

pub fn decode_jwt(content: &str) -> Result<String, TransformError> {
    let parts: Vec<&str> = content.trim().split('.').collect();
    if parts.len() < 2 {
        return Err(TransformError::Message(
            "JWT must have at least header and payload".into(),
        ));
    }

    let header = decode_jwt_part(parts[0], "header")?;
    let payload = decode_jwt_part(parts[1], "payload")?;
    Ok(format!(
        "{{\n  \"header\": {header},\n  \"payload\": {payload}\n}}"
    ))
}

fn decode_jwt_part(part: &str, label: &str) -> Result<String, TransformError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(part)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(part))
        .map_err(|err| TransformError::Message(format!("invalid JWT {label}: {err}")))?;
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|err| TransformError::Message(format!("JWT {label} is not JSON: {err}")))?;
    serde_json::to_string_pretty(&value)
        .map_err(|err| TransformError::Message(format!("failed to format JWT {label}: {err}")))
}

pub fn transform_entry(
    entry: &ClipboardEntry,
    kind: TransformKind,
) -> Result<String, TransformError> {
    match kind {
        TransformKind::PrettyJson => pretty_json(&entry.content),
        TransformKind::MinifyJson => minify_json(&entry.content),
        TransformKind::RemoveTracking => remove_tracking_params(&entry.content),
        TransformKind::DecodeJwt => decode_jwt(&entry.content),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    PrettyJson,
    MinifyJson,
    RemoveTracking,
    DecodeJwt,
}

pub fn openable_url(entry: &ClipboardEntry) -> Option<&str> {
    if entry.detected_types.contains(&ContentType::Url)
        || entry.detected_types.contains(&ContentType::GitHubUrl)
    {
        Some(entry.content.trim())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_and_minify_json_round_trip_shape() {
        let raw = r#"{"user_id":123,"enabled":true}"#;
        let pretty = pretty_json(raw).unwrap();
        assert!(pretty.contains('\n'));
        let mini = minify_json(&pretty).unwrap();
        assert!(!mini.contains('\n'));
        assert!(mini.contains("\"user_id\":123"));
    }

    #[test]
    fn strips_utm_params() {
        let cleaned =
            remove_tracking_params("https://example.com/foo?id=123&utm_source=x&utm_campaign=y")
                .unwrap();
        assert_eq!(cleaned, "https://example.com/foo?id=123");
    }

    #[test]
    fn decodes_jwt_claims() {
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiZXhwIjoxNzE2MjM5MDIyLCJyb2xlIjoiYWRtaW4ifQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let decoded = decode_jwt(token).unwrap();
        assert!(decoded.contains("\"role\": \"admin\"") || decoded.contains("\"role\":\"admin\""));
        assert!(decoded.contains("header"));
    }
}
