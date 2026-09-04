//! macOS clipboard read + frontmost app (main-thread safe).

use crate::model::SourceContext;
use arboard::Clipboard;
use objc2_app_kit::{NSPasteboard, NSRunningApplication, NSWorkspace};
use objc2_foundation::NSString;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
pub enum CaptureKind {
    Text(String),
    Image {
        png: Vec<u8>,
        width: u32,
        height: u32,
    },
}

#[derive(Debug, Clone)]
pub struct Capture {
    pub kind: CaptureKind,
    pub content_hash: String,
    pub source: SourceContext,
}

pub fn pasteboard_change_count() -> isize {
    let pb = NSPasteboard::generalPasteboard();
    pb.changeCount()
}

pub fn frontmost_source() -> SourceContext {
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication();
    match app {
        Some(app) => source_from_running_app(&app),
        None => SourceContext {
            app_name: "Unknown".into(),
            bundle_id: None,
            cwd: None,
            git_repo: None,
            git_branch: None,
            hostname: None,
        },
    }
}

fn source_from_running_app(app: &NSRunningApplication) -> SourceContext {
    let app_name = app
        .localizedName()
        .map(|s| nsstring_to_string(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unknown".into());
    let bundle_id = app
        .bundleIdentifier()
        .map(|s| nsstring_to_string(&s))
        .filter(|s| !s.is_empty());
    SourceContext {
        app_name,
        bundle_id,
        cwd: None,
        git_repo: None,
        git_branch: None,
        hostname: None,
    }
}

fn nsstring_to_string(s: &NSString) -> String {
    s.to_string()
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

/// Read current clipboard. Prefers text; falls back to image.
pub fn read_capture() -> Option<Capture> {
    let source = frontmost_source();
    let mut clipboard = Clipboard::new().ok()?;

    if let Ok(text) = clipboard.get_text() {
        let trimmed = text.trim_end_matches('\0');
        if !trimmed.is_empty() {
            let content_hash = hash_bytes(trimmed.as_bytes());
            return Some(Capture {
                kind: CaptureKind::Text(trimmed.to_string()),
                content_hash,
                source,
            });
        }
    }

    let image = clipboard.get_image().ok()?;
    if image.width == 0 || image.height == 0 || image.bytes.is_empty() {
        return None;
    }
    let png = rgba_to_png(&image.bytes, image.width as u32, image.height as u32)?;
    let content_hash = hash_bytes(&png);
    Some(Capture {
        kind: CaptureKind::Image {
            width: image.width as u32,
            height: image.height as u32,
            png,
        },
        content_hash,
        source,
    })
}

fn rgba_to_png(rgba: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let expected = (width as usize).checked_mul(height as usize)?.checked_mul(4)?;
    if rgba.len() < expected {
        return None;
    }
    let img = image::RgbaImage::from_raw(width, height, rgba[..expected].to_vec())?;
    let mut out = Vec::new();
    let encoder = image::codecs::png::PngEncoder::new(&mut out);
    use image::ImageEncoder;
    encoder
        .write_image(
            img.as_raw(),
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(out)
}

/// Dominant-ish accent from PNG bytes for list thumbnails.
pub fn accent_from_png(png: &[u8]) -> u32 {
    let Ok(img) = image::load_from_memory(png) else {
        return 0x7aa2f7;
    };
    let rgba = img.thumbnail(32, 32).to_rgba8();
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let mut n = 0u64;
    for pixel in rgba.pixels() {
        if pixel.0[3] < 16 {
            continue;
        }
        r += pixel.0[0] as u64;
        g += pixel.0[1] as u64;
        b += pixel.0[2] as u64;
        n += 1;
    }
    if n == 0 {
        return 0x7aa2f7;
    }
    let r = (r / n) as u32;
    let g = (g / n) as u32;
    let b = (b / n) as u32;
    (r << 16) | (g << 8) | b
}
