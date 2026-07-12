//! Renders a profile to a single PNG that looks like the physical deck
//! (8 keys + touch strip), for eyeballing the rich renderer without hardware.
//!
//! Run with:  cargo run -p deck-core --features render --example preview
//! Output:    target/preview/deck.png

use deck_core::model::{Action, EncoderConfig, KeyConfig, Page};
use deck_core::render::{render_key, render_strip};
use resvg::tiny_skia::{Pixmap, PixmapPaint, Transform};

fn key(label: &str, color: &str) -> KeyConfig {
    KeyConfig { label: label.into(), icon: None, color: Some(color.into()), action: Action::None }
}

fn bound_encoder(label: &str) -> EncoderConfig {
    EncoderConfig {
        label: label.into(),
        on_turn_cw: Action::AdjustBrightness { delta: 10 },
        ..Default::default()
    }
}

fn main() {
    // A demo page: the five app keys with brand logos + three extras.
    let mut page = Page::empty("main", "Main");
    let keys = [
        ("Spotify", "#1db954", Some("spotify")),
        ("Steam", "#1b2838", Some("steam")),
        ("Discord", "#5865f2", Some("discord")),
        ("Battle.net", "#148eff", Some("battlenet")),
        ("Claude", "#cc785c", Some("claude")),
        ("Mute", "#ff6b6b", None),
        ("Clip", "#ffd166", None),
        ("Shot", "#9d7cff", None),
    ];
    let brands: Vec<Option<&str>> = keys.iter().map(|k| k.2).collect();
    for (i, (label, color, _)) in keys.iter().enumerate() {
        page.keys[i] = key(label, color);
    }
    for (i, label) in ["Volume", "Mic", "Bright", "Scene"].iter().enumerate() {
        page.encoders[i] = bound_encoder(label);
    }

    let key_size = (120u16, 120u16);
    let strip_size = (800u16, 100u16);
    let accents = ["#7ce0ff", "#ff6bd6", "#9d7cff", "#7cffa8"];

    // Compose a montage: 4 columns x 2 rows of keys, then the strip below.
    let gap = 12u32;
    let kw = key_size.0 as u32;
    let kh = key_size.1 as u32;
    let cols = 4u32;
    let rows = 2u32;
    let margin = 20u32;
    let grid_w = cols * kw + (cols - 1) * gap;
    // Render the strip directly at the montage width (it's resolution
    // independent) instead of scaling, so it lands cleanly below the keys.
    let strip_h = 72u32;
    let total_w = grid_w + margin * 2;
    let total_h = margin * 2 + rows * kh + (rows - 1) * gap + gap * 2 + strip_h;

    let mut canvas = Pixmap::new(total_w, total_h).unwrap();
    canvas.fill(resvg::tiny_skia::Color::from_rgba8(12, 13, 20, 255));
    let paint = PixmapPaint::default();

    for i in 0..8u32 {
        let col = i % cols;
        let row = i / cols;
        let img = render_key(&page.keys[i as usize], brands[i as usize], key_size);
        let pm = Pixmap::from_vec(
            img.pixels.clone(),
            resvg::tiny_skia::IntSize::from_wh(kw, kh).unwrap(),
        )
        .unwrap();
        let x = margin + col * (kw + gap);
        let y = margin + row * (kh + gap);
        canvas.draw_pixmap(x as i32, y as i32, pm.as_ref(), &paint, Transform::identity(), None);
    }

    let _ = strip_size;
    let strip = render_strip(&page.encoders, &accents, (grid_w as u16, strip_h as u16), 4);
    let strip_pm = Pixmap::from_vec(
        strip.pixels.clone(),
        resvg::tiny_skia::IntSize::from_wh(grid_w, strip_h).unwrap(),
    )
    .unwrap();
    let strip_y = margin + rows * kh + (rows - 1) * gap + gap * 2;
    canvas.draw_pixmap(
        margin as i32,
        strip_y as i32,
        strip_pm.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );

    std::fs::create_dir_all("target/preview").unwrap();
    canvas.save_png("target/preview/deck.png").unwrap();
    println!("wrote target/preview/deck.png ({total_w}x{total_h})");
}
