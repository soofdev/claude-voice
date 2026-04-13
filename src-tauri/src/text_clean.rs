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
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '`' => {
                if let Some(end) = line[i + 1..].find('`') {
                    s.push_str(&line[i + 1..i + 1 + end]);
                    i += end + 2;
                    continue;
                }
            }
            '[' => {
                if let Some(close) = line[i + 1..].find("](") {
                    let label = &line[i + 1..i + 1 + close];
                    if let Some(end) = line[i + close + 3..].find(')') {
                        s.push_str(label);
                        i += close + 3 + end + 1;
                        continue;
                    }
                }
            }
            '*' | '_' | '#' | '>' | '~' => {
                i += 1;
                continue;
            }
            _ => {}
        }
        s.push(c);
        i += 1;
    }
    s
}
