//! The device worker: a background thread that owns the deck connection, keeps
//! it rendered with the active profile, and turns physical input into executed
//! actions.
//!
//! The backend is created *inside* the worker thread, so a non-`Send` hardware
//! handle never has to cross a thread boundary. The rest of the app talks to the
//! worker only through an [`Arc<Mutex<Profile>>`] and a [`DeviceCommand`] channel.

use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use deck_core::{Controller, DeckBackend, MockBackend, Profile};

use crate::actions;

/// Messages the app sends to the device worker.
pub enum DeviceCommand {
    /// The active profile changed on disk / in the UI — reload and redraw.
    Rerender,
    /// Set the deck brightness (0..=100).
    SetBrightness(u8),
    /// Stop the worker and release the device.
    Shutdown,
}

/// Pick the best available backend. Uses the real Stream Deck + when the
/// `hardware` feature is on and a device is present, otherwise a mock so the app
/// is fully usable without hardware (and in headless environments).
fn build_backend() -> Box<dyn DeckBackend> {
    #[cfg(feature = "hardware")]
    {
        match deck_core::backend::hardware::HardwareBackend::connect() {
            Ok(dev) => {
                println!("[device] connected to Stream Deck +");
                return Box::new(dev);
            }
            Err(e) => eprintln!("[device] no Stream Deck + ({e}); falling back to mock"),
        }
    }
    Box::new(MockBackend::new())
}

/// Run the worker loop until a [`DeviceCommand::Shutdown`] arrives or the channel
/// closes. Intended to be the body of a dedicated thread.
pub fn run(profile: Arc<Mutex<Profile>>, initial_brightness: u8, rx: Receiver<DeviceCommand>) {
    let backend = build_backend();
    let start = profile.lock().map(|p| p.clone()).unwrap_or_else(|_| Profile::new("default", "Default"));
    let mut controller = Controller::new(backend, start);
    let _ = controller.set_brightness(initial_brightness);
    if let Err(e) = controller.render() {
        eprintln!("[device] initial render failed: {e}");
    }

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
                    let _ = controller.set_brightness(b);
                }
                Err(TryRecvError::Empty) => break,
            }
        }

        // Then process device input.
        match controller.pump() {
            Ok(to_run) => {
                for action in to_run {
                    if let Err(e) = actions::execute(&action) {
                        eprintln!("[device] action error: {e}");
                    }
                }
            }
            Err(e) => eprintln!("[device] input error: {e}"),
        }

        std::thread::sleep(Duration::from_millis(15));
    }
}
