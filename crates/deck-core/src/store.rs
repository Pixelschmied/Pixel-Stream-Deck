//! Loading and saving profiles as JSON. Pure (de)serialization plus small
//! filesystem helpers — no knowledge of where the app chooses to store things.

use std::path::Path;

use crate::model::Profile;

/// Errors from reading/writing profile files.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid profile json: {0}")]
    Json(#[from] serde_json::Error),
}

/// Serialize a profile to pretty JSON.
pub fn to_json(profile: &Profile) -> Result<String, StoreError> {
    Ok(serde_json::to_string_pretty(profile)?)
}

/// Parse a profile from JSON, normalizing slot counts afterwards so hand-edited
/// files with missing keys/encoders still load cleanly.
pub fn from_json(json: &str) -> Result<Profile, StoreError> {
    let mut profile: Profile = serde_json::from_str(json)?;
    profile.normalize();
    Ok(profile)
}

/// Write a profile to `path` as pretty JSON, creating parent directories.
pub fn save(profile: &Profile, path: impl AsRef<Path>) -> Result<(), StoreError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, to_json(profile)?)?;
    Ok(())
}

/// Read a profile from `path`.
pub fn load(path: impl AsRef<Path>) -> Result<Profile, StoreError> {
    let json = std::fs::read_to_string(path)?;
    from_json(&json)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Action, Profile};

    #[test]
    fn roundtrip_preserves_actions() {
        let mut prof = Profile::new("demo", "Demo");
        prof.pages[0].keys[0].action = Action::OpenUrl { url: "https://x.dev".into() };
        let json = to_json(&prof).unwrap();
        let back = from_json(&json).unwrap();
        assert_eq!(prof, back);
    }

    #[test]
    fn from_json_normalizes_slots() {
        // A profile page with too few keys should be padded on load.
        let json = r#"{"id":"p","name":"P","pages":[{"id":"main","name":"Main","keys":[],"encoders":[]}]}"#;
        let prof = from_json(json).unwrap();
        assert_eq!(prof.pages[0].keys.len(), crate::model::KEY_COUNT);
        assert_eq!(prof.pages[0].encoders.len(), crate::model::ENCODER_COUNT);
    }
}
