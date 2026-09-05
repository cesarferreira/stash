//! Fetch a visual preview for a copied URL (og:image, then screenshot fallback).

use crate::clipboard::{accent_from_png, hash_bytes};
use std::io::Read;
use std::time::Duration;
use url::Url;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) stash/0.1";
const HTML_LIMIT: u64 = 512 * 1024;
const IMAGE_LIMIT: u64 = 5 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Debug, Clone)]
pub struct LinkPreviewImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub label: String,
    pub accent: u32,
    pub content_hash: String,
}

pub fn is_previewable_url(content: &str, types: &[crate::model::ContentType]) -> bool {
    use crate::model::ContentType;
    if !(types.contains(&ContentType::Url) || types.contains(&ContentType::GitHubUrl)) {
        return false;
    }
    let t = content.trim();
    t.starts_with("http://") || t.starts_with("https://")
}

/// Download a site preview image for `url`. Prefer Open Graph art; fall back to
/// a remote screenshot service when the page has no usable image meta.
pub fn fetch_link_preview(page_url: &str) -> Result<LinkPreviewImage, String> {
    let page_url = page_url.trim();
    let parsed = Url::parse(page_url).map_err(|e| format!("bad url: {e}"))?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("only http(s) urls supported".into());
    }

    let host = parsed.host_str().unwrap_or("site").to_string();

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(TIMEOUT)
        .timeout_read(TIMEOUT)
        .user_agent(USER_AGENT)
        .build();

    let mut errors = Vec::new();

    match fetch_html(&agent, page_url) {
        Ok(html) => {
            if let Some(image_url) = extract_preview_image_url(&html, &parsed) {
                match download_as_png(&agent, &image_url) {
                    Ok(preview) => {
                        return Ok(LinkPreviewImage {
                            label: host,
                            ..preview
                        });
                    }
                    Err(err) => errors.push(format!("og image: {err}")),
                }
            } else {
                errors.push("no og:image / twitter:image".into());
            }
        }
        Err(err) => errors.push(format!("html: {err}")),
    }

    // Screenshot fallback (third-party). Used only when the page itself has no
    // usable Open Graph image.
    let shot = format!(
        "https://image.thum.io/get/maxAge/24/width/900/crop/560/noanimate/{}",
        page_url
    );
    match download_as_png(&agent, &shot) {
        Ok(preview) => {
            return Ok(LinkPreviewImage {
                label: host,
                ..preview
            });
        }
        Err(err) => errors.push(format!("screenshot: {err}")),
    }

    Err(errors.join("; "))
}

fn fetch_html(agent: &ureq::Agent, url: &str) -> Result<String, String> {
    let response = agent
        .get(url)
        .set("Accept", "text/html,application/xhtml+xml")
        .call()
        .map_err(|e| e.to_string())?;
    let mut reader = response.into_reader().take(HTML_LIMIT);
    let mut buf = String::new();
    reader
        .read_to_string(&mut buf)
        .map_err(|e| format!("read html: {e}"))?;
    Ok(buf)
}

fn download_as_png(agent: &ureq::Agent, image_url: &str) -> Result<LinkPreviewImage, String> {
    let response = agent
        .get(image_url)
        .set("Accept", "image/*,*/*;q=0.8")
        .call()
        .map_err(|e| e.to_string())?;
    let mut reader = response.into_reader().take(IMAGE_LIMIT);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| format!("read image: {e}"))?;
    if bytes.is_empty() {
        return Err("empty image".into());
    }

    let img = image::load_from_memory(&bytes).map_err(|e| format!("decode image: {e}"))?;
    // Keep preview pane sized reasonably.
    let img = img.thumbnail(1200, 800);
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    let mut png = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut png);
    use image::ImageEncoder;
    encoder
        .write_image(
            rgba.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("encode png: {e}"))?;

    let content_hash = hash_bytes(&png);
    let accent = accent_from_png(&png);
    Ok(LinkPreviewImage {
        png,
        width,
        height,
        label: "preview".into(),
        accent,
        content_hash,
    })
}

fn extract_preview_image_url(html: &str, base: &Url) -> Option<String> {
    let lower = html.to_ascii_lowercase();
    // Prefer Open Graph, then Twitter cards.
    for (prop_key, prop_val) in [
        ("property", "og:image"),
        ("name", "twitter:image"),
        ("name", "twitter:image:src"),
        ("property", "og:image:url"),
        ("itemprop", "image"),
    ] {
        if let Some(raw) = find_meta_content(&lower, html, prop_key, prop_val) {
            if let Some(abs) = resolve_url(base, &raw) {
                return Some(abs);
            }
        }
    }
    None
}

/// Case-insensitive meta tag content lookup; returns content from original HTML casing.
fn find_meta_content(lower: &str, original: &str, attr: &str, value: &str) -> Option<String> {
    let needle = format!("{attr}=\"{value}\"");
    let needle_sq = format!("{attr}='{value}'");
    let pos = lower
        .find(&needle)
        .or_else(|| lower.find(&needle_sq))?;
    // Walk back to the start of this <meta ...> tag.
    let tag_start = lower[..pos].rfind("<meta")?;
    let tag_end = lower[tag_start..].find('>')? + tag_start;
    let tag_lower = &lower[tag_start..tag_end];
    let tag_orig = &original[tag_start..tag_end];

    content_from_tag(tag_lower, tag_orig)
}

fn content_from_tag(tag_lower: &str, tag_orig: &str) -> Option<String> {
    for key in ["content=\"", "content='"] {
        if let Some(start) = tag_lower.find(key) {
            let quote = key.as_bytes()[key.len() - 1] as char;
            let value_start = start + key.len();
            let rest_lower = &tag_lower[value_start..];
            let end = rest_lower.find(quote)?;
            let value = tag_orig[value_start..value_start + end].trim();
            if !value.is_empty() {
                return Some(html_unescape(value));
            }
        }
    }
    None
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn resolve_url(base: &Url, raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.starts_with("data:") {
        return None;
    }
    base.join(raw).ok().map(|u| u.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_og_image() {
        let html = r#"
        <html><head>
          <meta property="og:title" content="Hello">
          <meta property="og:image" content="https://cdn.example.com/card.jpg">
        </head></html>
        "#;
        let base = Url::parse("https://example.com/post").unwrap();
        assert_eq!(
            extract_preview_image_url(html, &base).as_deref(),
            Some("https://cdn.example.com/card.jpg")
        );
    }

    #[test]
    fn resolves_relative_twitter_image() {
        let html = r#"<meta name="twitter:image" content="/img/share.png">"#;
        let base = Url::parse("https://example.com/a/b").unwrap();
        assert_eq!(
            extract_preview_image_url(html, &base).as_deref(),
            Some("https://example.com/img/share.png")
        );
    }
}
