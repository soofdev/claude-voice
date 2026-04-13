use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    pub voice: String,
    pub rate: u32,
    pub port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            voice: "Samantha".into(),
            rate: 200,
            port: 8765,
        }
    }
}

fn settings_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".config"));
    base.join("claude-voice").join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        let path = settings_path();
        match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = settings_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }
}

pub struct SettingsStore(pub Mutex<Settings>);

impl SettingsStore {
    pub fn new() -> Self {
        Self(Mutex::new(Settings::load()))
    }

    pub fn get(&self) -> Settings {
        self.0.lock().unwrap().clone()
    }

    pub fn update(&self, new: Settings) -> Result<()> {
        new.save()?;
        *self.0.lock().unwrap() = new;
        Ok(())
    }
}
