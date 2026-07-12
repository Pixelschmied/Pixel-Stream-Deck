//! Spotify integration via the Web API (rspotify, PKCE flow).
//!
//! This is the reliable, focus-independent way to control Spotify: it targets
//! the user's Spotify account directly rather than relying on global media keys.
//! Requires Spotify Premium (for playback control) and a one-time browser login.
//!
//! Auth uses Authorization Code with PKCE (no client secret needed). The user
//! registers a free Spotify app to get a Client ID and configures the redirect
//! URI `http://127.0.0.1:8888/callback`; a tiny local listener captures the
//! auth code from the browser redirect.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Mutex;

use rspotify::clients::{BaseClient, OAuthClient};
use rspotify::{scopes, AuthCodePkceSpotify, Config, Credentials, OAuth};
use tauri::async_runtime::block_on;

/// Loopback redirect the user must register on their Spotify app.
const REDIRECT_URI: &str = "http://127.0.0.1:8888/callback";
const CALLBACK_ADDR: &str = "127.0.0.1:8888";

/// The connected Spotify client, once authorized.
static CLIENT: Mutex<Option<AuthCodePkceSpotify>> = Mutex::new(None);

/// A snapshot of what's currently playing, for the touch-strip timeline.
#[derive(Clone, Debug, serde::Serialize, Default)]
pub struct NowPlaying {
    pub is_playing: bool,
    pub title: String,
    pub artist: String,
    pub progress_ms: u64,
    pub duration_ms: u64,
    pub has_track: bool,
}

fn cache_path(config_dir: &std::path::Path) -> PathBuf {
    config_dir.join("spotify-token.json")
}

fn build_client(client_id: &str, config_dir: &std::path::Path) -> AuthCodePkceSpotify {
    let creds = Credentials::new_pkce(client_id);
    let oauth = OAuth {
        redirect_uri: REDIRECT_URI.to_string(),
        scopes: scopes!("user-read-playback-state", "user-modify-playback-state"),
        ..Default::default()
    };
    let config = Config {
        token_cached: true,
        cache_path: cache_path(config_dir),
        ..Default::default()
    };
    AuthCodePkceSpotify::with_config(creds, oauth, config)
}

/// Try to restore a previously cached session on startup.
pub fn restore(client_id: &str, config_dir: &std::path::Path) {
    if client_id.trim().is_empty() {
        return;
    }
    let mut spotify = build_client(client_id, config_dir);
    let ok = block_on(async {
        match spotify.read_token_cache(true).await {
            Ok(Some(token)) => {
                *spotify.get_token().lock().await.unwrap() = Some(token);
                spotify.refresh_token().await.is_ok()
            }
            _ => false,
        }
    });
    if ok {
        *CLIENT.lock().unwrap() = Some(spotify);
        println!("[spotify] restored session");
    }
}

/// Whether we currently have an authorized client.
pub fn is_connected() -> bool {
    CLIENT.lock().unwrap().is_some()
}

/// Begin the login: returns the URL to open in the browser and spawns a local
/// listener that captures the redirect code and completes the token exchange.
pub fn begin_login(client_id: &str, config_dir: PathBuf) -> Result<String, String> {
    if client_id.trim().is_empty() {
        return Err("Bitte zuerst die Spotify Client-ID in den Einstellungen eintragen".into());
    }
    let mut spotify = build_client(client_id, &config_dir);
    let url = spotify
        .get_authorize_url(None)
        .map_err(|e| format!("Auth-URL fehlgeschlagen: {e}"))?;

    // Wait for the redirect on a background thread so the UI can open the URL.
    std::thread::spawn(move || {
        match wait_for_code() {
            Ok(code) => {
                let done = block_on(async { spotify.request_token(&code).await });
                match done {
                    Ok(()) => {
                        *CLIENT.lock().unwrap() = Some(spotify);
                        println!("[spotify] login complete");
                    }
                    Err(e) => eprintln!("[spotify] token exchange failed: {e}"),
                }
            }
            Err(e) => eprintln!("[spotify] callback error: {e}"),
        }
    });

    Ok(url)
}

/// Block until the browser hits the loopback redirect, then return the `code`.
fn wait_for_code() -> Result<String, String> {
    let listener = TcpListener::bind(CALLBACK_ADDR).map_err(|e| e.to_string())?;
    let (mut stream, _) = listener.accept().map_err(|e| e.to_string())?;
    let mut buf = [0u8; 2048];
    let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]);
    // First line: "GET /callback?code=... HTTP/1.1"
    let code = request
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|path| path.split("code=").nth(1))
        .map(|rest| rest.split('&').next().unwrap_or("").to_string())
        .filter(|c| !c.is_empty())
        .ok_or_else(|| "kein code im Redirect".to_string())?;

    let body = "<html><body style='font-family:sans-serif;background:#11121c;color:#e6e6f0'>\
        <h2>Pixel Gaming Helper ist jetzt mit Spotify verbunden.</h2>\
        <p>Du kannst dieses Fenster schließen.</p></body></html>";
    let _ = stream.write_all(
        format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}", body.len(), body)
            .as_bytes(),
    );
    Ok(code)
}

/// Run a control command. `op`: play_pause, next, prev; seek uses [`seek_ms`].
pub fn control(op: &str) -> Result<(), String> {
    with_client(|c| {
        block_on(async {
            match op {
                "play_pause" => {
                    let playing = c
                        .current_playback(None, None::<Vec<_>>)
                        .await
                        .ok()
                        .flatten()
                        .map(|ctx| ctx.is_playing)
                        .unwrap_or(false);
                    if playing {
                        c.pause_playback(None).await
                    } else {
                        c.resume_playback(None, None).await
                    }
                }
                "next" => c.next_track(None).await,
                "prev" => c.previous_track(None).await,
                other => return Err(format!("unbekannte Spotify-Aktion: {other}")),
            }
            .map_err(|e| e.to_string())
        })
    })
}

/// Seek to an absolute position in the current track.
pub fn seek_ms(position_ms: u64) -> Result<(), String> {
    with_client(|c| {
        block_on(async {
            let pos = chrono::TimeDelta::try_milliseconds(position_ms as i64)
                .ok_or_else(|| "ungültige Position".to_string())?;
            c.seek_track(pos, None).await.map_err(|e| e.to_string())
        })
    })
}

/// Fetch the current playback for the touch-strip timeline.
pub fn now_playing() -> Option<NowPlaying> {
    let guard = CLIENT.lock().unwrap();
    let c = guard.as_ref()?;
    block_on(async {
        let ctx = c.current_playback(None, None::<Vec<_>>).await.ok()??;
        let progress_ms = ctx.progress.map(|d| d.num_milliseconds().max(0) as u64).unwrap_or(0);
        let (title, artist, duration_ms) = match ctx.item {
            Some(rspotify::model::PlayableItem::Track(t)) => (
                t.name,
                t.artists.first().map(|a| a.name.clone()).unwrap_or_default(),
                t.duration.num_milliseconds().max(0) as u64,
            ),
            Some(rspotify::model::PlayableItem::Episode(e)) => {
                (e.name, e.show.name, e.duration.num_milliseconds().max(0) as u64)
            }
            _ => (String::new(), String::new(), 0),
        };
        Some(NowPlaying {
            is_playing: ctx.is_playing,
            has_track: duration_ms > 0,
            title,
            artist,
            progress_ms,
            duration_ms,
        })
    })
}

fn with_client<F: FnOnce(&AuthCodePkceSpotify) -> Result<(), String>>(f: F) -> Result<(), String> {
    let guard = CLIENT.lock().unwrap();
    let c = guard.as_ref().ok_or("Spotify ist nicht verbunden")?;
    f(c)
}
