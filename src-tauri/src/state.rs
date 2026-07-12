//! Shared application state, multi-profile storage and the built-in profiles.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use deck_core::model::{Action, AppMatch, EncoderConfig, KeyConfig, Page};
use deck_core::{DeckInfo, Profile};

use crate::device::{DeviceCommand, DeviceStatus};
use crate::settings::Settings;

/// State shared across Tauri commands and worker threads.
pub struct AppState {
    pub config_dir: PathBuf,
    pub settings: Mutex<Settings>,
    /// Every profile known to the app.
    pub profiles: Mutex<Vec<Profile>>,
    /// Id of the currently active profile.
    pub active_id: Mutex<String>,
    /// Contents of the active profile, shared with the device worker thread.
    pub profile: Arc<Mutex<Profile>>,
    /// Nominal layout of the target deck, so the UI can render slots.
    pub deck_info: DeckInfo,
    /// Live connection status, updated by the device worker.
    pub device_status: Arc<Mutex<DeviceStatus>>,
    /// Channel to the device worker; `None` until the worker is spawned.
    pub device_tx: Mutex<Option<Sender<DeviceCommand>>>,
}

impl AppState {
    /// Notify the device worker, ignoring the error if it isn't running yet.
    pub fn notify_device(&self, cmd: DeviceCommand) {
        if let Ok(guard) = self.device_tx.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(cmd);
            }
        }
    }

    /// Find a profile by id (cloned).
    pub fn profile_by_id(&self, id: &str) -> Option<Profile> {
        self.profiles
            .lock()
            .ok()?
            .iter()
            .find(|p| p.id == id)
            .cloned()
    }

    /// Pick the id of the first profile whose rules match the given foreground
    /// app (matched against both the process name and the friendlier app name).
    pub fn match_profile_id(&self, process: &str, app: &str, title: &str) -> Option<String> {
        let profiles = self.profiles.lock().ok()?;
        profiles
            .iter()
            .find(|p| p.matches(process, title) || p.matches(app, title))
            .map(|p| p.id.clone())
    }
}

/// Directory holding profile JSON files.
pub fn profiles_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("profiles")
}

/// Path of a single profile file.
pub fn profile_path(config_dir: &Path, id: &str) -> PathBuf {
    profiles_dir(config_dir).join(format!("{id}.json"))
}

/// Load every profile from disk, seeding the built-in set on first run.
pub fn load_all_profiles(config_dir: &Path) -> Vec<Profile> {
    let dir = profiles_dir(config_dir);
    let mut profiles: Vec<Profile> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(mut p) = deck_core::store::load(&path) {
                    p.normalize();
                    profiles.push(p);
                }
            }
        }
    }
    if profiles.is_empty() {
        profiles = default_profiles();
        for p in &profiles {
            if let Err(e) = deck_core::store::save(p, profile_path(config_dir, &p.id)) {
                eprintln!("[state] could not write default profile {}: {e}", p.id);
            }
        }
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    profiles
}

/// Persist a profile to `config_dir/profiles/<id>.json`.
pub fn save_profile(config_dir: &Path, profile: &Profile) -> Result<(), String> {
    deck_core::store::save(profile, profile_path(config_dir, &profile.id)).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Built-in profiles
// ---------------------------------------------------------------------------

fn hotkey(keys: &[&str]) -> Action {
    Action::SendHotkey { keys: keys.iter().map(|k| k.to_string()).collect() }
}
fn launch(path: &str) -> Action {
    Action::LaunchApp { path: path.to_string(), args: vec![] }
}
fn url(u: &str) -> Action {
    Action::OpenUrl { url: u.to_string() }
}

/// Build a one-page profile from a color and a list of `(label, action)` keys.
fn make_profile(
    id: &str,
    name: &str,
    color: &str,
    process: Option<&str>,
    keys: Vec<(&str, Action)>,
    encoders: Vec<(&str, EncoderConfig)>,
) -> Profile {
    let mut page = Page::empty("main", "Main");
    for (i, (label, action)) in keys.into_iter().take(8).enumerate() {
        page.keys[i] = KeyConfig {
            label: label.to_string(),
            icon: None,
            color: Some(color.to_string()),
            action,
        };
    }
    for (i, (label, mut enc)) in encoders.into_iter().take(4).enumerate() {
        enc.label = label.to_string();
        page.encoders[i] = enc;
    }
    let activates_for = process
        .map(|p| vec![AppMatch { process: Some(p.to_string()), title_contains: None }])
        .unwrap_or_default();
    Profile { id: id.to_string(), name: name.to_string(), pages: vec![page], activates_for }
}

fn volume_encoder() -> EncoderConfig {
    EncoderConfig {
        on_turn_cw: hotkey(&["volumeup"]),
        on_turn_ccw: hotkey(&["volumedown"]),
        on_press: hotkey(&["volumemute"]),
        ..Default::default()
    }
}
fn brightness_encoder() -> EncoderConfig {
    EncoderConfig {
        on_turn_cw: Action::AdjustBrightness { delta: 10 },
        on_turn_ccw: Action::AdjustBrightness { delta: -10 },
        ..Default::default()
    }
}

/// The profiles shipped on first launch: a generic default plus one per app.
pub fn default_profiles() -> Vec<Profile> {
    vec![
        make_profile(
            "default",
            "Default",
            "#3a3d52",
            None,
            vec![
                ("Discord", launch("discord")),
                ("Spotify", launch("spotify")),
                ("Steam", launch("steam")),
                ("Claude", url("https://claude.ai")),
                ("Browser", url("https://github.com")),
                ("Screenshot", hotkey(&["printscreen"])),
                ("Mute", hotkey(&["ctrl", "shift", "m"])),
                ("Battle.net", launch("battle.net")),
            ],
            vec![
                ("Volume", volume_encoder()),
                ("Bright", brightness_encoder()),
                ("Mic", EncoderConfig { on_press: hotkey(&["f13"]), ..Default::default() }),
                ("Scene", EncoderConfig { on_press: hotkey(&["f14"]), ..Default::default() }),
            ],
        ),
        make_profile(
            "spotify",
            "Spotify",
            "#1db954",
            Some("spotify"),
            vec![
                ("Play/Pause", hotkey(&["playpause"])),
                ("Next", hotkey(&["nexttrack"])),
                ("Prev", hotkey(&["prevtrack"])),
                ("Like", hotkey(&["alt", "shift", "b"])),
                ("Shuffle", hotkey(&["ctrl", "s"])),
                ("Repeat", hotkey(&["ctrl", "r"])),
                ("Search", hotkey(&["ctrl", "l"])),
                ("Open", launch("spotify")),
            ],
            vec![
                ("Volume", volume_encoder()),
                ("Seek", EncoderConfig { on_turn_cw: hotkey(&["right"]), on_turn_ccw: hotkey(&["left"]), ..Default::default() }),
                ("Bright", brightness_encoder()),
                ("Mute", EncoderConfig { on_press: hotkey(&["volumemute"]), ..Default::default() }),
            ],
        ),
        make_profile(
            "steam",
            "Steam",
            "#66c0f4",
            Some("steam"),
            vec![
                ("Library", url("steam://open/games")),
                ("Big Picture", url("steam://open/bigpicture")),
                ("Friends", url("steam://open/friends")),
                ("Store", url("steam://open/store")),
                ("Downloads", url("steam://open/downloads")),
                ("Screenshot", hotkey(&["f12"])),
                ("Overlay", hotkey(&["shift", "tab"])),
                ("Open", launch("steam")),
            ],
            vec![
                ("Volume", volume_encoder()),
                ("Bright", brightness_encoder()),
                ("Mic", EncoderConfig { on_press: hotkey(&["f13"]), ..Default::default() }),
                ("Scene", EncoderConfig { on_press: hotkey(&["f14"]), ..Default::default() }),
            ],
        ),
        make_profile(
            "discord",
            "Discord",
            "#5865f2",
            Some("discord"),
            vec![
                ("Mute", hotkey(&["ctrl", "shift", "m"])),
                ("Deafen", hotkey(&["ctrl", "shift", "d"])),
                ("Video", hotkey(&["ctrl", "shift", "v"])),
                ("Screen", hotkey(&["ctrl", "shift", "e"])),
                ("Disconnect", hotkey(&["ctrl", "shift", "d"])),
                ("Overlay", hotkey(&["shift", "`"])),
                ("Emoji", hotkey(&["ctrl", "e"])),
                ("Open", launch("discord")),
            ],
            vec![
                ("Volume", volume_encoder()),
                ("Mic", EncoderConfig { on_press: hotkey(&["ctrl", "shift", "m"]), ..Default::default() }),
                ("Bright", brightness_encoder()),
                ("Scroll", EncoderConfig { on_turn_cw: hotkey(&["pagedown"]), on_turn_ccw: hotkey(&["pageup"]), ..Default::default() }),
            ],
        ),
        make_profile(
            "battlenet",
            "Battle.net",
            "#148eff",
            Some("battle.net"),
            vec![
                ("Launcher", launch("battle.net")),
                ("Friends", hotkey(&["f11"])),
                ("Shop", url("https://shop.battle.net")),
                ("News", url("https://news.blizzard.com")),
                ("Screenshot", hotkey(&["printscreen"])),
                ("Mute", hotkey(&["ctrl", "shift", "m"])),
                ("Discord", launch("discord")),
                ("Record", hotkey(&["alt", "f9"])),
            ],
            vec![
                ("Volume", volume_encoder()),
                ("Bright", brightness_encoder()),
                ("Mic", EncoderConfig { on_press: hotkey(&["f13"]), ..Default::default() }),
                ("Scene", EncoderConfig { on_press: hotkey(&["f14"]), ..Default::default() }),
            ],
        ),
        make_profile(
            "claude",
            "Claude",
            "#cc785c",
            Some("claude"),
            vec![
                ("New Chat", hotkey(&["ctrl", "k"])),
                ("Open", url("https://claude.ai")),
                ("Projects", url("https://claude.ai/projects")),
                ("Copy", hotkey(&["ctrl", "c"])),
                ("Paste", hotkey(&["ctrl", "v"])),
                ("Search", hotkey(&["ctrl", "f"])),
                ("Sidebar", hotkey(&["ctrl", "b"])),
                ("Send", hotkey(&["ctrl", "enter"])),
            ],
            vec![
                ("Volume", volume_encoder()),
                ("Bright", brightness_encoder()),
                ("Scroll", EncoderConfig { on_turn_cw: hotkey(&["pagedown"]), on_turn_ccw: hotkey(&["pageup"]), ..Default::default() }),
                ("Zoom", EncoderConfig { on_turn_cw: hotkey(&["ctrl", "+"]), on_turn_ccw: hotkey(&["ctrl", "-"]), ..Default::default() }),
            ],
        ),
    ]
}
