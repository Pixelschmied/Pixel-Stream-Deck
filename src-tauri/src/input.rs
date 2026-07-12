//! Keyboard/hotkey synthesis via `enigo`. Turns an action's list of key names
//! (e.g. `["ctrl","shift","m"]` or `["playpause"]`) into real key events.
//!
//! All but the last key are treated as modifiers held down while the last key
//! is clicked, then released — the usual chord behaviour. A single media key
//! (volume, play/pause, …) is just clicked.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

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
    // Function keys F1..F13.
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
    // Single printable character.
    let mut chars = n.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Some(Key::Unicode(c)),
        _ => None,
    }
}

/// Send a hotkey chord. Returns an error string on failure (no display, key not
/// recognised, backend error).
pub fn send_hotkey(keys: &[String]) -> Result<(), String> {
    let parsed: Vec<Key> = keys.iter().filter_map(|k| parse_key(k)).collect();
    if parsed.is_empty() {
        return Err(format!("no recognisable keys in {keys:?}"));
    }

    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    let (modifiers, trigger) = parsed.split_at(parsed.len() - 1);

    for m in modifiers {
        enigo.key(*m, Direction::Press).map_err(|e| e.to_string())?;
    }
    let result = enigo.key(trigger[0], Direction::Click).map_err(|e| e.to_string());
    // Always release modifiers, even if the trigger failed.
    for m in modifiers.iter().rev() {
        let _ = enigo.key(*m, Direction::Release);
    }
    result
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
