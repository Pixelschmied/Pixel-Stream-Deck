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
        Action::LaunchApp { path, args } => Command::new(path)
            .args(args)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("failed to launch {path}: {e}")),
        Action::RunCommand { command } => run_shell(command),
        Action::OpenUrl { url } => open_target(url),
        Action::SendHotkey { keys } => {
            // Synthesizing key events is platform-specific and needs an extra
            // dependency (e.g. `enigo`); wired up in a later milestone.
            eprintln!("[action] SendHotkey {keys:?} is not implemented yet");
            Ok(())
        }
        // Handled inside the controller; nothing to do at the OS level.
        Action::SwitchPage { .. } | Action::AdjustBrightness { .. } | Action::None => Ok(()),
    }
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
