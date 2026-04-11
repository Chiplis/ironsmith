pub(crate) fn strip_parenthetical_text(text: &str) -> String {
    text.lines()
        .map(strip_parenthetical_text_from_line)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn strip_parenthetical_text_from_line(line: &str) -> String {
    let mut stripped = String::with_capacity(line.len());
    let mut depth = 0usize;

    for ch in line.chars() {
        match ch {
            '(' => {
                if depth == 0 {
                    while stripped.ends_with([' ', '\t']) {
                        stripped.pop();
                    }
                }
                depth += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
            }
            _ if depth == 0 => stripped.push(ch),
            _ => {}
        }
    }

    tighten_spacing(&stripped)
}

fn tighten_spacing(line: &str) -> String {
    let collapsed = line
        .replace('\u{00a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut tightened = String::with_capacity(collapsed.len());
    for ch in collapsed.chars() {
        if matches!(ch, '.' | ',' | ';' | ':' | '!' | '?') && tightened.ends_with(' ') {
            tightened.pop();
        }
        tightened.push(ch);
    }
    tightened.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::strip_parenthetical_text;

    #[test]
    fn strips_keyword_reminder_text_and_annotation_only_lines() {
        let text = "Flying (This creature can't be blocked except by creatures with flying or reach.)\n(Transforms from Conqueror's Galleon.)";
        assert_eq!(strip_parenthetical_text(text), "Flying");
    }

    #[test]
    fn strips_inline_gloss_without_leaving_space_before_punctuation() {
        let text = "You get {E}{E} (two energy counters).";
        assert_eq!(strip_parenthetical_text(text), "You get {E}{E}.");
    }

    #[test]
    fn strips_nested_parenthetical_text() {
        let text = "Foo (bar (baz)) qux.";
        assert_eq!(strip_parenthetical_text(text), "Foo qux.");
    }
}
