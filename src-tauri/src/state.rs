//! Shared application state and profile persistence.

use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use deck_core::model::{Action, Page};
use deck_core::{DeckInfo, Profile};

use crate::device::DeviceCommand;
use crate::settings::Settings;

/// State shared across Tauri commands. Kept behind interior mutability so command
/// handlers can take `&State`.
pub struct AppState {
    pub config_dir: PathBuf,
    pub settings: Mutex<Settings>,
    /// The active profile, shared with the device worker thread.
    pub profile: Arc<Mutex<Profile>>,
    /// Nominal layout of the target deck, so the UI can render slots.
    pub deck_info: DeckInfo,
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
}

/// Directory holding profile JSON files.
pub fn profiles_dir(config_dir: &Path) -> PathBuf {
    config_dir.join("profiles")
}

/// Path of a single profile file.
pub fn profile_path(config_dir: &Path, id: &str) -> PathBuf {
    profiles_dir(config_dir).join(format!("{id}.json"))
}

/// Load a profile by id, creating (and saving) a starter profile if none exists.
pub fn load_or_create_profile(config_dir: &Path, id: &str) -> Profile {
    let path = profile_path(config_dir, id);
    if let Ok(profile) = deck_core::store::load(&path) {
        return profile;
    }
    let profile = starter_profile(id);
    if let Err(e) = deck_core::store::save(&profile, &path) {
        eprintln!("[state] could not write starter profile: {e}");
    }
    profile
}

/// Persist a profile to `config_dir/profiles/<id>.json`.
pub fn save_profile(config_dir: &Path, profile: &Profile) -> Result<(), String> {
    let path = profile_path(config_dir, &profile.id);
    deck_core::store::save(profile, path).map_err(|e| e.to_string())
}

/// A friendly example profile so a first launch isn't a blank grid: a few common
/// gaming-adjacent bindings across the keys and encoders.
fn starter_profile(id: &str) -> Profile {
    let mut page = Page::empty("main", "Main");

    let keys: [(&str, &str, Action); 8] = [
        ("Discord", "#5865f2", Action::LaunchApp { path: "discord".into(), args: vec![] }),
        ("OBS", "#302e31", Action::LaunchApp { path: "obs".into(), args: vec![] }),
        ("Spotify", "#1db954", Action::LaunchApp { path: "spotify".into(), args: vec![] }),
        ("Browser", "#7ce0ff", Action::OpenUrl { url: "https://github.com".into() }),
        ("Screenshot", "#9d7cff", Action::SendHotkey { keys: vec!["printscreen".into()] }),
        ("Mute", "#ff6b6b", Action::SendHotkey { keys: vec!["ctrl".into(), "shift".into(), "m".into()] }),
        ("Clip", "#ffd166", Action::SendHotkey { keys: vec!["alt".into(), "f10".into()] }),
        ("Games", "#7cffa8", Action::LaunchApp { path: "steam".into(), args: vec![] }),
    ];
    for (i, (label, color, action)) in keys.into_iter().enumerate() {
        page.keys[i].label = label.to_string();
        page.keys[i].color = Some(color.to_string());
        page.keys[i].action = action;
    }

    // Encoders: volume, mic, brightness, scrub.
    page.encoders[0].label = "Volume".into();
    page.encoders[0].on_turn_cw = Action::SendHotkey { keys: vec!["volumeup".into()] };
    page.encoders[0].on_turn_ccw = Action::SendHotkey { keys: vec!["volumedown".into()] };
    page.encoders[0].on_press = Action::SendHotkey { keys: vec!["volumemute".into()] };

    page.encoders[1].label = "Mic".into();
    page.encoders[1].on_press = Action::SendHotkey { keys: vec!["f13".into()] };

    page.encoders[2].label = "Bright".into();
    page.encoders[2].on_turn_cw = Action::AdjustBrightness { delta: 10 };
    page.encoders[2].on_turn_ccw = Action::AdjustBrightness { delta: -10 };

    page.encoders[3].label = "Scene".into();
    page.encoders[3].on_press = Action::SendHotkey { keys: vec!["f14".into()] };

    Profile {
        id: id.to_string(),
        name: "Default".to_string(),
        pages: vec![page],
        activates_for: vec![],
    }
}
