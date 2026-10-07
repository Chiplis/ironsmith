//! Shared source-name ownership across coordinated quoted grants.
#[derive(Debug, Clone, Copy)]
pub struct AttachmentGrantQuote { pub start: usize, pub end: usize, pub labeled: bool }

pub fn attachment_grant_header(words: &[&str]) -> bool {
    words.len() >= 2 && matches!(words.last(), Some(&"has" | &"have"))
}

pub fn attachment_grant_quote_scopes(text: &str) -> Vec<AttachmentGrantQuote> {
    let quotes = text.char_indices().filter(|(_, ch)| matches!(ch, '"' | '\u{201c}' | '\u{201d}')).collect::<Vec<_>>();
    let mut scopes = Vec::<AttachmentGrantQuote>::new();
    let mut previous_close = 0;
    let mut previous_grant = None;
    for pair in quotes.chunks_exact(2) {
        let (open, open_char) = pair[0]; let (close, close_char) = pair[1];
        let prefix = &text[previous_close..open];
        let header = prefix.rsplit(['.', ';', '\n']).next().unwrap_or(prefix).trim();
        let (header, labeled) = header.rsplit_once(" — ").map_or((header, false), |(_, rest)| (rest.trim(), true));
        let words = header.split_whitespace().collect::<Vec<_>>();
        let explicit = attachment_grant_header(&words);
        let coordinated = matches!(header.trim_matches(|ch: char| ch == ',' || ch.is_whitespace()), "and" | "");
        let grant = if explicit { Some(labeled) } else if coordinated { previous_grant } else { None };
        if let Some(labeled) = grant { scopes.push(AttachmentGrantQuote { start: open + open_char.len_utf8(), end: close, labeled }); }
        previous_grant = grant; previous_close = close + close_char.len_utf8();
    }
    scopes
}

pub fn attachment_grant_name_is_operand(previous: Option<&str>, before_previous: Option<&str>, colon_follows: bool) -> bool {
    if matches!(previous, Some("on" | "from")) && matches!(before_previous, Some("counter" | "counters")) { return true; }
    match previous {
        Some("sacrifice" | "tap" | "untap") => true,
        Some("return" | "exile" | "destroy" | "remove" | "fight" | "fights") => !colon_follows,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coordinated_grants_keep_one_origin_but_a_new_statement_does_not() {
        let text = "equipped creature has \"{t}: put an aim counter on equipment\" and \"{t}, remove all aim counters from equipment: deal damage\". other text \"equipment\"";
        let scopes = attachment_grant_quote_scopes(text); assert_eq!(scopes.len(), 2);
        assert!(text[scopes[1].start..scopes[1].end].contains("from equipment"));
        assert!(attachment_grant_name_is_operand(Some("from"), Some("counters"), true));
        assert!(!attachment_grant_name_is_operand(Some("return"), None, true));
    }
}
