//! Hardware abstraction. Everything above this layer (the controller, the Tauri
//! app, the UI) talks to a [`DeckBackend`] and never to the USB device directly.
//!
//! Two implementations exist:
//!   * [`mock::MockBackend`] — pure in-memory, used for tests and headless dev.
//!   * `hardware::HardwareBackend` — the real Elgato driver, compiled only with
//!     the `hardware` feature (needs libusb/hidapi system libraries).

pub mod mock;

#[cfg(feature = "hardware")]
pub mod hardware;

use crate::model::{ENCODER_COUNT, KEY_COUNT};

/// Errors a backend can raise while talking to (or pretending to be) a device.
#[derive(Debug, thiserror::Error)]
pub enum DeckError {
    #[error("no Stream Deck device found")]
    NotFound,
    #[error("key index {0} out of range")]
    KeyOutOfRange(u8),
    #[error("image is {got} bytes but the target expects {expected}")]
    ImageSize { got: usize, expected: usize },
    #[error("device communication failed: {0}")]
    Io(String),
}

/// Result alias for backend operations.
pub type Result<T> = std::result::Result<T, DeckError>;

/// A raw RGBA8 image destined for a key or the touch strip. Backends are
/// responsible for any resizing/encoding the concrete hardware needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgba8Image {
    pub width: u16,
    pub height: u16,
    /// `width * height * 4` bytes, row-major RGBA.
    pub pixels: Vec<u8>,
}

impl Rgba8Image {
    /// Create a solid-colour image of the given size.
    pub fn solid(width: u16, height: u16, rgba: [u8; 4]) -> Self {
        let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);
        for _ in 0..(width as usize * height as usize) {
            pixels.extend_from_slice(&rgba);
        }
        Rgba8Image { width, height, pixels }
    }

    /// Whether the pixel buffer length matches the declared dimensions.
    pub fn is_valid(&self) -> bool {
        self.pixels.len() == self.width as usize * self.height as usize * 4
    }
}

/// Static description of a connected deck, so upper layers can lay out slots
/// without hardcoding a specific model.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DeckInfo {
    pub model: String,
    pub serial: String,
    pub key_count: u8,
    pub encoder_count: u8,
    /// Pixel size of a single key LCD, `(width, height)`.
    pub key_image_size: (u16, u16),
    /// Pixel size of the full touch strip, `(width, height)`.
    pub touchstrip_size: (u16, u16),
}

impl DeckInfo {
    /// The nominal Stream Deck + description.
    pub fn stream_deck_plus(serial: impl Into<String>) -> Self {
        DeckInfo {
            model: "Stream Deck +".into(),
            serial: serial.into(),
            key_count: KEY_COUNT as u8,
            encoder_count: ENCODER_COUNT as u8,
            key_image_size: (120, 120),
            touchstrip_size: (800, 100),
        }
    }
}

/// A physical input event coming from the deck.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeckEvent {
    KeyDown(u8),
    KeyUp(u8),
    EncoderDown(u8),
    EncoderUp(u8),
    /// Positive `ticks` = clockwise, negative = counter-clockwise.
    EncoderTurn { index: u8, ticks: i16 },
    /// A tap on the touch strip at the given pixel coordinate.
    TouchShort { x: u16, y: u16 },
    /// A swipe across the touch strip.
    TouchSwipe { from: (u16, u16), to: (u16, u16) },
}

/// Lets a boxed backend be used anywhere a `DeckBackend` is expected — e.g. the
/// app selects `Box::new(MockBackend)` or `Box::new(HardwareBackend)` at runtime.
impl<B: DeckBackend + ?Sized> DeckBackend for Box<B> {
    fn info(&self) -> &DeckInfo {
        (**self).info()
    }
    fn set_key_image(&mut self, index: u8, image: &Rgba8Image) -> Result<()> {
        (**self).set_key_image(index, image)
    }
    fn set_touchstrip_image(&mut self, image: &Rgba8Image) -> Result<()> {
        (**self).set_touchstrip_image(image)
    }
    fn set_brightness(&mut self, percent: u8) -> Result<()> {
        (**self).set_brightness(percent)
    }
    fn clear(&mut self) -> Result<()> {
        (**self).clear()
    }
    fn poll_events(&mut self) -> Result<Vec<DeckEvent>> {
        (**self).poll_events()
    }
}

/// The operations any deck (real or mock) must support.
pub trait DeckBackend {
    /// Static information about the connected device.
    fn info(&self) -> &DeckInfo;

    /// Draw an image on key `index` (0-based).
    fn set_key_image(&mut self, index: u8, image: &Rgba8Image) -> Result<()>;

    /// Draw an image across the whole touch strip.
    fn set_touchstrip_image(&mut self, image: &Rgba8Image) -> Result<()>;

    /// Set global brightness, 0..=100 percent.
    fn set_brightness(&mut self, percent: u8) -> Result<()>;

    /// Clear all keys and the touch strip.
    fn clear(&mut self) -> Result<()>;

    /// Non-blocking: drain any input events that have arrived since the last call.
    fn poll_events(&mut self) -> Result<Vec<DeckEvent>>;
}
