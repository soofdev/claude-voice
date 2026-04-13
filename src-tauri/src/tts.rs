use std::process::{Child, Command};
use std::sync::Mutex;

pub struct TtsEngine {
    current: Mutex<Option<Child>>,
}

impl TtsEngine {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
        }
    }

    pub fn speak(&self, text: &str, voice: &str, rate: u32) {
        self.stop();
        if text.trim().is_empty() {
            return;
        }
        let mut cmd = Command::new("say");
        cmd.arg("-v").arg(voice);
        cmd.arg("-r").arg(rate.to_string());
        cmd.arg(text);
        match cmd.spawn() {
            Ok(child) => {
                *self.current.lock().unwrap() = Some(child);
            }
            Err(e) => eprintln!("[claude-voice] failed to spawn say: {e}"),
        }
    }

    pub fn stop(&self) {
        let mut guard = self.current.lock().unwrap();
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub fn is_speaking(&self) -> bool {
        let mut guard = self.current.lock().unwrap();
        match guard.as_mut() {
            Some(child) => match child.try_wait() {
                Ok(Some(_)) => {
                    *guard = None;
                    false
                }
                Ok(None) => true,
                Err(_) => false,
            },
            None => false,
        }
    }
}

pub fn list_voices() -> Vec<String> {
    let output = match Command::new("say").arg("-v").arg("?").output() {
        Ok(o) => o,
        Err(_) => return vec!["Samantha".into()],
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut voices: Vec<String> = text
        .lines()
        .filter_map(|l| {
            let name = l.split_whitespace().next()?.to_string();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        })
        .collect();
    voices.sort();
    voices.dedup();
    voices
}
