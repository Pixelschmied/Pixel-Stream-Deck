//! Executing an [`Action`] as an actual OS side effect. This is the platform
//! layer the pure `deck-core` controller deliberately leaves out.
//!
//! Page switches and brightness are handled inside the controller, so only the
//! OS-facing variants are meaningful here.

use std::process::Command;

use deck_core::Action;

/// Run the OS side effect for `action`. Returns a human-readable error string on
/// failure (suitable for surfacing to the UI).
pub fn execute(action: &Action) -> Result<(), String> {
    match action {
        Action::LaunchApp { path, args } => launch_app(path, args),
        Action::RunCommand { command } => run_shell(command),
        Action::OpenUrl { url } => open_target(url),
        Action::SendHotkey { keys } => crate::input::send_hotkey(keys),
        Action::Spotify { op } => crate::spotify::control(op),
        // Handled inside the controller; nothing to do at the OS level.
        Action::SwitchPage { .. } | Action::AdjustBrightness { .. } | Action::None => Ok(()),
    }
}

/// Map a well-known app name to a reliable launch URI, so bare names like
/// "steam" (which are NOT on the Windows PATH) still work — including in
/// profiles that were already saved to disk.
fn known_app_uri(name: &str) -> Option<&'static str> {
    match name.trim().to_lowercase().as_str() {
        "steam" => Some("steam://open/main"),
        "spotify" => Some("spotify:"),
        "discord" => Some("discord://"),
        "battle.net" | "battlenet" | "battle net" | "blizzard" => Some("battlenet://"),
        "epic" | "epicgames" | "epic games" => Some("com.epicgames.launcher://"),
        _ => None,
    }
}

/// Launch an application robustly: known apps go through their URI scheme, real
/// URIs/paths open via the OS, and everything else is spawned directly with a
/// shell-`start` fallback so registered apps resolve.
fn launch_app(path: &str, args: &[String]) -> Result<(), String> {
    if let Some(uri) = known_app_uri(path) {
        return open_target(uri);
    }
    if path.contains("://") || path.ends_with(':') {
        return open_target(path);
    }
    // Direct spawn works for full paths and things actually on PATH.
    if Command::new(path).args(args).spawn().is_ok() {
        return Ok(());
    }
    // Fallback: let the shell resolve App Execution Aliases / registered paths.
    #[cfg(target_os = "windows")]
    {
        let mut c = Command::new("cmd");
        c.args(["/C", "start", "", path]);
        c.args(args);
        return c
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("konnte {path} nicht starten: {e}"));
    }
    #[cfg(not(target_os = "windows"))]
    Err(format!("konnte {path} nicht starten"))
}

/// Run a command line through the platform shell.
fn run_shell(command: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    };
    #[cfg(not(target_os = "windows"))]
    let mut cmd = {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("failed to run command: {e}"))
}

/// Open a URL/path with the OS default handler (public wrapper).
pub fn open_url(target: &str) -> Result<(), String> {
    open_target(target)
}

/// Open a URL or file path with the OS default handler.
fn open_target(target: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let result = Command::new("cmd").args(["/C", "start", "", target]).spawn();
    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(target).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(target).spawn();

    result
        .map(|_| ())
        .map_err(|e| format!("failed to open {target}: {e}"))
}
