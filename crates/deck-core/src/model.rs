//! The configuration model: profiles, pages and the actions bound to keys and
//! encoders. This is what gets serialized to disk (JSON) and edited in the UI.
//!
//! The Stream Deck + exposes two kinds of controls:
//!   * 8 LCD **keys** — an image plus a press action.
//!   * 4 **encoders** (dials) — each pairs a rotary dial (turn + press) with a
//!     segment of the touch strip (tap). Elgato calls this combination an
//!     "Encoder".

use serde::{Deserialize, Serialize};

/// Number of LCD keys on a Stream Deck +.
pub const KEY_COUNT: usize = 8;
/// Number of encoders (dials) on a Stream Deck +.
pub const ENCODER_COUNT: usize = 4;

/// A single action that can be triggered by a key press, an encoder press/turn
/// or a touch-strip tap.
///
/// The variants are intentionally serialized with an internal `type` tag so the
/// on-disk JSON and the messages sent to the frontend stay readable and stable
/// as new action kinds are added.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    /// Does nothing. The default for unassigned controls.
    #[default]
    None,
    /// Launch an executable with optional arguments.
    LaunchApp { path: String, #[serde(default)] args: Vec<String> },
    /// Run a command through the user's shell.
    RunCommand { command: String },
    /// Open a URL in the default browser.
    OpenUrl { url: String },
    /// Send a key combination to the focused window, e.g. `["ctrl", "shift", "m"]`.
    SendHotkey { keys: Vec<String> },
    /// Switch the active page within the current profile.
    SwitchPage { page_id: String },
    /// Adjust deck brightness by a relative amount (percent, may be negative).
    AdjustBrightness { delta: i16 },
    /// Control Spotify via the Web API (reliable, targets Spotify regardless of
    /// focus). `op` is one of `play_pause`, `next`, `prev`.
    Spotify { op: String },
}

impl Action {
    /// Whether this action actually does something when triggered.
    pub fn is_bound(&self) -> bool {
        !matches!(self, Action::None)
    }
}

/// Visual + behaviour configuration for one LCD key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct KeyConfig {
    /// Text drawn on the key (may be empty).
    #[serde(default)]
    pub label: String,
    /// Optional path to an icon image rendered on the key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Background colour as a CSS-style hex string, e.g. `#1e1e2e`.
    #[serde(default)]
    pub color: Option<String>,
    /// What happens when the key is pressed.
    #[serde(default)]
    pub action: Action,
}

/// Visual + behaviour configuration for one encoder (dial + touch-strip segment).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct EncoderConfig {
    /// Text shown above the dial on the touch strip.
    #[serde(default)]
    pub label: String,
    /// Optional icon shown on the touch-strip segment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Triggered when the dial is pushed in.
    #[serde(default)]
    pub on_press: Action,
    /// Triggered on each detent when turning clockwise.
    #[serde(default)]
    pub on_turn_cw: Action,
    /// Triggered on each detent when turning counter-clockwise.
    #[serde(default)]
    pub on_turn_ccw: Action,
    /// Triggered when the touch-strip segment above the dial is tapped.
    #[serde(default)]
    pub on_touch: Action,
}

impl EncoderConfig {
    /// Whether any of this encoder's four gestures is bound to an action.
    pub fn has_binding(&self) -> bool {
        self.on_press.is_bound()
            || self.on_turn_cw.is_bound()
            || self.on_turn_ccw.is_bound()
            || self.on_touch.is_bound()
    }
}

/// A single page (layout) of keys and encoders.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Page {
    pub id: String,
    #[serde(default)]
    pub name: String,
    /// Exactly [`KEY_COUNT`] entries once normalized.
    #[serde(default)]
    pub keys: Vec<KeyConfig>,
    /// Exactly [`ENCODER_COUNT`] entries once normalized.
    #[serde(default)]
    pub encoders: Vec<EncoderConfig>,
}

impl Page {
    /// Create an empty page with fully populated key/encoder slots.
    pub fn empty(id: impl Into<String>, name: impl Into<String>) -> Self {
        Page {
            id: id.into(),
            name: name.into(),
            keys: vec![KeyConfig::default(); KEY_COUNT],
            encoders: vec![EncoderConfig::default(); ENCODER_COUNT],
        }
    }

    /// Pad or truncate the key/encoder vectors so they always have the exact
    /// slot count. Tolerant of hand-edited JSON with missing/extra entries.
    pub fn normalize(&mut self) {
        self.keys.resize(KEY_COUNT, KeyConfig::default());
        self.encoders.resize(ENCODER_COUNT, EncoderConfig::default());
    }
}

/// A rule describing when a profile should automatically activate, based on the
/// foreground application. Reserved for the context-aware engine; not yet
/// evaluated but modelled so profiles are forward-compatible.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AppMatch {
    /// Match against the process/executable name (case-insensitive substring).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub process: Option<String>,
    /// Match against the window title (case-insensitive substring).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title_contains: Option<String>,
}

/// A named collection of pages, optionally bound to one or more applications.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub pages: Vec<Page>,
    /// Applications that should auto-activate this profile (context engine).
    #[serde(default)]
    pub activates_for: Vec<AppMatch>,
}

impl Profile {
    /// A minimal profile with a single empty page.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Profile {
            id: id.into(),
            name: name.into(),
            pages: vec![Page::empty("main", "Main")],
            activates_for: Vec::new(),
        }
    }

    /// Look up a page by id.
    pub fn page(&self, page_id: &str) -> Option<&Page> {
        self.pages.iter().find(|p| p.id == page_id)
    }

    /// Normalize every page's slot counts.
    pub fn normalize(&mut self) {
        for page in &mut self.pages {
            page.normalize();
        }
    }

    /// Does this profile want to be active for the given foreground app?
    pub fn matches(&self, process: &str, title: &str) -> bool {
        self.activates_for.iter().any(|m| {
            let proc_ok = m
                .process
                .as_deref()
                .map(|p| process.to_lowercase().contains(&p.to_lowercase()))
                .unwrap_or(true);
            let title_ok = m
                .title_contains
                .as_deref()
                .map(|t| title.to_lowercase().contains(&t.to_lowercase()))
                .unwrap_or(true);
            // An empty rule (both None) never matches — it would hijack every app.
            (m.process.is_some() || m.title_contains.is_some()) && proc_ok && title_ok
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_page_has_full_slots() {
        let p = Page::empty("main", "Main");
        assert_eq!(p.keys.len(), KEY_COUNT);
        assert_eq!(p.encoders.len(), ENCODER_COUNT);
    }

    #[test]
    fn normalize_pads_and_truncates() {
        let mut p = Page {
            id: "x".into(),
            name: "X".into(),
            keys: vec![KeyConfig::default(); 3],
            encoders: vec![EncoderConfig::default(); 9],
        };
        p.normalize();
        assert_eq!(p.keys.len(), KEY_COUNT);
        assert_eq!(p.encoders.len(), ENCODER_COUNT);
    }

    #[test]
    fn action_roundtrips_through_json() {
        let a = Action::SendHotkey {
            keys: vec!["ctrl".into(), "c".into()],
        };
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("\"type\":\"send_hotkey\""));
        let back: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(a, back);
    }

    #[test]
    fn profile_matches_process_case_insensitively() {
        let mut prof = Profile::new("obs", "OBS");
        prof.activates_for.push(AppMatch {
            process: Some("obs".into()),
            title_contains: None,
        });
        assert!(prof.matches("OBS.exe", "OBS 30"));
        assert!(!prof.matches("chrome", "OBS in a tab"));
    }

    #[test]
    fn empty_match_rule_never_hijacks() {
        let mut prof = Profile::new("x", "X");
        prof.activates_for.push(AppMatch::default());
        assert!(!prof.matches("anything", "anything"));
    }
}
