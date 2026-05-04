pub fn clean_for_speech(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_code_block = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }
        out.push_str(&strip_inline(line));
        out.push('\n');
    }
    out.trim().to_string()
}

fn strip_inline(line: &str) -> String {
    let mut s = String::with_capacity(line.len());
    let mut chars = line.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match c {
            '`' => {
                if let Some(end_rel) = line[i + 1..].find('`') {
                    let end = i + 1 + end_rel;
                    s.push_str(&line[i + 1..end]);
                    while let Some(&(j, _)) = chars.peek() {
                        if j > end {
                            break;
                        }
                        chars.next();
                    }
                    continue;
                }
                s.push(c);
            }
            '[' => {
                if let Some(close_rel) = line[i + 1..].find("](") {
                    let label_end = i + 1 + close_rel;
                    let paren_start = label_end + 2;
                    if let Some(end_rel) = line[paren_start..].find(')') {
                        let end = paren_start + end_rel;
                        s.push_str(&line[i + 1..label_end]);
                        while let Some(&(j, _)) = chars.peek() {
                            if j > end {
                                break;
                            }
                            chars.next();
                        }
                        continue;
                    }
                }
                s.push(c);
            }
            '*' | '_' | '#' | '>' | '~' => {}
            _ => s.push(c),
        }
    }
    s
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_unchanged() {
        assert_eq!(clean_for_speech("Hello world"), "Hello world");
    }

    #[test]
    fn strips_code_blocks() {
        let input = "before\n```rust\nlet x = 1;\n```\nafter";
        assert_eq!(clean_for_speech(input), "before\nafter");
    }

    #[test]
    fn strips_nested_code_blocks() {
        let input = "a\n```\ncode\n```\nb\n```\nmore\n```\nc";
        assert_eq!(clean_for_speech(input), "a\nb\nc");
    }

    #[test]
    fn strips_bold_and_italic() {
        assert_eq!(clean_for_speech("**bold** and *italic*"), "bold and italic");
    }

    #[test]
    fn strips_underscores() {
        assert_eq!(clean_for_speech("__also bold__"), "also bold");
    }

    #[test]
    fn strips_headers() {
        assert_eq!(clean_for_speech("## Header"), "Header");
    }

    #[test]
    fn strips_blockquotes() {
        assert_eq!(clean_for_speech("> quoted text"), "quoted text");
    }

    #[test]
    fn strips_strikethrough() {
        assert_eq!(clean_for_speech("~~deleted~~"), "deleted");
    }

    #[test]
    fn inline_code_keeps_content() {
        assert_eq!(
            clean_for_speech("run `npm install` now"),
            "run npm install now"
        );
    }

    #[test]
    fn unclosed_backtick_preserved() {
        assert_eq!(clean_for_speech("it`s fine"), "it`s fine");
    }

    #[test]
    fn markdown_link_keeps_label() {
        assert_eq!(
            clean_for_speech("see [the docs](https://example.com) here"),
            "see the docs here"
        );
    }

    #[test]
    fn bracket_without_link_preserved() {
        assert_eq!(clean_for_speech("[not a link]"), "[not a link]");
    }

    #[test]
    fn utf8_preserved() {
        assert_eq!(clean_for_speech("café résumé naïve"), "café résumé naïve");
    }

    #[test]
    fn empty_input() {
        assert_eq!(clean_for_speech(""), "");
    }

    #[test]
    fn only_code_block() {
        assert_eq!(clean_for_speech("```\ncode\n```"), "");
    }

    #[test]
    fn multiple_formatting() {
        assert_eq!(
            clean_for_speech("**bold** `code` [link](http://x.com)"),
            "bold code link"
        );
    }
}
