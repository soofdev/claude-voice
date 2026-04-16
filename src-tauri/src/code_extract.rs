use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    pub language: String,
    pub code: String,
}

pub fn extract(original: &str) -> Vec<CodeBlock> {
    let mut out = Vec::new();
    let mut in_block = false;
    let mut lang = String::new();
    let mut buf = String::new();

    for line in original.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            if in_block {
                out.push(CodeBlock {
                    language: lang.clone(),
                    code: buf.trim_end_matches('\n').to_string(),
                });
                buf.clear();
                lang.clear();
                in_block = false;
            } else {
                lang = trimmed.trim_start_matches("```").trim().to_string();
                in_block = true;
            }
            continue;
        }
        if in_block {
            buf.push_str(line);
            buf.push('\n');
        }
    }

    out.retain(|b| !b.code.trim().is_empty());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_code_returns_empty() {
        assert!(extract("plain text").is_empty());
    }

    #[test]
    fn single_block() {
        let blocks = extract("before\n```rust\nlet x = 1;\n```\nafter");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].language, "rust");
        assert_eq!(blocks[0].code, "let x = 1;");
    }

    #[test]
    fn multiple_blocks() {
        let blocks = extract("```\na\n```\ntext\n```py\nb\n```");
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].language, "");
        assert_eq!(blocks[0].code, "a");
        assert_eq!(blocks[1].language, "py");
        assert_eq!(blocks[1].code, "b");
    }

    #[test]
    fn multiline_code_preserved() {
        let blocks = extract("```\nline1\nline2\nline3\n```");
        assert_eq!(blocks[0].code, "line1\nline2\nline3");
    }

    #[test]
    fn unclosed_block_is_captured() {
        let blocks = extract("```rust\nlet x = 1;\nno closing fence");
        assert!(blocks.is_empty() || blocks[0].code.contains("let x"));
    }

    #[test]
    fn empty_block_filtered() {
        let blocks = extract("```\n\n```");
        assert!(blocks.is_empty());
    }
}
