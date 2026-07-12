# Pixel Gaming Helper

**An open, lightweight replacement for the Elgato Stream Deck software, built for
the Stream Deck +.** It drives all 8 LCD keys, the 800×100 touch strip and the 4
rotary encoders — with brand logos on the keys, a system-tray presence, and a
context engine that **switches profiles automatically depending on the app in
the foreground** (Spotify, Steam, Discord, Battle.net, Claude, …). Built with
**Tauri + Rust** and a **React/TypeScript** UI, and it **updates itself** from
GitHub Releases.

[![CI](https://github.com/Pixelschmied/Pixel-Stream-Deck/actions/workflows/ci.yml/badge.svg)](https://github.com/Pixelschmied/Pixel-Stream-Deck/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/Pixelschmied/Pixel-Stream-Deck?label=download)](https://github.com/Pixelschmied/Pixel-Stream-Deck/releases/latest)

## ⬇️ Download (Windows)

Grab the latest installer from the **[Releases page](https://github.com/Pixelschmied/Pixel-Stream-Deck/releases/latest)**
— run the `…-setup.exe` (or `.msi`). After that you never need to download
again: the app checks for new versions on startup and updates itself.

> Releases are produced automatically by the [release workflow](.github/workflows/release.yml)
> when a `v*` tag is pushed. If the Releases page is still empty, no version has
> been tagged yet — see [Cutting a release](#cutting-a-release).

> Highlights: hardware-agnostic core with 23 tests, real Elgato driver behind a
> feature flag, system tray, context-aware profile switching, rich SVG key
> rendering, working hotkey synthesis and in-app auto-updates.

## Why

The Stream Deck + is a generic USB-HID device. Once you talk to it directly you
get full control over every key image, the 800×100 touch strip and the encoder
turn/press/touch events — no vendor software required. Pixel Gaming Helper is
that direct-control app, with an emphasis on getting the *maximum* out of the
display and being pleasant to configure.

## Architecture

The project is split so that all the interesting logic is testable without a USB
device or a GUI:

```
crates/deck-core/        Pure Rust. No OS/USB/UI dependencies. Fully unit-tested.
  model.rs               Profiles, pages, keys, encoders, actions (serde).
  backend/               DeckBackend trait + MockBackend; real driver behind a
                         feature flag (backend/hardware.rs, `--features hardware`).
  controller.rs          Maps input events -> actions; renders pages (keys + strip).
  store.rs               Load/save profiles as JSON.

src-tauri/               Thin Tauri app (needs system webkit libs to build).
  device.rs              Background worker: owns the deck, renders, dispatches input.
  actions.rs             Executes OS side effects (launch app, run command, URL…).
  settings.rs            User settings, persisted as JSON.
  state.rs, commands.rs  Shared state + the RPC surface for the UI.
  lib.rs                 Window, system tray (close-to-tray), worker wiring.

src/                     React + TypeScript UI (Vite). Deck editor + settings.
```

Key design choice: `deck-core` never performs OS side effects. The controller
decides *what* should happen; the Tauri layer decides *how*. That keeps the
brains of the app compilable and testable anywhere, including headless CI.

### Hardware support

The real device driver uses the [`elgato-streamdeck`](https://crates.io/crates/elgato-streamdeck)
crate and is compiled only with the `hardware` feature, because it pulls in
`hidapi` (which needs `libudev`/`libusb`). Without it — or when no device is
present — the app runs against an in-memory mock, so the UI is fully usable for
configuration regardless.

## Getting started

Prerequisites: **Rust**, **Node + pnpm**, and the
[Tauri system dependencies](https://tauri.app/start/prerequisites/) for your OS
(on Debian/Ubuntu: `libwebkit2gtk-4.1-dev libgtk-3-dev
libayatana-appindicator3-dev librsvg2-dev`; for hotkey synthesis
`libxdo-dev`; for the device driver also `libudev-dev libhidapi-dev`).

```bash
pnpm install

# Run the desktop app in dev mode (uses the mock deck):
pnpm tauri dev

# Run it with real Stream Deck + support:
pnpm tauri dev -- --features hardware
```

Just the UI in a browser (no desktop shell, uses a mock snapshot):

```bash
pnpm dev      # http://localhost:1420
```

### Testing & checks

```bash
cargo test -p deck-core                        # core logic (no hardware needed)
cargo test -p deck-core --features render      # incl. the rich renderer
cargo check -p deck-core --features hardware   # type-check the real driver
pnpm build                                     # type-check + build the UI

# Preview what the deck would display, as a PNG (no device required):
cargo run -p deck-core --features render --example preview
#   -> target/preview/deck.png
```

On Linux you may need `udev` rules so a non-root user can access the device; see
the `elgato-streamdeck` docs.

## Configuration

Profiles and settings are stored as JSON in the OS app-config directory
(e.g. `~/.config/dev.pixelschmied.pixel-gaming-helper/`). A friendly starter
profile is created on first launch. Everything is editable in the app, and the
files are human-readable if you want to hand-edit them.

## Auto-updates

The app uses the Tauri updater: on startup it fetches
`releases/latest/download/latest.json`, and if a newer signed version exists it
shows an **Update available** banner (there's also a manual check in Settings →
Updates). Installing downloads the signed package and relaunches.

Updates are only accepted if they're signed with the private key matching the
public key in `src-tauri/tauri.conf.json`. For CI to sign releases, add two
repository secrets (Settings → Secrets and variables → Actions):

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | contents of the generated private key file |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the key's password (empty if none) |

Regenerate a key with `pnpm tauri signer generate -w mykey.key` and put the
matching `.pub` contents into the `plugins.updater.pubkey` field.

## Cutting a release

```bash
# bump the version in package.json + src-tauri/tauri.conf.json, then:
git tag v0.1.0
git push origin v0.1.0
```

The [release workflow](.github/workflows/release.yml) builds the Windows
installer (with real device support), signs the updater artifacts and publishes
a GitHub Release with `latest.json` — which is exactly what the in-app updater
reads. You can also trigger it from the Actions tab (workflow_dispatch).

## Roadmap

- [x] Hardware-agnostic core (model, backend abstraction, controller) with tests
- [x] Real Stream Deck + driver behind a feature flag
- [x] Tauri shell: system tray, close-to-tray, settings, profile editor
- [x] Full-resolution touch-strip rendering (per-encoder colour bands)
- [x] Rich key/strip rendering: brand logos + labels via resvg (`render` feature)
- [x] Context engine: auto-switch profiles by foreground app
- [x] Built-in profiles for Spotify, Steam, Discord, Battle.net and Claude
- [x] Profile switcher UI with brand icons and animations
- [x] Hotkey synthesis (`send_hotkey`) via enigo (modifiers, media/volume keys)
- [x] In-app auto-updates + Windows release pipeline (GitHub Releases)
- [ ] Rich rendering v2: live gauges, album art, per-key custom images
- [ ] Multi-page navigation UI and profile management
- [ ] Launch-on-startup implementation per platform

## License

MIT.
