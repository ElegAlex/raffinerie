use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Profiles {
    pub profiles: HashMap<String, Vec<String>>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Session {
    #[serde(default)]
    pub filters: serde_json::Value,
    #[serde(default)]
    pub active_profile: Option<String>,
    #[serde(default)]
    pub columns: Vec<String>,
}

fn config_dir() -> PathBuf {
    let base = dirs_next::config_dir().unwrap_or_else(std::env::temp_dir);
    let dir = base.join("raffinerie");
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn load_profiles() -> Profiles {
    let p = config_dir().join("profiles.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_profiles(profiles: &Profiles) -> Result<(), String> {
    let p = config_dir().join("profiles.json");
    let s = serde_json::to_string_pretty(profiles).map_err(|e| e.to_string())?;
    fs::write(&p, s).map_err(|e| e.to_string())
}

pub fn load_session() -> Session {
    let p = config_dir().join("last-session.json");
    fs::read_to_string(&p)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save_session(session: &Session) -> Result<(), String> {
    let p = config_dir().join("last-session.json");
    let s = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(&p, s).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_roundtrip_default() {
        let p = Profiles::default();
        let s = serde_json::to_string(&p).unwrap();
        let q: Profiles = serde_json::from_str(&s).unwrap();
        assert_eq!(q.profiles.len(), 0);
    }

    #[test]
    fn session_default_parses_empty_object() {
        let q: Session = serde_json::from_str("{}").unwrap();
        assert!(q.columns.is_empty());
        assert!(q.active_profile.is_none());
    }
}
