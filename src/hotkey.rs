//! Global hotkey registration (macOS main-thread manager).

use crate::config::normalize_hotkey;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use std::str::FromStr;

pub struct HotkeyService {
    _manager: GlobalHotKeyManager,
    hotkey_id: u32,
    pub label: String,
}

impl HotkeyService {
    pub fn start(toggle: &str) -> Result<Self, String> {
        let label = normalize_hotkey(toggle);
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        let hotkey = HotKey::from_str(&label)
            .map_err(|e| format!("invalid hotkey `{label}`: {e}"))?;
        manager
            .register(hotkey)
            .map_err(|e| format!("failed to register `{label}`: {e}"))?;
        Ok(Self {
            _manager: manager,
            hotkey_id: hotkey.id(),
            label,
        })
    }

    /// Drain pending hotkey events; true if the configured toggle was pressed.
    pub fn take_toggle_pressed(&self) -> bool {
        let mut pressed = false;
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.id == self.hotkey_id && event.state == HotKeyState::Pressed {
                pressed = true;
            }
        }
        pressed
    }
}

pub fn app_is_hidden() -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    NSApplication::sharedApplication(mtm).isHidden()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_chord() {
        let hotkey = HotKey::from_str("cmd+shift+v").expect("parse");
        assert_eq!(hotkey.id(), HotKey::from_str("CMD+SHIFT+V").unwrap().id());
    }
}
