use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::Mutex;

use crate::code_extract::CodeBlock;
use crate::link_extract::Link;
use crate::tts::{SessionTag, Word};

const MAX_ENTRIES: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp_ms: u128,
    pub original: String,
    pub spoken: String,
    #[serde(default)]
    pub links: Vec<Link>,
    #[serde(default)]
    pub code_blocks: Vec<CodeBlock>,
    #[serde(default)]
    pub words: Vec<Word>,
    #[serde(default)]
    pub audio_path: Option<String>,
    #[serde(default)]
    pub backend: String,
    #[serde(default)]
    pub session: Option<SessionTag>,
    // True while the TTS pipeline is still summarizing/fetching audio for
    // this message. The UI uses this to show a queued indicator and hide
    // replay controls.
    #[serde(default)]
    pub pending: bool,
}

pub struct HistoryStore(Mutex<Vec<HistoryEntry>>);

impl HistoryStore {
    pub fn new() -> Self {
        Self(Mutex::new(load()))
    }

    pub fn list(&self) -> Vec<HistoryEntry> {
        let entries = self.0.lock().unwrap();
        let mut out = entries.clone();
        out.reverse();
        out
    }

    pub fn get(&self, id: &str) -> Option<HistoryEntry> {
        self.0.lock().unwrap().iter().find(|e| e.id == id).cloned()
    }

    pub fn append(&self, entry: HistoryEntry) {
        let (snapshot, evicted) = {
            let mut guard = self.0.lock().unwrap();
            guard.push(entry);
            let mut evicted: Vec<HistoryEntry> = Vec::new();
            if guard.len() > MAX_ENTRIES {
                let overflow = guard.len() - MAX_ENTRIES;
                evicted = guard.drain(0..overflow).collect();
            }
            (guard.clone(), evicted)
        };
        for e in evicted {
            delete_audio_of(&e);
        }
        if let Err(e) = save(&snapshot) {
            eprintln!("[claude-voice] history save failed: {e}");
        }
    }

    /// Replace the entry with the matching id (or append if missing).
    /// Used when a pending entry finishes processing.
    pub fn update(&self, entry: HistoryEntry) {
        let snapshot = {
            let mut guard = self.0.lock().unwrap();
            let mut found = false;
            for e in guard.iter_mut() {
                if e.id == entry.id {
                    *e = entry.clone();
                    found = true;
                    break;
                }
            }
            if !found {
                guard.push(entry);
            }
            guard.clone()
        };
        if let Err(e) = save(&snapshot) {
            eprintln!("[claude-voice] history save failed: {e}");
        }
    }

    pub fn remove_entry(&self, id: &str) {
        let removed = {
            let mut guard = self.0.lock().unwrap();
            let before = guard.len();
            guard.retain(|e| e.id != id);
            let after = guard.len();
            if before != after {
                let snapshot = guard.clone();
                drop(guard);
                if let Err(e) = save(&snapshot) {
                    eprintln!("[claude-voice] history save failed: {e}");
                }
                true
            } else {
                false
            }
        };
        if removed {
            let path = audio_path_for(id);
            let _ = std::fs::remove_file(path);
        }
    }

    pub fn clear(&self) {
        let removed = {
            let mut guard = self.0.lock().unwrap();
            std::mem::take(&mut *guard)
        };
        for e in removed {
            delete_audio_of(&e);
        }
        if let Err(e) = save(&[]) {
            eprintln!("[claude-voice] history clear failed: {e}");
        }
    }
}

fn history_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".config"));
    base.join("claude-voice").join("history.jsonl")
}

fn load() -> Vec<HistoryEntry> {
    let path = history_path();
    let Ok(file) = File::open(&path) else {
        return Vec::new();
    };
    // Tighten perms on upgrade. Best-effort.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    let reader = BufReader::new(file);
    reader
        .lines()
        .filter_map(|l| l.ok())
        .filter_map(|l| serde_json::from_str(&l).ok())
        .collect()
}

fn save(entries: &[HistoryEntry]) -> Result<()> {
    let path = history_path();
    let mut buf = String::new();
    for e in entries {
        buf.push_str(&serde_json::to_string(e)?);
        buf.push('\n');
    }
    // 0o600: history entries include verbatim agent output, which can
    // contain code, command output, or other user-private content.
    crate::fs_util::write_atomic_with_dir_mode(&path, buf.as_bytes(), 0o600, 0o700)?;
    Ok(())
}

pub fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

pub fn audio_dir() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap().join(".config"));
    base.join("claude-voice").join("audio")
}

pub fn audio_path_for(id: &str) -> PathBuf {
    audio_dir().join(format!("{id}.mp3"))
}

fn delete_audio_of(entry: &HistoryEntry) {
    if let Some(path) = entry.audio_path.as_deref() {
        let _ = std::fs::remove_file(path);
    }
}
