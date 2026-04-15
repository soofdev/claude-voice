use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub label: String,
    pub url: String,
}

pub struct Extracted {
    pub text: String,
    pub links: Vec<Link>,
}

pub fn extract(original: &str) -> Extracted {
    let mut out = String::with_capacity(original.len());
    let mut links: Vec<Link> = Vec::new();
    let mut in_code_block = false;

    for line in original.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            out.push('\n');
            continue;
        }
        if in_code_block {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        out.push_str(&process_line(line, &mut links));
        out.push('\n');
    }

    dedupe(&mut links);
    Extracted {
        text: out.trim_end().to_string(),
        links,
    }
}

fn process_line(line: &str, links: &mut Vec<Link>) -> String {
    let mut s = String::with_capacity(line.len());
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < line.len() {
        let rest = &line[i..];

        if rest.starts_with('[') {
            if let Some((label, url, consumed)) = parse_md_link(rest) {
                links.push(Link {
                    label: label.clone(),
                    url,
                });
                s.push_str(&label);
                i += consumed;
                continue;
            }
        }

        if rest.starts_with("http://") || rest.starts_with("https://") {
            if let Some((url, consumed)) = parse_bare_url(rest) {
                let label = friendly_label(&url);
                s.push_str(&label);
                links.push(Link { label, url });
                i += consumed;
                continue;
            }
        }

        let c = bytes[i] as char;
        if c.is_ascii() {
            s.push(c);
            i += 1;
        } else {
            let ch = rest.chars().next().unwrap();
            s.push(ch);
            i += ch.len_utf8();
        }
    }
    s
}

fn parse_md_link(rest: &str) -> Option<(String, String, usize)> {
    if !rest.starts_with('[') {
        return None;
    }
    let label_close = rest[1..].find(']')?;
    let after_label = 1 + label_close + 1;
    if rest.as_bytes().get(after_label) != Some(&b'(') {
        return None;
    }
    let paren_start = after_label + 1;
    let paren_close = rest[paren_start..].find(')')?;
    let label = rest[1..1 + label_close].to_string();
    let url = rest[paren_start..paren_start + paren_close]
        .trim()
        .to_string();
    let consumed = paren_start + paren_close + 1;
    if label.is_empty() || !is_valid_url(&url) {
        return None;
    }
    Some((label, url, consumed))
}

fn parse_bare_url(rest: &str) -> Option<(String, usize)> {
    let end = rest
        .find(|c: char| c.is_whitespace() || matches!(c, ')' | ']' | '>' | '"' | '\'' | '`'))
        .unwrap_or(rest.len());
    let mut url = rest[..end].to_string();
    while url
        .chars()
        .last()
        .map(|c| matches!(c, '.' | ',' | ';' | ':' | '!' | '?'))
        .unwrap_or(false)
    {
        url.pop();
    }
    if !is_valid_url(&url) {
        return None;
    }
    let len = url.len();
    Some((url, len))
}

fn is_valid_url(url: &str) -> bool {
    if url.is_empty() {
        return false;
    }
    for c in url.chars() {
        if c.is_whitespace() {
            return false;
        }
        if matches!(
            c,
            '\u{2026}' | '\u{2018}' | '\u{2019}' | '\u{201C}' | '\u{201D}' | '`' | '{' | '}' | '<' | '>' | '|' | '^'
        ) {
            return false;
        }
    }

    if let Some(rest) = url.strip_prefix("mailto:") {
        if let Some((local, domain)) = rest.split_once('@') {
            return !local.is_empty() && is_valid_host(domain);
        }
        return false;
    }

    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"));
    let Some(rest) = rest else {
        return false;
    };

    let host: &str = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");

    is_valid_host(host)
}

fn is_valid_host(host: &str) -> bool {
    if host.len() < 3 || !host.contains('.') {
        return false;
    }
    if host.starts_with('.') || host.ends_with('.') || host.starts_with('-') {
        return false;
    }
    let host_no_port = host.split(':').next().unwrap_or(host);
    if !host_no_port
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    {
        return false;
    }
    let tld = host_no_port.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 || !tld.chars().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    true
}

fn friendly_label(url: &str) -> String {
    let without_scheme = url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let host = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .trim_start_matches("www.");
    format!("({host} link)")
}

fn dedupe(links: &mut Vec<Link>) {
    let mut seen = std::collections::HashSet::new();
    links.retain(|l| seen.insert(l.url.clone()));
}
