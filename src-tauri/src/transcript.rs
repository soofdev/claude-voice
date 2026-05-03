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
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_tmp(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{}", line).unwrap();
        }
        f
    }

    #[test]
    fn extracts_last_assistant_string_content() {
        let f = write_tmp(&[
            r#"{"type":"user","message":{"content":"hi"}}"#,
            r#"{"type":"assistant","message":{"content":"Hello there"}}"#,
        ]);
        assert_eq!(last_assistant_text(f.path()), Some("Hello there".into()));
    }

    #[test]
    fn extracts_last_of_multiple_assistants() {
        let f = write_tmp(&[
            r#"{"type":"assistant","message":{"content":"first"}}"#,
            r#"{"type":"user","message":{"content":"ok"}}"#,
            r#"{"type":"assistant","message":{"content":"second"}}"#,
        ]);
        assert_eq!(last_assistant_text(f.path()), Some("second".into()));
    }

    #[test]
    fn extracts_text_from_array_content() {
        let f = write_tmp(&[
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"part one"},{"type":"text","text":"part two"}]}}"#,
        ]);
        assert_eq!(
            last_assistant_text(f.path()),
            Some("part one\n\npart two".into())
        );
    }

    #[test]
    fn skips_tool_use_in_array() {
        let f = write_tmp(&[
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","name":"read"},{"type":"text","text":"done"}]}}"#,
        ]);
        assert_eq!(last_assistant_text(f.path()), Some("done".into()));
    }

    #[test]
    fn returns_none_for_no_assistant() {
        let f = write_tmp(&[r#"{"type":"user","message":{"content":"hi"}}"#]);
        assert_eq!(last_assistant_text(f.path()), None);
    }

    #[test]
    fn returns_none_for_empty_file() {
        let f = write_tmp(&[]);
        assert_eq!(last_assistant_text(f.path()), None);
    }

    #[test]
    fn skips_invalid_json_lines() {
        let f = write_tmp(&[
            "not json",
            r#"{"type":"assistant","message":{"content":"ok"}}"#,
            "also bad",
        ]);
        assert_eq!(last_assistant_text(f.path()), Some("ok".into()));
    }

    #[test]
    fn skips_empty_assistant_text() {
        let f = write_tmp(&[
            r#"{"type":"assistant","message":{"content":""}}"#,
            r#"{"type":"assistant","message":{"content":"real"}}"#,
        ]);
        assert_eq!(last_assistant_text(f.path()), Some("real".into()));
    }

    #[test]
    fn returns_none_for_missing_file() {
        assert_eq!(last_assistant_text(Path::new("/nonexistent/path")), None);
    }
}
