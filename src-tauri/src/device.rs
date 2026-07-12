//! The device worker: a background thread that owns the deck connection, keeps
//! it rendered with the active profile, and turns physical input into executed
//! actions.
//!
//! The backend is created *inside* the worker thread, so a non-`Send` hardware
//! handle never has to cross a thread boundary. It also keeps trying to (re)connect
//! to a real Stream Deck + while it only has the mock, so plugging in the device
//! or quitting the official Elgato software is picked up automatically.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use deck_core::{Controller, DeckBackend, MockBackend, Profile};

use crate::actions;

/// Whether a real device is driving the deck, reported to the UI.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DeviceStatus {
    /// Looking for a Stream Deck +.
    Searching,
    /// A physical device is connected and being driven.
    Connected { model: String, serial: String },
    /// No device found — running on the in-memory mock.
    Simulated,
}

/// Messages the app sends to the device worker.
pub enum DeviceCommand {
    /// The active profile changed on disk / in the UI — reload and redraw.
    Rerender,
    /// Set the deck brightness (0..=100).
    SetBrightness(u8),
    /// Stop the worker and release the device.
    Shutdown,
}

/// Try to connect to a real Stream Deck +. `None` when the `hardware` feature is
/// off or no device is available (e.g. unplugged or held by other software).
#[cfg(feature = "hardware")]
fn try_hardware() -> Option<Box<dyn DeckBackend>> {
    match deck_core::backend::hardware::HardwareBackend::connect() {
        Ok(dev) => {
            println!("[device] connected to Stream Deck +");
            Some(Box::new(dev))
        }
        Err(e) => {
            println!("[device] no Stream Deck + yet ({e})");
            None
        }
    }
}

#[cfg(not(feature = "hardware"))]
fn try_hardware() -> Option<Box<dyn DeckBackend>> {
    None
}

/// Publish a status change to the shared state and the UI.
fn report(app: &AppHandle, shared: &Arc<Mutex<DeviceStatus>>, status: DeviceStatus) {
    if let Ok(mut g) = shared.lock() {
        *g = status.clone();
    }
    let _ = app.emit("device-status", &status);
}

/// Build a controller around whichever backend is available, render the current
/// profile, and report whether a real device was found.
fn build_controller(
    profile: &Arc<Mutex<Profile>>,
    brightness: u8,
) -> (Controller<Box<dyn DeckBackend>>, bool) {
    let (backend, connected): (Box<dyn DeckBackend>, bool) = match try_hardware() {
        Some(dev) => (dev, true),
        None => (Box::new(MockBackend::new()), false),
    };
    let start = profile
        .lock()
        .map(|p| p.clone())
        .unwrap_or_else(|_| Profile::new("default", "Default"));
    let mut controller = Controller::new(backend, start);
    let _ = controller.set_brightness(brightness);
    if let Err(e) = controller.render() {
        eprintln!("[device] render failed: {e}");
    }
    (controller, connected)
}

fn status_of(controller: &Controller<Box<dyn DeckBackend>>, connected: bool) -> DeviceStatus {
    if connected {
        let info = controller.backend().info();
        DeviceStatus::Connected { model: info.model.clone(), serial: info.serial.clone() }
    } else {
        DeviceStatus::Simulated
    }
}

/// Run the worker loop until a [`DeviceCommand::Shutdown`] arrives or the channel
/// closes. Intended to be the body of a dedicated thread.
pub fn run(
    app: AppHandle,
    profile: Arc<Mutex<Profile>>,
    status: Arc<Mutex<DeviceStatus>>,
    initial_brightness: u8,
    rx: Receiver<DeviceCommand>,
) {
    let mut brightness = initial_brightness;
    let (mut controller, mut connected) = build_controller(&profile, brightness);
    report(&app, &status, status_of(&controller, connected));

    let retry_every = Duration::from_secs(3);
    let mut last_retry = Instant::now();

    loop {
        // Drain any pending commands first.
        loop {
            match rx.try_recv() {
                Ok(DeviceCommand::Shutdown) | Err(TryRecvError::Disconnected) => {
                    println!("[device] worker shutting down");
                    return;
                }
                Ok(DeviceCommand::Rerender) => {
                    if let Ok(p) = profile.lock() {
                        controller.set_profile(p.clone());
                    }
                    if let Err(e) = controller.render() {
                        eprintln!("[device] render failed: {e}");
                    }
                }
                Ok(DeviceCommand::SetBrightness(b)) => {
                    brightness = b;
                    let _ = controller.set_brightness(b);
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        // While running on the mock, periodically try to grab a real device.
        if !connected && last_retry.elapsed() >= retry_every {
            last_retry = Instant::now();
            if let Some(dev) = try_hardware() {
                let start = profile.lock().map(|p| p.clone()).unwrap_or_else(|_| Profile::new("default", "Default"));
                controller = Controller::new(dev, start);
                let _ = controller.set_brightness(brightness);
                let _ = controller.render();
                connected = true;
                report(&app, &status, status_of(&controller, connected));
            }
        }

        // Process device input.
        match controller.pump() {
            Ok(to_run) => {
                for action in to_run {
                    if let Err(e) = actions::execute(&action) {
                        eprintln!("[device] action error: {e}");
                    }
                }
            }
            Err(e) => {
                eprintln!("[device] input error: {e}");
                if connected {
                    // The device likely went away — drop back to the mock and
                    // let the reconnect loop pick it up again.
                    connected = false;
                    let (c, _) = build_controller(&profile, brightness);
                    controller = c;
                    report(&app, &status, DeviceStatus::Searching);
                    last_retry = Instant::now();
                }
            }
        }

        // Poll quickly when driving a device, slowly while just waiting for one.
        std::thread::sleep(if connected {
            Duration::from_millis(15)
        } else {
            Duration::from_millis(150)
        });
    }
}
