//! User-configurable application settings, persisted as JSON in the app's
//! config directory.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Which colour theme the UI should use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    System,
    Dark,
    Light,
}

/// Everything the user can tweak in the Settings panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Start hidden in the tray instead of showing the window.
    pub start_minimized: bool,
    /// Hitting the window's close button hides to tray instead of quitting.
    pub minimize_to_tray_on_close: bool,
    /// Launch the app automatically on login (best-effort; platform dependent).
    pub launch_on_startup: bool,
    /// Deck brightness, 0..=100.
    pub brightness: u8,
    /// Id of the profile that is currently active.
    pub active_profile_id: String,
    /// Automatically switch profiles based on the foreground application.
    pub context_switching_enabled: bool,
    /// UI colour theme.
    pub theme: Theme,
    /// Spotify app Client ID for the Web API integration (empty = not set up).
    pub spotify_client_id: String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            start_minimized: false,
            minimize_to_tray_on_close: true,
            launch_on_startup: false,
            brightness: 80,
            active_profile_id: "default".to_string(),
            context_switching_enabled: false,
            theme: Theme::System,
            spotify_client_id: String::new(),
        }
    }
}

impl Settings {
    /// The settings file inside `config_dir`.
    pub fn path(config_dir: &Path) -> PathBuf {
        config_dir.join("settings.json")
    }

    /// Load settings, falling back to defaults if the file is missing or invalid.
    pub fn load(config_dir: &Path) -> Self {
        let path = Self::path(config_dir);
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    /// Persist settings as pretty JSON, creating the config directory if needed.
    pub fn save(&self, config_dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(config_dir)?;
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(Self::path(config_dir), json)
    }

    /// Clamp fields that came from untrusted JSON into valid ranges.
    pub fn sanitized(mut self) -> Self {
        self.brightness = self.brightness.min(100);
        if self.active_profile_id.trim().is_empty() {
            self.active_profile_id = "default".to_string();
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_reasonable() {
        let s = Settings::default();
        assert!(s.minimize_to_tray_on_close);
        assert_eq!(s.brightness, 80);
    }

    #[test]
    fn sanitize_clamps_brightness_and_profile() {
        let s = Settings { brightness: 250, active_profile_id: "  ".into(), ..Default::default() }
            .sanitized();
        assert_eq!(s.brightness, 100);
        assert_eq!(s.active_profile_id, "default");
    }

    #[test]
    fn roundtrip_via_json() {
        let s = Settings { launch_on_startup: true, theme: Theme::Dark, ..Default::default() };
        let json = serde_json::to_string(&s).unwrap();
        let back: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
