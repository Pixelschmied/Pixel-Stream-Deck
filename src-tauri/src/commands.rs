//! Tauri commands — the RPC surface the frontend calls via `invoke`.

use serde::Serialize;
use tauri::{AppHandle, State};

use deck_core::{Action, DeckInfo, Profile};

use crate::device::DeviceCommand;
use crate::settings::Settings;
use crate::state::{self, AppState};

/// Compact profile entry for the switcher list.
#[derive(Serialize)]
pub struct ProfileSummary {
    pub id: String,
    pub name: String,
    /// True when this profile auto-activates for some application.
    pub has_rules: bool,
}

/// One-shot snapshot of everything the UI needs to render on load.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub product_name: String,
    pub version: String,
    pub hardware_build: bool,
    pub deck_info: DeckInfo,
    pub profiles: Vec<ProfileSummary>,
    pub active_profile_id: String,
    pub profile: Profile,
    pub settings: Settings,
}

fn summaries(state: &AppState) -> Vec<ProfileSummary> {
    state
        .profiles
        .lock()
        .map(|list| {
            list.iter()
                .map(|p| ProfileSummary {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    has_rules: !p.activates_for.is_empty(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Return the current profile, all profiles, settings and deck layout.
#[tauri::command]
pub fn get_snapshot(state: State<'_, AppState>) -> Result<Snapshot, String> {
    let settings = state.settings.lock().map_err(|_| "settings lock poisoned")?.clone();
    let profile = state.profile.lock().map_err(|_| "profile lock poisoned")?.clone();
    let active_profile_id = state.active_id.lock().map_err(|_| "active id lock poisoned")?.clone();
    Ok(Snapshot {
        product_name: "Pixel Gaming Helper".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hardware_build: cfg!(feature = "hardware"),
        deck_info: state.deck_info.clone(),
        profiles: summaries(&state),
        active_profile_id,
        profile,
        settings,
    })
}

/// Switch the active profile and return its contents.
#[tauri::command]
pub fn set_active_profile(app: AppHandle, id: String) -> Result<Profile, String> {
    crate::activate_profile(&app, &id).ok_or_else(|| format!("unknown profile '{id}'"))
}

/// Persist an edited profile. If it is the active one, redraw the device.
#[tauri::command]
pub fn save_profile(state: State<'_, AppState>, mut profile: Profile) -> Result<(), String> {
    profile.normalize();
    state::save_profile(&state.config_dir, &profile)?;

    if let Ok(mut list) = state.profiles.lock() {
        match list.iter_mut().find(|p| p.id == profile.id) {
            Some(existing) => *existing = profile.clone(),
            None => list.push(profile.clone()),
        }
    }

    let active = state.active_id.lock().map(|g| g.clone()).unwrap_or_default();
    if active == profile.id {
        if let Ok(mut g) = state.profile.lock() {
            *g = profile.clone();
        }
        state.notify_device(DeviceCommand::Rerender);
    }
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

/// Execute a single action now — used by the UI to test a binding.
#[tauri::command]
pub fn run_action(action: Action) -> Result<(), String> {
    crate::actions::execute(&action)
}
