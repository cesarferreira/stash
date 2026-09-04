//! Bundled UI fonts for crisp GPUI text rendering.

use std::borrow::Cow;

use gpui::App;

pub const UI_MONO: &str = "Lilex";

pub fn load(cx: &App) {
    let fonts: Vec<Cow<'static, [u8]>> = vec![
        Cow::Borrowed(include_bytes!("../assets/fonts/lilex/Lilex-Regular.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/lilex/Lilex-Bold.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/lilex/Lilex-Italic.ttf").as_slice()),
        Cow::Borrowed(include_bytes!("../assets/fonts/lilex/Lilex-BoldItalic.ttf").as_slice()),
    ];
    if let Err(err) = cx.text_system().add_fonts(fonts) {
        eprintln!("failed to load UI fonts: {err:#}");
    }
}
