//! The context engine: watches the foreground application and, when enabled,
//! auto-activates the profile whose rules match it (e.g. focus Spotify -> the
//! Spotify profile lights up the deck).

use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::state::AppState;

/// Poll interval for the foreground-window check.
const POLL: Duration = Duration::from_millis(900);

/// Spawn the watcher on its own thread. Cheap to run; it no-ops while the
/// `context_switching_enabled` setting is off.
pub fn spawn(app: AppHandle) {
    std::thread::Builder::new()
        .name("context-watcher".into())
        .spawn(move || run(app))
        .ok();
}

fn run(app: AppHandle) {
    // Remember the last foreground signature so we only react to changes.
    let mut last_signature = String::new();

    loop {
        std::thread::sleep(POLL);

        let Some(state) = app.try_state::<AppState>() else { continue };

        let enabled = state
            .settings
            .lock()
            .ok()
            .map(|g| g.context_switching_enabled)
            .unwrap_or(false);
        if !enabled {
            last_signature.clear();
            continue;
        }

        let Ok(win) = active_win_pos_rs::get_active_window() else { continue };

        let process = win
            .process_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let signature = format!("{}|{}|{}", process, win.app_name, win.title);
        if signature == last_signature {
            continue;
        }
        last_signature = signature;

        if let Some(id) = state.match_profile_id(&process, &win.app_name, &win.title) {
            let current = state
                .active_id
                .lock()
                .ok()
                .map(|g| g.clone())
                .unwrap_or_default();
            if id != current {
                println!("[context] {process} -> activating profile '{id}'");
                crate::activate_profile(&app, &id);
            }
        }
    }
}
