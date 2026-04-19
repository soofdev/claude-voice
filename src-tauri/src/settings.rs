use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub enabled: bool,
    pub port: u16,
    #[serde(default = "default_true")]
    pub show_popup: bool,
    #[serde(default)]
    pub pin_popup: bool,
    #[serde(default = "default_dismiss_delay")]
    pub popup_dismiss_delay_ms: u32,
    #[serde(default = "default_true")]
    pub speak_session_prefix: bool,
    #[serde(default = "default_prefix_skip_window")]
    pub prefix_skip_window_ms: u32,
    #[serde(default = "default_orb_style")]
    pub orb_style: String,
    #[serde(default)]
    pub popup_width: Option<f64>,
    #[serde(default)]
    pub popup_height: Option<f64>,
    #[serde(default)]
    pub popup_x: Option<f64>,
    #[serde(default)]
    pub popup_y: Option<f64>,
    #[serde(default)]
    pub history_panel_width: Option<f64>,

    #[serde(default = "default_backend")]
    pub backend: String,

    #[serde(default = "default_say_voice")]
    pub voice: String,
    #[serde(default = "default_rate")]
    pub rate: u32,

    #[serde(default)]
    pub browser_voice: String,
    #[serde(default = "default_browser_rate")]
    pub browser_rate: f32,

    #[serde(default)]
    pub elevenlabs_api_key: String,
    #[serde(default = "default_eleven_voice")]
    pub elevenlabs_voice_id: String,
    #[serde(default = "default_eleven_model")]
    pub elevenlabs_model_id: String,
    #[serde(default = "default_eleven_speed")]
    pub elevenlabs_speed: f32,

    #[serde(default = "default_true")]
    pub summarize: bool,
    #[serde(default)]
    pub anthropic_api_key: String,
    #[serde(default = "default_summary_model")]
    pub summary_model: String,
    #[serde(default = "default_summary_threshold")]
    pub summary_threshold_chars: u32,
    #[serde(default = "default_summary_brevity")]
    pub summary_brevity: String,
    // When true, recent spoken messages from the same session (up to 5
    // within the last 30 minutes) are passed to the summarizer as
    // context, and it's instructed to skip anything already covered.
    #[serde(default)]
    pub avoid_repetition: bool,
}

fn default_true() -> bool {
    true
}
fn default_backend() -> String {
    "say".into()
}
fn default_say_voice() -> String {
    "Samantha".into()
}
fn default_rate() -> u32 {
    200
}
fn default_browser_rate() -> f32 {
    1.0
}
fn default_eleven_voice() -> String {
    "pNInz6obpgDQGcFmaJgB".into()
}
fn default_eleven_model() -> String {
    "eleven_flash_v2_5".into()
}
fn default_eleven_speed() -> f32 {
    0.9
}
fn default_summary_model() -> String {
    "claude-haiku-4-5-20251001".into()
}
fn default_summary_threshold() -> u32 {
    180
}
fn default_summary_brevity() -> String {
    "balanced".into()
}
fn default_dismiss_delay() -> u32 {
    1500
}
fn default_prefix_skip_window() -> u32 {
    30000
}
fn default_orb_style() -> String {
    "glass".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: true,
            port: 8765,
            show_popup: true,
            pin_popup: false,
            popup_dismiss_delay_ms: default_dismiss_delay(),
            speak_session_prefix: true,
            prefix_skip_window_ms: default_prefix_skip_window(),
            orb_style: default_orb_style(),
            popup_width: None,
            popup_height: None,
            popup_x: None,
            popup_y: None,
            history_panel_width: None,
            backend: default_backend(),
            voice: default_say_voice(),
            rate: default_rate(),
            browser_voice: String::new(),
            browser_rate: default_browser_rate(),
            elevenlabs_api_key: String::new(),
            elevenlabs_voice_id: default_eleven_voice(),
            elevenlabs_model_id: default_eleven_model(),
            elevenlabs_speed: default_eleven_speed(),
            summarize: true,
            anthropic_api_key: String::new(),
            summary_model: default_summary_model(),
            summary_threshold_chars: default_summary_threshold(),
            summary_brevity: default_summary_brevity(),
            avoid_repetition: false,
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
