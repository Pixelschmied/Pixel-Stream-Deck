//! An in-memory [`DeckBackend`] used for tests, UI development and the headless
//! cloud/CI environment where no USB device is present.
//!
//! It records everything written to it (last image per key, brightness, …) so
//! tests can assert on rendered output, and it lets tests inject input events
//! via [`MockBackend::push_event`].

use std::collections::VecDeque;

use super::{DeckBackend, DeckError, DeckEvent, DeckInfo, Result, Rgba8Image};

/// A fake Stream Deck + that lives entirely in memory.
#[derive(Debug)]
pub struct MockBackend {
    info: DeckInfo,
    key_images: Vec<Option<Rgba8Image>>,
    touchstrip: Option<Rgba8Image>,
    brightness: u8,
    pending: VecDeque<DeckEvent>,
    /// Total number of key writes, handy for tests.
    pub key_writes: u64,
}

impl MockBackend {
    /// Create a mock that presents itself as a Stream Deck +.
    pub fn new() -> Self {
        let info = DeckInfo::stream_deck_plus("MOCK-0001");
        let key_images = vec![None; info.key_count as usize];
        MockBackend {
            info,
            key_images,
            touchstrip: None,
            brightness: 100,
            pending: VecDeque::new(),
            key_writes: 0,
        }
    }

    /// Queue an input event to be returned by the next [`poll_events`] call.
    ///
    /// [`poll_events`]: DeckBackend::poll_events
    pub fn push_event(&mut self, ev: DeckEvent) {
        self.pending.push_back(ev);
    }

    /// The last image drawn to `index`, if any.
    pub fn key_image(&self, index: u8) -> Option<&Rgba8Image> {
        self.key_images.get(index as usize).and_then(|s| s.as_ref())
    }

    /// The last image drawn to the touch strip, if any.
    pub fn touchstrip_image(&self) -> Option<&Rgba8Image> {
        self.touchstrip.as_ref()
    }

    /// Current brightness (0..=100).
    pub fn brightness(&self) -> u8 {
        self.brightness
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl DeckBackend for MockBackend {
    fn info(&self) -> &DeckInfo {
        &self.info
    }

    fn set_key_image(&mut self, index: u8, image: &Rgba8Image) -> Result<()> {
        let slot = self
            .key_images
            .get_mut(index as usize)
            .ok_or(DeckError::KeyOutOfRange(index))?;
        if !image.is_valid() {
            return Err(DeckError::ImageSize {
                got: image.pixels.len(),
                expected: image.width as usize * image.height as usize * 4,
            });
        }
        *slot = Some(image.clone());
        self.key_writes += 1;
        Ok(())
    }

    fn set_touchstrip_image(&mut self, image: &Rgba8Image) -> Result<()> {
        if !image.is_valid() {
            return Err(DeckError::ImageSize {
                got: image.pixels.len(),
                expected: image.width as usize * image.height as usize * 4,
            });
        }
        self.touchstrip = Some(image.clone());
        Ok(())
    }

    fn set_brightness(&mut self, percent: u8) -> Result<()> {
        self.brightness = percent.min(100);
        Ok(())
    }

    fn clear(&mut self) -> Result<()> {
        for slot in &mut self.key_images {
            *slot = None;
        }
        self.touchstrip = None;
        Ok(())
    }

    fn poll_events(&mut self) -> Result<Vec<DeckEvent>> {
        Ok(self.pending.drain(..).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_key_image() {
        let mut deck = MockBackend::new();
        let (w, h) = deck.info().key_image_size;
        let img = Rgba8Image::solid(w, h, [255, 0, 0, 255]);
        deck.set_key_image(0, &img).unwrap();
        assert_eq!(deck.key_image(0), Some(&img));
        assert_eq!(deck.key_writes, 1);
    }

    #[test]
    fn rejects_out_of_range_key() {
        let mut deck = MockBackend::new();
        let img = Rgba8Image::solid(1, 1, [0, 0, 0, 0]);
        assert!(matches!(
            deck.set_key_image(99, &img),
            Err(DeckError::KeyOutOfRange(99))
        ));
    }

    #[test]
    fn rejects_mismatched_image() {
        let mut deck = MockBackend::new();
        let bad = Rgba8Image { width: 10, height: 10, pixels: vec![0; 8] };
        assert!(matches!(
            deck.set_key_image(0, &bad),
            Err(DeckError::ImageSize { .. })
        ));
    }

    #[test]
    fn events_drain_once() {
        let mut deck = MockBackend::new();
        deck.push_event(DeckEvent::KeyDown(2));
        deck.push_event(DeckEvent::EncoderTurn { index: 1, ticks: -3 });
        let first = deck.poll_events().unwrap();
        assert_eq!(first.len(), 2);
        assert!(deck.poll_events().unwrap().is_empty());
    }

    #[test]
    fn brightness_is_clamped() {
        let mut deck = MockBackend::new();
        deck.set_brightness(200).unwrap();
        assert_eq!(deck.brightness(), 100);
    }
}
