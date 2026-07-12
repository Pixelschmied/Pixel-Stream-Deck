//! Tauri commands — the RPC surface the frontend calls via `invoke`.

use serde::Serialize;
use tauri::State;

use deck_core::{Action, DeckInfo, Profile};

use crate::device::DeviceCommand;
use crate::settings::Settings;
use crate::state::{self, AppState};

/// One-shot snapshot of everything the UI needs to render on load.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub product_name: String,
    pub version: String,
    /// True when compiled with real USB support (`hardware` feature).
    pub hardware_build: bool,
    pub deck_info: DeckInfo,
    pub profile: Profile,
    pub settings: Settings,
}

/// Return the current profile, settings and deck layout.
#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let settings = state.settings.lock().map_err(|_| "settings lock poisoned")?.clone();
    let profile = state.profile.lock().map_err(|_| "profile lock poisoned")?.clone();
    Ok(Snapshot {
        product_name: "Pixel Gaming Helper".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hardware_build: cfg!(feature = "hardware"),
        deck_info: state.deck_info.clone(),
        profile,
        settings,
    })
}

/// Replace the active profile, persist it and redraw the device.
#[tauri::command]
pub fn save_profile(state: State<'_, AppState>, mut profile: Profile) -> Result<(), String> {
    profile.normalize();
    state::save_profile(&state.config_dir, &profile)?;
    {
        let mut guard = state.profile.lock().map_err(|_| "profile lock poisoned")?;
        *guard = profile;
    }
    state.notify_device(DeviceCommand::Rerender);
    Ok(())
}

/// Persist settings and apply the ones that affect the device immediately.
#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let settings = settings.sanitized();
    settings.save(&state.config_dir).map_err(|e| e.to_string())?;
    let brightness = settings.brightness;
    {
        let mut guard = state.settings.lock().map_err(|_| "settings lock poisoned")?;
        *guard = settings;
    }
    state.notify_device(DeviceCommand::SetBrightness(brightness));
    Ok(())
}

/// Execute a single action now — used by the UI to test a binding without the
/// physical deck (e.g. clicking a key in the editor).
#[tauri::command]
pub fn run_action(action: Action) -> Result<(), String> {
    crate::actions::execute(&action)
}
