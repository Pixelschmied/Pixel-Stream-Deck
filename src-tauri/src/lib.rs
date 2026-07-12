//! Pixel Gaming Helper — Tauri application entry point.
//!
//! Wires together the config/profile state, the system-tray icon, the window
//! behaviour (close-to-tray) and the background device worker that drives the
//! Stream Deck +.

mod actions;
mod commands;
mod device;
mod settings;
mod state;

use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

use deck_core::DeckInfo;

use crate::device::DeviceCommand;
use crate::settings::Settings;
use crate::state::AppState;

/// Show and focus the main window, creating focus even if it was hidden to tray.
fn reveal_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

/// Build the tray icon with a small Show/Quit menu.
fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "Öffnen", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Beenden", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &quit])?;

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("app bundle always ships a default icon");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .tooltip("Pixel Gaming Helper")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => reveal_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            // Left-click the tray icon to bring the window back.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Run the application. Called from `main`.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let handle = app.handle();

            // Resolve (and create) the per-user config directory.
            let config_dir = handle
                .path()
                .app_config_dir()
                .expect("a platform config directory must exist");
            std::fs::create_dir_all(&config_dir).ok();

            // Load settings + the active profile (seeding defaults on first run).
            let settings = Settings::load(&config_dir).sanitized();
            let profile =
                state::load_or_create_profile(&config_dir, &settings.active_profile_id);
            let profile = Arc::new(Mutex::new(profile));

            // Spawn the device worker and keep the command channel in state.
            let (tx, rx) = mpsc::channel::<DeviceCommand>();
            let worker_profile = profile.clone();
            let initial_brightness = settings.brightness;
            thread::Builder::new()
                .name("deck-device".into())
                .spawn(move || device::run(worker_profile, initial_brightness, rx))
                .expect("failed to spawn device worker");

            let start_minimized = settings.start_minimized;
            app.manage(AppState {
                config_dir,
                settings: Mutex::new(settings),
                profile,
                deck_info: DeckInfo::stream_deck_plus("—"),
                device_tx: Mutex::new(Some(tx)),
            });

            setup_tray(app)?;

            if start_minimized {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let minimize = window
                    .app_handle()
                    .try_state::<AppState>()
                    .and_then(|s| s.settings.lock().ok().map(|g| g.minimize_to_tray_on_close))
                    .unwrap_or(true);
                if minimize {
                    // Hide to tray instead of quitting.
                    let _ = window.hide();
                    api.prevent_close();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_snapshot,
            commands::save_profile,
            commands::save_settings,
            commands::run_action,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Pixel Gaming Helper")
        .run(|handle, event| {
            // Release the deck gracefully when the app is quitting.
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = handle.try_state::<AppState>() {
                    state.notify_device(DeviceCommand::Shutdown);
                }
            }
        });
}
