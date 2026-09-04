//! User config: `~/.config/stash/stash.toml`

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const DEFAULT_TOGGLE: &str = "cmd+shift+v";

const DEFAULT_TOML: &str = r#"# stash configuration
#
# Hotkeys are lowercase chords: modifiers + key, joined by +.
# Examples: cmd+shift+v, ctrl+alt+s, cmd+shift+space
# Modifiers: cmd, ctrl, alt, shift
# Keys: a–z, 0–9, space, enter, escape, tab, …

[hotkey]
# Global shortcut to show / hide the popup
toggle = "cmd+shift+v"
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub hotkey: HotkeyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Global show/hide shortcut, e.g. `"cmd+shift+v"`.
    #[serde(default = "default_toggle")]
    pub toggle: String,
}

fn default_toggle() -> String {
    DEFAULT_TOGGLE.into()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: HotkeyConfig {
                toggle: default_toggle(),
            },
        }
    }
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            toggle: default_toggle(),
        }
    }
}

/// Normalize user input to a stable lowercase chord (`cmd+shift+v`).
pub fn normalize_hotkey(raw: &str) -> String {
    raw.split('+')
        .map(|part| {
            let p = part.trim().to_ascii_lowercase();
            match p.as_str() {
                "command" | "super" | "meta" => "cmd".into(),
                "control" | "controlkey" => "ctrl".into(),
                "option" | "opt" => "alt".into(),
                // Strip legacy KeyV / Digit1 style from older configs.
                s if s.starts_with("key") && s.len() == 4 => s[3..].to_string(),
                s if s.starts_with("digit") && s.len() == 6 => s[5..].to_string(),
                other => other.to_string(),
            }
        })
        .filter(|p| !p.is_empty())
        .collect::<Vec<_>>()
        .join("+")
}

pub fn config_dir() -> PathBuf {
    dirs_fallback().join(".config/stash")
}

pub fn config_path() -> PathBuf {
    config_dir().join("stash.toml")
}

fn dirs_fallback() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Load config, creating a default `stash.toml` if missing.
pub fn load_or_create() -> Result<Config, String> {
    let path = config_path();
    if !path.exists() {
        fs::create_dir_all(config_dir()).map_err(|e| e.to_string())?;
        fs::write(&path, DEFAULT_TOML).map_err(|e| e.to_string())?;
        eprintln!("stash: wrote default config to {}", path.display());
        return Ok(Config::default());
    }

    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut config: Config = toml::from_str(&raw)
        .map_err(|e| format!("invalid config {}: {e}", path.display()))?;
    config.hotkey.toggle = normalize_hotkey(&config.hotkey.toggle);
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_toggle() {
        let cfg: Config = toml::from_str(DEFAULT_TOML).unwrap();
        assert_eq!(cfg.hotkey.toggle, "cmd+shift+v");
    }

    #[test]
    fn empty_file_uses_defaults() {
        let cfg: Config = toml::from_str("").unwrap();
        assert_eq!(cfg.hotkey.toggle, DEFAULT_TOGGLE);
    }

    #[test]
    fn normalizes_legacy_and_messy_forms() {
        assert_eq!(normalize_hotkey("Cmd+Shift+KeyV"), "cmd+shift+v");
        assert_eq!(normalize_hotkey("CMD + SHIFT + V"), "cmd+shift+v");
        assert_eq!(normalize_hotkey("command+shift+v"), "cmd+shift+v");
        assert_eq!(normalize_hotkey("ctrl+alt+Digit1"), "ctrl+alt+1");
    }
}
