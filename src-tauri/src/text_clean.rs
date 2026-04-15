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
