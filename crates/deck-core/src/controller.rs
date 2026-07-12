//! The controller ties a [`DeckBackend`] together with a [`Profile`]. It owns
//! the "what is currently shown" state (active profile + page) and translates
//! physical [`DeckEvent`]s into the [`Action`] that should be executed.
//!
//! Executing the resulting action (launching an app, sending a hotkey, …) is
//! deliberately *not* this crate's job — that is platform-specific and lives in
//! the Tauri layer. The controller only decides *what* should happen, which
//! keeps this logic fully unit-testable without any OS side effects.

use crate::backend::{DeckBackend, DeckEvent, Result, Rgba8Image};
use crate::model::{Action, Profile};

use crate::model::{EncoderConfig, Page};

/// A colour used to render a key's background when it has no icon.
fn parse_hex_color(hex: &Option<String>) -> [u8; 4] {
    let default = [30, 30, 46, 255]; // a calm dark slate
    let Some(hex) = hex else { return default };
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        if let Ok(v) = u32::from_str_radix(hex, 16) {
            return [(v >> 16) as u8, (v >> 8) as u8, v as u8, 255];
        }
    }
    default
}

/// Accent colours cycled across encoders/keys, matching the app icon.
const PALETTE: [[u8; 4]; 6] = [
    [0x7c, 0xe0, 0xff, 255], // cyan
    [0xff, 0x6b, 0xd6, 255], // pink
    [0x9d, 0x7c, 0xff, 255], // violet
    [0x7c, 0xff, 0xa8, 255], // green
    [0xff, 0xd1, 0x66, 255], // amber
    [0xff, 0x8b, 0x6b, 255], // coral
];

/// Render the whole touch strip at full resolution as one image, divided into a
/// coloured band per encoder. Bound encoders glow in an accent colour; unbound
/// ones stay dim. This deliberately uses the entire strip so the hardware's LCD
/// is fully utilised rather than left blank.
fn render_touchstrip(page: &Page, width: u16, height: u16, encoder_count: u8) -> Rgba8Image {
    let count = encoder_count.max(1) as usize;
    let w = width as usize;
    let h = height as usize;
    let mut pixels = vec![0u8; w * h * 4];
    let seg_w = w / count;
    for x in 0..w {
        let seg = (x / seg_w.max(1)).min(count - 1);
        let bound = page
            .encoders
            .get(seg)
            .map(EncoderConfig::has_binding)
            .unwrap_or(false);
        let accent = PALETTE[seg % PALETTE.len()];
        // A subtle top-to-bottom fade; dim the band if the encoder is unbound.
        for y in 0..h {
            let fade = 1.0 - (y as f32 / h as f32) * 0.35;
            let dim = if bound { 1.0 } else { 0.18 };
            let i = (y * w + x) * 4;
            pixels[i] = (accent[0] as f32 * fade * dim) as u8;
            pixels[i + 1] = (accent[1] as f32 * fade * dim) as u8;
            pixels[i + 2] = (accent[2] as f32 * fade * dim) as u8;
            pixels[i + 3] = 255;
        }
        // Thin dark separator between bands.
        if seg_w > 0 && x % seg_w == 0 && x != 0 {
            for y in 0..h {
                let i = (y * w + x) * 4;
                pixels[i] = 10;
                pixels[i + 1] = 10;
                pixels[i + 2] = 16;
            }
        }
    }
    Rgba8Image { width, height, pixels }
}

/// Drives a deck from a profile.
pub struct Controller<B: DeckBackend> {
    backend: B,
    profile: Profile,
    active_page: String,
    brightness: u8,
}

impl<B: DeckBackend> Controller<B> {
    /// Create a controller for `backend` showing `profile`'s first page.
    pub fn new(backend: B, mut profile: Profile) -> Self {
        profile.normalize();
        let active_page = profile
            .pages
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
        Controller { backend, profile, active_page, brightness: 100 }
    }

    /// The id of the currently displayed page.
    pub fn active_page(&self) -> &str {
        &self.active_page
    }

    /// Borrow the underlying backend (useful in tests).
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Current brightness (0..=100).
    pub fn brightness(&self) -> u8 {
        self.brightness
    }

    /// Replace the profile shown by this controller (e.g. after the user edited
    /// it in the UI, or a different profile was auto-activated). Resets to the
    /// first page. Call [`render`](Self::render) afterwards to update the device.
    pub fn set_profile(&mut self, mut profile: Profile) {
        profile.normalize();
        self.active_page = profile
            .pages
            .first()
            .map(|p| p.id.clone())
            .unwrap_or_default();
        self.profile = profile;
    }

    /// Set absolute brightness and push it to the device.
    pub fn set_brightness(&mut self, percent: u8) -> Result<()> {
        let percent = percent.min(100);
        self.backend.set_brightness(percent)?;
        self.brightness = percent;
        Ok(())
    }

    /// Apply a relative brightness change, returning the new value.
    pub fn adjust_brightness(&mut self, delta: i16) -> Result<u8> {
        let next = (self.brightness as i16 + delta).clamp(0, 100) as u8;
        self.set_brightness(next)?;
        Ok(next)
    }

    /// Switch to another page and re-render. No-op if the id is unknown.
    pub fn switch_page(&mut self, page_id: &str) -> Result<bool> {
        if self.profile.page(page_id).is_some() {
            self.active_page = page_id.to_string();
            self.render()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Push the current page's visuals onto the backend: every key plus the
    /// full-resolution touch strip.
    pub fn render(&mut self) -> Result<()> {
        let (kw, kh) = self.backend.info().key_image_size;
        let (sw, sh) = self.backend.info().touchstrip_size;
        let encoder_count = self.backend.info().encoder_count;

        // Compute everything that reads `self.profile` up front, so we don't
        // hold an immutable borrow of it while mutably borrowing `self.backend`.
        let (key_colors, strip) = {
            let Some(page) = self.profile.page(&self.active_page) else {
                return self.backend.clear();
            };
            let key_colors: Vec<[u8; 4]> =
                page.keys.iter().map(|k| parse_hex_color(&k.color)).collect();
            let strip = render_touchstrip(page, sw, sh, encoder_count);
            (key_colors, strip)
        };

        for (i, color) in key_colors.iter().enumerate() {
            let img = Rgba8Image::solid(kw, kh, *color);
            self.backend.set_key_image(i as u8, &img)?;
        }
        self.backend.set_touchstrip_image(&strip)?;
        Ok(())
    }

    /// Poll the device for input and process it in one shot. Page switches and
    /// brightness changes are applied to the device internally; the remaining
    /// OS-level actions (launch app, run command, open URL, hotkey) are returned
    /// for the host to execute.
    pub fn pump(&mut self) -> Result<Vec<Action>> {
        let events = self.backend.poll_events()?;
        let mut actions = Vec::new();
        for event in events {
            if let Some(action) = self.handle_event(event)? {
                match action {
                    Action::AdjustBrightness { delta } => {
                        self.adjust_brightness(delta)?;
                    }
                    other => actions.push(other),
                }
            }
        }
        Ok(actions)
    }

    /// Map a physical event to the action that should be executed, updating
    /// internal state (e.g. page switches) as a side effect.
    ///
    /// Returns `Ok(Some(action))` when the caller should execute `action`.
    pub fn handle_event(&mut self, event: DeckEvent) -> Result<Option<Action>> {
        let action = match event {
            // Only act on release for keys/dials — matches typical deck feel.
            DeckEvent::KeyUp(i) => self.key_action(i),
            DeckEvent::EncoderUp(i) => self.encoder_press_action(i),
            DeckEvent::EncoderTurn { index, ticks } => self.encoder_turn_action(index, ticks),
            DeckEvent::TouchShort { x, .. } => self.touch_action(x),
            // Down/swipe events carry no bound action in this first version.
            DeckEvent::KeyDown(_)
            | DeckEvent::EncoderDown(_)
            | DeckEvent::TouchSwipe { .. } => None,
        };

        // Page switches are handled internally rather than handed to the caller.
        if let Some(Action::SwitchPage { page_id }) = &action {
            let page_id = page_id.clone();
            self.switch_page(&page_id)?;
            return Ok(None);
        }

        Ok(action.filter(Action::is_bound))
    }

    fn page(&self) -> Option<&crate::model::Page> {
        self.profile.page(&self.active_page)
    }

    fn key_action(&self, index: u8) -> Option<Action> {
        self.page()?.keys.get(index as usize).map(|k| k.action.clone())
    }

    fn encoder_press_action(&self, index: u8) -> Option<Action> {
        self.page()?
            .encoders
            .get(index as usize)
            .map(|e| e.on_press.clone())
    }

    fn encoder_turn_action(&self, index: u8, ticks: i16) -> Option<Action> {
        let enc = self.page()?.encoders.get(index as usize)?;
        Some(if ticks >= 0 {
            enc.on_turn_cw.clone()
        } else {
            enc.on_turn_ccw.clone()
        })
    }

    /// Map a touch-strip x coordinate to the encoder segment beneath it.
    fn touch_action(&self, x: u16) -> Option<Action> {
        let (strip_w, _) = self.backend.info().touchstrip_size;
        let count = self.backend.info().encoder_count.max(1) as u16;
        let seg = (x / (strip_w / count)).min(count - 1);
        self.page()?
            .encoders
            .get(seg as usize)
            .map(|e| e.on_touch.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::mock::MockBackend;
    use crate::model::Page;

    fn profile_with_bindings() -> Profile {
        let mut page = Page::empty("main", "Main");
        page.keys[0].action = Action::RunCommand { command: "echo hi".into() };
        page.keys[1].action = Action::SwitchPage { page_id: "second".into() };
        page.encoders[2].on_press = Action::OpenUrl { url: "https://example.com".into() };
        page.encoders[2].on_turn_cw = Action::AdjustBrightness { delta: 10 };
        page.encoders[2].on_turn_ccw = Action::AdjustBrightness { delta: -10 };
        page.encoders[0].on_touch = Action::RunCommand { command: "touched".into() };

        let second = Page::empty("second", "Second");
        Profile {
            id: "p".into(),
            name: "P".into(),
            pages: vec![page, second],
            activates_for: vec![],
        }
    }

    #[test]
    fn render_writes_every_key() {
        let mut c = Controller::new(MockBackend::new(), Profile::new("x", "X"));
        c.render().unwrap();
        assert_eq!(c.backend().key_writes, 8);
    }

    #[test]
    fn key_release_yields_action() {
        let mut c = Controller::new(MockBackend::new(), profile_with_bindings());
        let action = c.handle_event(DeckEvent::KeyUp(0)).unwrap();
        assert_eq!(action, Some(Action::RunCommand { command: "echo hi".into() }));
    }

    #[test]
    fn key_press_down_does_nothing() {
        let mut c = Controller::new(MockBackend::new(), profile_with_bindings());
        assert_eq!(c.handle_event(DeckEvent::KeyDown(0)).unwrap(), None);
    }

    #[test]
    fn unbound_key_yields_none() {
        let mut c = Controller::new(MockBackend::new(), profile_with_bindings());
        assert_eq!(c.handle_event(DeckEvent::KeyUp(5)).unwrap(), None);
    }

    #[test]
    fn switch_page_is_handled_internally() {
        let mut c = Controller::new(MockBackend::new(), profile_with_bindings());
        assert_eq!(c.active_page(), "main");
        let action = c.handle_event(DeckEvent::KeyUp(1)).unwrap();
        assert_eq!(action, None); // consumed internally
        assert_eq!(c.active_page(), "second");
    }

    #[test]
    fn encoder_turn_direction_picks_action() {
        let mut c = Controller::new(MockBackend::new(), profile_with_bindings());
        assert_eq!(
            c.handle_event(DeckEvent::EncoderTurn { index: 2, ticks: 4 }).unwrap(),
            Some(Action::AdjustBrightness { delta: 10 })
        );
        assert_eq!(
            c.handle_event(DeckEvent::EncoderTurn { index: 2, ticks: -1 }).unwrap(),
            Some(Action::AdjustBrightness { delta: -10 })
        );
    }

    #[test]
    fn touch_maps_to_correct_segment() {
        let mut c = Controller::new(MockBackend::new(), profile_with_bindings());
        // Strip is 800px wide, 4 encoders -> 200px each. x=50 -> segment 0.
        let action = c.handle_event(DeckEvent::TouchShort { x: 50, y: 20 }).unwrap();
        assert_eq!(action, Some(Action::RunCommand { command: "touched".into() }));
    }

    #[test]
    fn adjust_brightness_clamps_and_reaches_backend() {
        let mut c = Controller::new(MockBackend::new(), Profile::new("x", "X"));
        assert_eq!(c.adjust_brightness(-30).unwrap(), 70);
        assert_eq!(c.backend().brightness(), 70);
        assert_eq!(c.adjust_brightness(-100).unwrap(), 0);
        assert_eq!(c.brightness(), 0);
    }

    #[test]
    fn pump_dispatches_os_actions_and_applies_side_effects() {
        // Prime the mock with a burst of input: a key release (RunCommand), an
        // encoder turned counter-clockwise (brightness -10, applied internally),
        // and a key that switches page (consumed internally).
        let mut backend = MockBackend::new();
        backend.push_event(DeckEvent::KeyUp(0));
        backend.push_event(DeckEvent::EncoderTurn { index: 2, ticks: -2 });
        backend.push_event(DeckEvent::KeyUp(1));
        let mut c = Controller::new(backend, profile_with_bindings());

        let actions = c.pump().unwrap();
        assert_eq!(actions, vec![Action::RunCommand { command: "echo hi".into() }]);
        assert_eq!(c.brightness(), 90); // 100 - 10
        assert_eq!(c.active_page(), "second"); // key1 switched page
    }

    #[test]
    fn set_profile_resets_to_first_page() {
        let mut c = Controller::new(MockBackend::new(), profile_with_bindings());
        c.switch_page("second").unwrap();
        assert_eq!(c.active_page(), "second");
        c.set_profile(Profile::new("new", "New"));
        assert_eq!(c.active_page(), "main");
    }

    #[test]
    fn touch_far_right_clamps_to_last_segment() {
        let mut c = Controller::new(MockBackend::new(), Profile::new("x", "X"));
        // Should not panic even at the extreme right edge.
        let _ = c.handle_event(DeckEvent::TouchShort { x: 799, y: 0 }).unwrap();
    }
}
