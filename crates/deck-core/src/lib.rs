//! # deck-core
//!
//! Hardware-agnostic core of **Pixel Stream Deck** — an open replacement for the
//! Elgato Stream Deck software, targeting the Stream Deck + (8 LCD keys, 4
//! encoders/dials and a touch strip).
//!
//! This crate contains everything that does **not** depend on the OS, the USB
//! stack or the UI toolkit, so it compiles and is fully unit-tested anywhere
//! (including headless CI):
//!
//! * [`model`] — the profile/page/action configuration model (serde).
//! * [`backend`] — the [`DeckBackend`] abstraction plus a
//!   [`MockBackend`]; the real Elgato driver lives behind the `hardware`
//!   feature.
//! * [`controller`] — maps physical input events to actions and renders pages.
//!
//! Side-effecting concerns (spawning processes, sending hotkeys, watching the
//! foreground window) live in the `src-tauri` application crate that consumes
//! this library.
//!
//! [`DeckBackend`]: backend::DeckBackend
//! [`MockBackend`]: backend::mock::MockBackend

pub mod backend;
#[cfg(feature = "render")]
pub mod brand_icons;
pub mod controller;
pub mod model;
#[cfg(feature = "render")]
pub mod render;
pub mod store;

pub use backend::mock::MockBackend;
pub use backend::{DeckBackend, DeckError, DeckEvent, DeckInfo, Rgba8Image};
pub use controller::Controller;
pub use model::{
    Action, AppMatch, EncoderConfig, KeyConfig, Page, Profile, ENCODER_COUNT, KEY_COUNT,
};
