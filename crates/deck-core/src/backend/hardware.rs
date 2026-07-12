//! The real Elgato Stream Deck + backend, built on the [`elgato-streamdeck`]
//! crate (USB HID). Compiled only with the `hardware` feature because it pulls
//! in `hidapi`, which needs system libraries (libudev/libusb) that aren't
//! present in the headless cloud/CI environment.
//!
//! This module talks to the device; it is verified on real hardware rather than
//! in unit tests. The pure logic it feeds (event -> action mapping, rendering)
//! is covered by the mock-backed tests in [`crate::controller`].
//!
//! [`elgato-streamdeck`]: https://crates.io/crates/elgato-streamdeck

use std::time::Duration;

use elgato_streamdeck::info::Kind;
use elgato_streamdeck::images::ImageRect;
use elgato_streamdeck::{list_devices, new_hidapi, StreamDeck, StreamDeckInput};
use image::{DynamicImage, RgbaImage};

use super::{DeckBackend, DeckError, DeckEvent, DeckInfo, Result, Rgba8Image};

/// A live connection to a physical Stream Deck +.
pub struct HardwareBackend {
    deck: StreamDeck,
    info: DeckInfo,
    /// Last known pressed-state of each key, to derive down/up edges.
    last_keys: Vec<bool>,
    /// Last known pressed-state of each encoder.
    last_encoders: Vec<bool>,
}

impl HardwareBackend {
    /// Find and connect to the first Stream Deck + on the system.
    pub fn connect() -> Result<Self> {
        let hid = new_hidapi().map_err(|e| DeckError::Io(e.to_string()))?;
        let (kind, serial) = list_devices(&hid)
            .into_iter()
            .find(|(kind, _)| matches!(kind, Kind::Plus))
            .ok_or(DeckError::NotFound)?;
        let deck = StreamDeck::connect(&hid, kind, &serial)
            .map_err(|e| DeckError::Io(e.to_string()))?;
        let info = DeckInfo::stream_deck_plus(&serial);
        Ok(HardwareBackend {
            deck,
            last_keys: vec![false; info.key_count as usize],
            last_encoders: vec![false; info.encoder_count as usize],
            info,
        })
    }

    fn to_dynamic(image: &Rgba8Image) -> Result<DynamicImage> {
        let buf = RgbaImage::from_raw(
            image.width as u32,
            image.height as u32,
            image.pixels.clone(),
        )
        .ok_or(DeckError::ImageSize {
            got: image.pixels.len(),
            expected: image.width as usize * image.height as usize * 4,
        })?;
        Ok(DynamicImage::ImageRgba8(buf))
    }
}

impl DeckBackend for HardwareBackend {
    fn info(&self) -> &DeckInfo {
        &self.info
    }

    fn set_key_image(&mut self, index: u8, image: &Rgba8Image) -> Result<()> {
        let dynimg = Self::to_dynamic(image)?;
        self.deck
            .set_button_image(index, dynimg)
            .map_err(|e| DeckError::Io(e.to_string()))
    }

    fn set_touchstrip_image(&mut self, image: &Rgba8Image) -> Result<()> {
        let dynimg = Self::to_dynamic(image)?;
        let rect = ImageRect::from_image(dynimg).map_err(|e| DeckError::Io(e.to_string()))?;
        self.deck
            .write_lcd(0, 0, &rect)
            .map_err(|e| DeckError::Io(e.to_string()))
    }

    fn set_brightness(&mut self, percent: u8) -> Result<()> {
        self.deck
            .set_brightness(percent.min(100))
            .map_err(|e| DeckError::Io(e.to_string()))
    }

    fn clear(&mut self) -> Result<()> {
        self.deck
            .clear_all_button_images()
            .map_err(|e| DeckError::Io(e.to_string()))
    }

    fn poll_events(&mut self) -> Result<Vec<DeckEvent>> {
        let mut out = Vec::new();
        // Drain everything currently queued without blocking, then stop on the
        // first NoData (timeout of zero = non-blocking read).
        loop {
            let input = self
                .deck
                .read_input(Some(Duration::from_millis(0)))
                .map_err(|e| DeckError::Io(e.to_string()))?;
            match input {
                StreamDeckInput::NoData => break,
                StreamDeckInput::ButtonStateChange(states) => {
                    for (i, &pressed) in states.iter().enumerate() {
                        let was = self.last_keys.get(i).copied().unwrap_or(false);
                        match (was, pressed) {
                            (false, true) => out.push(DeckEvent::KeyDown(i as u8)),
                            (true, false) => out.push(DeckEvent::KeyUp(i as u8)),
                            _ => {}
                        }
                    }
                    self.last_keys = states;
                }
                StreamDeckInput::EncoderStateChange(states) => {
                    for (i, &pressed) in states.iter().enumerate() {
                        let was = self.last_encoders.get(i).copied().unwrap_or(false);
                        match (was, pressed) {
                            (false, true) => out.push(DeckEvent::EncoderDown(i as u8)),
                            (true, false) => out.push(DeckEvent::EncoderUp(i as u8)),
                            _ => {}
                        }
                    }
                    self.last_encoders = states;
                }
                StreamDeckInput::EncoderTwist(ticks) => {
                    for (i, &t) in ticks.iter().enumerate() {
                        if t != 0 {
                            out.push(DeckEvent::EncoderTurn {
                                index: i as u8,
                                ticks: t as i16,
                            });
                        }
                    }
                }
                StreamDeckInput::TouchScreenPress(x, y)
                | StreamDeckInput::TouchScreenLongPress(x, y) => {
                    out.push(DeckEvent::TouchShort { x, y });
                }
                StreamDeckInput::TouchScreenSwipe(from, to) => {
                    out.push(DeckEvent::TouchSwipe { from, to });
                }
            }
        }
        Ok(out)
    }
}
