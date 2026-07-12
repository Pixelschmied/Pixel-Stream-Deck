//! Rich rendering of key and touch-strip images: brand logos, labels and
//! colour, rasterised from composed SVG via `resvg`. Enabled by the `render`
//! feature.
//!
//! The output is straight [`Rgba8Image`] buffers at the device's native
//! resolution, so this both drives the real hardware and can be dumped to PNG
//! for previewing (see `examples/preview.rs`).

use resvg::tiny_skia::Pixmap;
use resvg::usvg::{Options, Transform, Tree};

use crate::backend::Rgba8Image;
use crate::brand_icons;
use crate::model::{EncoderConfig, KeyConfig};

fn hex_or(color: &Option<String>, default: &str) -> String {
    match color {
        Some(c) if c.starts_with('#') && (c.len() == 7) => c.clone(),
        _ => default.to_string(),
    }
}

/// Escape text for inclusion in SVG.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Rasterise an SVG document to a straight-alpha RGBA image.
fn rasterize(svg: &str, w: u16, h: u16) -> Rgba8Image {
    let mut options = Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = Tree::from_str(svg, &options).expect("internally generated SVG must be valid");
    let mut pixmap = Pixmap::new(w as u32, h as u32).expect("non-zero size");
    resvg::render(&tree, Transform::identity(), &mut pixmap.as_mut());
    // tiny-skia stores premultiplied RGBA; demultiply so downstream consumers
    // (and PNG export) see straight alpha. Backgrounds are opaque so this is a
    // no-op there, but it keeps semi-transparent pixels correct.
    let mut pixels = pixmap.take();
    for px in pixels.chunks_exact_mut(4) {
        let a = px[3];
        if a != 0 && a != 255 {
            px[0] = ((px[0] as u16 * 255) / a as u16) as u8;
            px[1] = ((px[1] as u16 * 255) / a as u16) as u8;
            px[2] = ((px[2] as u16 * 255) / a as u16) as u8;
        }
    }
    Rgba8Image { width: w, height: h, pixels }
}

/// Build the `<g>` that draws a 24×24 brand path scaled into a box.
fn icon_group(path: &str, cx: f32, cy: f32, scale: f32, fill: &str) -> String {
    // Centre the 24×24 icon on (cx, cy).
    let tx = cx - 12.0 * scale;
    let ty = cy - 12.0 * scale;
    format!(
        r#"<g transform="translate({tx:.2},{ty:.2}) scale({scale:.3})"><path d="{path}" fill="{fill}"/></g>"#
    )
}

/// Render a single key image. `brand_id` optionally selects a logo to overlay.
pub fn render_key(key: &KeyConfig, brand_id: Option<&str>, size: (u16, u16)) -> Rgba8Image {
    let (w, h) = size;
    let bg = hex_or(&key.color, "#1e1e2e");
    let label = esc(&key.label);
    let icon = brand_id
        .and_then(brand_icons::brand_path)
        .map(|p| icon_group(p, w as f32 / 2.0, h as f32 * 0.38, w as f32 / 60.0, "#ffffffee"))
        .unwrap_or_default();
    let font = (w as f32 * 0.13).round();
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect x="2" y="2" width="{rw}" height="{rh}" rx="{rx}" fill="{bg}"/>
{icon}
<text x="{tx}" y="{ty}" font-family="sans-serif" font-size="{font}" font-weight="700"
 fill="#ffffff" text-anchor="middle">{label}</text>
</svg>"##,
        rw = w - 4,
        rh = h - 4,
        rx = (w as f32 * 0.12) as u16,
        tx = w / 2,
        ty = (h as f32 * 0.86) as u16,
    );
    rasterize(&svg, w, h)
}

/// Render the whole touch strip: one labelled, colour-banded segment per encoder.
pub fn render_strip(
    encoders: &[EncoderConfig],
    accents: &[&str],
    size: (u16, u16),
    encoder_count: u8,
) -> Rgba8Image {
    let (w, h) = size;
    let count = encoder_count.max(1) as usize;
    let seg = w as f32 / count as f32;
    let font = (h as f32 * 0.26).round();
    let mut bands = String::new();
    for i in 0..count {
        let x = seg * i as f32;
        let accent = accents.get(i).copied().unwrap_or("#7ce0ff");
        let bound = encoders.get(i).map(EncoderConfig::has_binding).unwrap_or(false);
        let (fill, op) = if bound { (accent, "0.22") } else { ("#20222e", "1") };
        let label = esc(encoders.get(i).map(|e| e.label.as_str()).unwrap_or(""));
        bands.push_str(&format!(
            r##"<rect x="{x:.1}" y="0" width="{seg:.1}" height="{h}" fill="{fill}" fill-opacity="{op}"/>
<rect x="{x:.1}" y="{uy}" width="{seg:.1}" height="4" fill="{accent}"/>
<text x="{cx:.1}" y="{ty}" font-family="sans-serif" font-size="{font}" font-weight="600"
 fill="#ffffff" text-anchor="middle">{label}</text>"##,
            uy = h - 4,
            cx = x + seg / 2.0,
            ty = (h as f32 * 0.42) as u16,
        ));
    }
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">
<rect width="{w}" height="{h}" fill="#0c0d14"/>{bands}</svg>"##
    );
    rasterize(&svg, w, h)
}
