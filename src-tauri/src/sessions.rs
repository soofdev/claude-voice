use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: String,
    #[serde(default)]
    pub cwd: String,
    pub label: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_seen_ms: u128,
    #[serde(default)]
    pub voice_override: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub tty: Option<String>,
}

const PALETTE: &[&str] = &[
    "#ff5a5f", "#ff9a3c", "#f5cb5c", "#4ecb71", "#4dc4d3",
    "#5b9bff", "#a47eff", "#ff77c0", "#8b9dab", "#b8e986",
];

fn pick_color(session_id: &str) -> String {
    let mut h: u32 = 5381;
    for b in session_id.bytes() {
        h = h.wrapping_mul(33) ^ b as u32;
    }
    PALETTE[(h as usize) % PALETTE.len()].to_string()
}

fn default_true() -> bool {
    true
}

pub struct SessionsStore(Mutex<HashMap<String, SessionInfo>>);

impl SessionsStore {
    pub fn new() -> Self {
        Self(Mutex::new(load()))
    }

    pub fn list(&self) -> Vec<SessionInfo> {
        let guard = self.0.lock().unwrap();
        let mut out: Vec<SessionInfo> = guard.values().cloned().collect();
        out.sort_by(|a, b| b.last_seen_ms.cmp(&a.last_seen_ms));
        out
    }

    pub fn is_enabled(&self, session_id: &str) -> bool {
        self.0
            .lock()
            .unwrap()
            .get(session_id)
            .map(|s| s.enabled)
            .unwrap_or(true)
    }

    pub fn upsert_active(
        &self,
        session_id: &str,
        cwd: &str,
        tty: Option<&str>,
        now_ms: u128,
    ) -> SessionInfo {
        let entry = {
            let mut guard = self.0.lock().unwrap();
            let e = guard
                .entry(session_id.to_string())
                .or_insert_with(|| SessionInfo {
                    session_id: session_id.to_string(),
                    cwd: cwd.to_string(),
                    label: default_label(cwd, session_id),
                    enabled: true,
                    last_seen_ms: now_ms,
                    voice_override: None,
                    color: Some(pick_color(session_id)),
                    tty: tty.map(String::from),
                });
            if e.color.is_none() {
                e.color = Some(pick_color(session_id));
            }
            e.last_seen_ms = now_ms;
            if !cwd.is_empty() && e.cwd != cwd {
                e.cwd = cwd.to_string();
            }
            if let Some(t) = tty {
                if !t.is_empty() && t != "/dev/" {
                    e.tty = Some(t.to_string());
                }
            }
            e.clone()
        };
        let _ = save(&self.snapshot());
        entry
    }

    pub fn set_enabled(&self, session_id: &str, enabled: bool) -> bool {
        let changed = {
            let mut guard = self.0.lock().unwrap();
            if let Some(s) = guard.get_mut(session_id) {
                s.enabled = enabled;
                true
            } else {
                false
            }
        };
        if changed {
            let _ = save(&self.snapshot());
        }
        changed
    }

    pub fn get(&self, session_id: &str) -> Option<SessionInfo> {
        self.0.lock().unwrap().get(session_id).cloned()
    }

    pub fn set_color(&self, session_id: &str, color: Option<String>) -> bool {
        let changed = {
            let mut guard = self.0.lock().unwrap();
            if let Some(s) = guard.get_mut(session_id) {
                s.color = color.filter(|c| !c.is_empty());
                true
            } else {
                false
            }
        };
        if changed {
            let _ = save(&self.snapshot());
        }
        changed
    }

    pub fn set_voice(&self, session_id: &str, voice_id: Option<String>) -> bool {
        let changed = {
            let mut guard = self.0.lock().unwrap();
            if let Some(s) = guard.get_mut(session_id) {
                s.voice_override = voice_id.filter(|v| !v.is_empty());
                true
            } else {
                false
            }
        };
        if changed {
            let _ = save(&self.snapshot());
        }
        changed
    }

    pub fn rename(&self, session_id: &str, label: &str) -> bool {
        let changed = {
            let mut guard = self.0.lock().unwrap();
            if let Some(s) = guard.get_mut(session_id) {
                s.label = label.to_string();
                true
            } else {
                false
            }
        };
        if changed {
            let _ = save(&self.snapshot());
        }
        changed
    }

    pub fn remove(&self, session_id: &str) -> bool {
        let removed = self.0.lock().unwrap().remove(session_id).is_some();
        if removed {
            let _ = save(&self.snapshot());
        }
        removed
    }

    fn snapshot(&self) -> Vec<SessionInfo> {
        self.0.lock().unwrap().values().cloned().collect()
    }
}

fn default_label(cwd: &str, session_id: &str) -> String {
    if cwd.is_empty() {
        return format!("Session {}", &session_id[..session_id.len().min(8)]);
    }
    let name = std::path::Path::new(cwd)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(cwd);
    if name.is_empty() {
        format!("Session {}", &session_id[..session_id.len().min(8)])
    } else {
        name.to_string()
    }
}

fn sessions_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".config"));
    base.join("claude-voice").join("sessions.json")
}

fn load() -> HashMap<String, SessionInfo> {
    let path = sessions_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };
    let list: Vec<SessionInfo> = serde_json::from_str(&content).unwrap_or_default();
    list.into_iter().map(|s| (s.session_id.clone(), s)).collect()
}

fn save(entries: &[SessionInfo]) -> Result<()> {
    let path = sessions_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(entries)?)?;
    Ok(())
}
