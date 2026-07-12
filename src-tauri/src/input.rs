//! Keyboard/hotkey synthesis via `enigo`. Turns an action's list of key names
//! (e.g. `["ctrl","shift","m"]` or `["playpause"]`) into real key events.
//!
//! The `Enigo` instance is created once per thread and reused: creating a fresh
//! one for every keystroke is expensive and, on Windows, its first synthesized
//! event is frequently dropped — which made keys need several presses and dials
//! barely register. Modifiers are also held briefly around the trigger so target
//! apps (Discord global keybinds, media keys, …) reliably catch the chord.

use std::cell::RefCell;
use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

thread_local! {
    static ENIGO: RefCell<Option<Enigo>> = RefCell::new(None);
}

/// Translate a key name to an enigo [`Key`]. Case-insensitive; single
/// characters map to their Unicode key.
fn parse_key(name: &str) -> Option<Key> {
    let n = name.trim().to_lowercase();
    let key = match n.as_str() {
        "ctrl" | "control" => Key::Control,
        "shift" => Key::Shift,
        "alt" | "option" => Key::Alt,
        "meta" | "super" | "win" | "cmd" | "command" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "space" => Key::Space,
        "esc" | "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "printscreen" | "printscr" | "prtsc" => Key::PrintScr,
        "pageup" | "pgup" => Key::PageUp,
        "pagedown" | "pgdn" => Key::PageDown,
        "home" => Key::Home,
        "end" => Key::End,
        "up" | "uparrow" => Key::UpArrow,
        "down" | "downarrow" => Key::DownArrow,
        "left" | "leftarrow" => Key::LeftArrow,
        "right" | "rightarrow" => Key::RightArrow,
        "volumeup" => Key::VolumeUp,
        "volumedown" => Key::VolumeDown,
        "volumemute" | "mute" => Key::VolumeMute,
        "playpause" | "play" => Key::MediaPlayPause,
        "nexttrack" | "next" => Key::MediaNextTrack,
        "prevtrack" | "prev" | "previoustrack" => Key::MediaPrevTrack,
        _ => return parse_function_or_char(&n),
    };
    Some(key)
}

fn parse_function_or_char(n: &str) -> Option<Key> {
    if let Some(num) = n.strip_prefix('f').and_then(|d| d.parse::<u8>().ok()) {
        return match num {
            1 => Some(Key::F1),
            2 => Some(Key::F2),
            3 => Some(Key::F3),
            4 => Some(Key::F4),
            5 => Some(Key::F5),
            6 => Some(Key::F6),
            7 => Some(Key::F7),
            8 => Some(Key::F8),
            9 => Some(Key::F9),
            10 => Some(Key::F10),
            11 => Some(Key::F11),
            12 => Some(Key::F12),
            13 => Some(Key::F13),
            _ => None,
        };
    }
    let mut chars = n.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(Key::Unicode(c)),
        _ => None,
    }
}

/// Send a hotkey chord using the reused per-thread [`Enigo`]. Modifiers are held
/// while the trigger key is pressed for a short moment, then released.
pub fn send_hotkey(keys: &[String]) -> Result<(), String> {
    let parsed: Vec<Key> = keys.iter().filter_map(|k| parse_key(k)).collect();
    if parsed.is_empty() {
        return Err(format!("no recognisable keys in {keys:?}"));
    }

    ENIGO.with(|cell| {
        let mut guard = cell.borrow_mut();
        if guard.is_none() {
            *guard = Some(Enigo::new(&Settings::default()).map_err(|e| e.to_string())?);
        }
        let enigo = guard.as_mut().expect("just initialized");

        let (modifiers, trigger) = parsed.split_at(parsed.len() - 1);

        for m in modifiers {
            enigo.key(*m, Direction::Press).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(12));
        }

        // Press, hold briefly so the target reliably registers it, then release.
        let result = (|| -> Result<(), String> {
            enigo.key(trigger[0], Direction::Press).map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(24));
            enigo.key(trigger[0], Direction::Release).map_err(|e| e.to_string())?;
            Ok(())
        })();

        // Always release modifiers, even if the trigger failed.
        for m in modifiers.iter().rev() {
            std::thread::sleep(Duration::from_millis(8));
            let _ = enigo.key(*m, Direction::Release);
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_modifiers_and_chars() {
        assert!(matches!(parse_key("Ctrl"), Some(Key::Control)));
        assert!(matches!(parse_key("m"), Some(Key::Unicode('m'))));
        assert!(matches!(parse_key("F13"), Some(Key::F13)));
        assert!(matches!(parse_key("volumeup"), Some(Key::VolumeUp)));
        assert!(parse_key("").is_none());
        assert!(parse_key("notakey").is_none());
    }
}
