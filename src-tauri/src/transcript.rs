use serde_json::Value;
use std::fs;
use std::path::Path;

pub fn last_assistant_text(transcript_path: &Path) -> Option<String> {
    let contents = fs::read_to_string(transcript_path).ok()?;
    let mut last: Option<String> = None;
    for line in contents.lines() {
        let v: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if v.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }
        let content = match v.pointer("/message/content") {
            Some(c) => c,
            None => continue,
        };
        let text = extract_text(content);
        if !text.trim().is_empty() {
            last = Some(text);
        }
    }
    last
}

fn extract_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(items) => {
            let mut parts = Vec::new();
            for item in items {
                if item.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(t) = item.get("text").and_then(|t| t.as_str()) {
                        parts.push(t.to_string());
                    }
                }
            }
            parts.join("\n\n")
        }
        _ => String::new(),
    }
}
