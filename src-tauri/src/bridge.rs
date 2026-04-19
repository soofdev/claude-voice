use std::collections::HashMap;
use std::sync::Mutex;

// Per-session reply queue. The Chrome extension polls to drain its session's
// queue and inject the text into the agent's input (e.g. Replit chat box).
pub struct BridgeQueue(Mutex<HashMap<String, Vec<String>>>);

impl BridgeQueue {
    pub fn new() -> Self {
        Self(Mutex::new(HashMap::new()))
    }

    pub fn enqueue(&self, session_id: &str, text: String) {
        let mut guard = self.0.lock().unwrap();
        guard.entry(session_id.to_string()).or_default().push(text);
    }

    pub fn drain(&self, session_id: &str) -> Vec<String> {
        let mut guard = self.0.lock().unwrap();
        guard.remove(session_id).unwrap_or_default()
    }
}

pub fn is_bridge_session(session_id: &str) -> bool {
    session_id.starts_with("replit:")
}
