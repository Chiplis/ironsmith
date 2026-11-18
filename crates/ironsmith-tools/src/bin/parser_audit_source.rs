//! Mask inline tests and non-code text while retaining report line numbers.

pub(super) fn production_source(source: &str) -> String {
    blank_test_modules(&blank_comments_and_literals(source))
}

/// Replace comments and string/char literal contents with spaces, keeping
/// every byte offset and newline where it was.
fn blank_comments_and_literals(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut out = bytes.to_vec();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    out[i] = b' ';
                    i += 1;
                }
            }
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'*' => {
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    if bytes[i] != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
                i += 2;
            }
            b'"' => {
                let raw_hashes = raw_string_hashes(bytes, i);
                i += 1;
                loop {
                    if i >= bytes.len() {
                        break;
                    }
                    if bytes[i] == b'\\' && raw_hashes.is_none() {
                        out[i] = b' ';
                        if i + 1 < bytes.len() && bytes[i + 1] != b'\n' {
                            out[i + 1] = b' ';
                        }
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'"' {
                        let closes = match raw_hashes {
                            Some(hashes) => {
                                bytes[i + 1..]
                                    .iter()
                                    .take_while(|byte| **byte == b'#')
                                    .count()
                                    >= hashes
                            }
                            None => true,
                        };
                        if closes {
                            break;
                        }
                    }
                    if bytes[i] != b'\n' {
                        out[i] = b' ';
                    }
                    i += 1;
                }
            }
            b'\'' => {
                // a char literal is 'x' or '\x'; anything else is a lifetime
                if i + 2 < bytes.len() && bytes[i + 2] == b'\'' {
                    out[i + 1] = b' ';
                    i += 2;
                } else if i + 3 < bytes.len() && bytes[i + 1] == b'\\' && bytes[i + 3] == b'\'' {
                    out[i + 1] = b' ';
                    out[i + 2] = b' ';
                    i += 3;
                }
            }
            _ => {}
        }
        i += 1;
    }
    String::from_utf8(out).expect("blanking keeps the source valid UTF-8")
}

/// For a `"` at `quote`, the number of hashes if it opens a raw string.
fn raw_string_hashes(bytes: &[u8], quote: usize) -> Option<usize> {
    let mut i = quote;
    let mut hashes = 0usize;
    while i > 0 && bytes[i - 1] == b'#' {
        hashes += 1;
        i -= 1;
    }
    (i > 0
        && bytes[i - 1] == b'r'
        && (i == 1 || !(bytes[i - 2].is_ascii_alphanumeric() || bytes[i - 2] == b'_')))
        .then_some(hashes)
}

/// Blank every `#[cfg(test)] mod name { ... }` block to spaces.
fn blank_test_modules(text: &str) -> String {
    let mut out = text.as_bytes().to_vec();
    let mut at = 0usize;
    while let Some(offset) = text[at..].find("#[cfg(test)]") {
        let start = at + offset;
        at = start + 12;
        let rest = &text[at..];
        let trimmed = rest.trim_start();
        let Some(after_mod) = trimmed
            .strip_prefix("pub(crate) mod ")
            .or_else(|| trimmed.strip_prefix("pub(super) mod "))
            .or_else(|| trimmed.strip_prefix("pub mod "))
            .or_else(|| trimmed.strip_prefix("mod "))
        else {
            continue;
        };
        let Some(brace_offset) = after_mod.find('{') else {
            continue;
        };
        if after_mod[..brace_offset].contains(';') {
            continue;
        }
        let open = text.len() - after_mod.len() + brace_offset;
        let Some(close) = matching_brace(text.as_bytes(), open) else {
            continue;
        };
        for byte in &mut out[start..=close] {
            if *byte != b'\n' {
                *byte = b' ';
            }
        }
        at = close + 1;
    }
    String::from_utf8(out).expect("blanking keeps the source valid UTF-8")
}

pub(super) fn matching_brace(bytes: &[u8], open: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut i = open;
    while i < bytes.len() {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
