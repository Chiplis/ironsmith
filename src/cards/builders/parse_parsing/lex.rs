use super::*;

pub(crate) fn tokenize_line(line: &str, line_index: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut buffer = String::new();
    let mut word_start: Option<usize> = None;
    let mut word_end: usize = 0;
    let mut in_mana_braces = false;

    let flush = |buffer: &mut String,
                 tokens: &mut Vec<Token>,
                 word_start: &mut Option<usize>,
                 word_end: &mut usize| {
        if !buffer.is_empty() {
            let start = word_start.unwrap_or(0);
            tokens.push(Token::Word(
                buffer.clone(),
                TextSpan {
                    line: line_index,
                    start,
                    end: *word_end,
                },
            ));
            buffer.clear();
        }
        *word_start = None;
        *word_end = 0;
    };

    let chars: Vec<(usize, char)> = line.char_indices().collect();
    for (idx, (byte_idx, mut ch)) in chars.iter().copied().enumerate() {
        if ch == '−' {
            ch = '-';
        }
        if ch == '{' {
            flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
            in_mana_braces = true;
            continue;
        }
        if ch == '}' {
            flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
            in_mana_braces = false;
            continue;
        }
        let prev = if idx > 0 { chars[idx - 1].1 } else { '\0' };
        let next = if idx + 1 < chars.len() {
            chars[idx + 1].1
        } else {
            '\0'
        };
        let is_counter_char = match ch {
            '+' | '-' => next.is_ascii_digit() || next == 'x' || next == 'X',
            '/' => {
                (prev.is_ascii_digit() || prev == 'x' || prev == 'X')
                    && (next.is_ascii_digit()
                        || next == '-'
                        || next == '+'
                        || next == 'x'
                        || next == 'X')
            }
            _ => false,
        };
        let is_mana_hybrid_slash = ch == '/' && in_mana_braces;

        if ch.is_ascii_alphanumeric() || is_counter_char || is_mana_hybrid_slash {
            if word_start.is_none() {
                word_start = Some(byte_idx);
            }
            word_end = byte_idx + ch.len_utf8();
            buffer.push(ch.to_ascii_lowercase());
            continue;
        }

        if ch == '\'' || ch == '’' || ch == '‘' {
            if word_start.is_some() {
                word_end = byte_idx + ch.len_utf8();
            }
            continue;
        }

        flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);

        let span = TextSpan {
            line: line_index,
            start: byte_idx,
            end: byte_idx + ch.len_utf8(),
        };

        match ch {
            ',' => tokens.push(Token::Comma(span)),
            '.' => tokens.push(Token::Period(span)),
            ':' => tokens.push(Token::Colon(span)),
            ';' => tokens.push(Token::Semicolon(span)),
            _ => {}
        }
    }

    flush(&mut buffer, &mut tokens, &mut word_start, &mut word_end);
    tokens
}

pub(crate) fn parse_metadata_line(line: &str) -> Result<Option<MetadataLine>, CardTextError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let lower = trimmed.to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("mana cost:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::ManaCost(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("type line:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::TypeLine(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("type:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::TypeLine(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("power/toughness:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::PowerToughness(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("loyalty:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::Loyalty(value.to_string())));
    }
    if let Some(rest) = lower.strip_prefix("defense:") {
        let value = trimmed[trimmed.len() - rest.len()..].trim();
        return Ok(Some(MetadataLine::Defense(value.to_string())));
    }

    Ok(None)
}

#[derive(Debug, Clone)]
pub(crate) enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

pub(crate) fn words(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().filter_map(Token::as_word).collect()
}

pub(crate) fn parser_stacktrace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_STACKTRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn parser_trace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_TRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn parser_allow_unsupported_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

pub(crate) fn parser_trace(stage: &str, tokens: &[Token]) {
    if !parser_trace_enabled() {
        return;
    }
    eprintln!(
        "[parser-flow] stage={stage} clause='{}'",
        words(tokens).join(" ")
    );
}

pub(crate) fn parser_trace_line(stage: &str, line: &str) {
    if !parser_trace_enabled() {
        return;
    }
    eprintln!("[parser-flow] stage={stage} line='{}'", line.trim());
}

pub(crate) fn parser_trace_stack(stage: &str, tokens: &[Token]) {
    if !parser_stacktrace_enabled() {
        return;
    }
    eprintln!(
        "[parser-trace] stage={stage} clause='{}'",
        words(tokens).join(" ")
    );
    eprintln!("{}", std::backtrace::Backtrace::force_capture());
}

pub(crate) fn span_from_tokens(tokens: &[Token]) -> Option<TextSpan> {
    let first = tokens.first()?;
    let last = tokens.last()?;
    let start = first.span().start;
    let end = last.span().end;
    Some(TextSpan {
        line: first.span().line,
        start,
        end,
    })
}

#[derive(Debug, Clone)]
pub(crate) struct NormalizedLine {
    pub(crate) original: String,
    pub(crate) normalized: String,
    pub(crate) char_map: Vec<usize>,
}

#[derive(Debug, Clone)]
pub(crate) struct LineInfo {
    pub(crate) line_index: usize,
    pub(crate) raw_line: String,
    pub(crate) normalized: NormalizedLine,
}

pub(crate) fn replace_names_with_map(
    line: &str,
    full_name: &str,
    short_name: &str,
    base_offset: usize,
) -> (String, Vec<usize>) {
    fn has_word_boundaries_at(bytes: &[u8], idx: usize, len: usize) -> bool {
        let is_word = |b: u8| b.is_ascii_alphanumeric();
        let start_ok = if idx == 0 {
            true
        } else {
            !is_word(bytes[idx - 1])
        };
        let end = idx + len;
        let end_ok = if end >= bytes.len() {
            true
        } else {
            !is_word(bytes[end])
        };
        start_ok && end_ok
    }

    fn is_single_word_keyword_verb(name: &str) -> bool {
        !name.contains(' ')
            && matches!(
                name,
                "add"
                    | "move"
                    | "deal"
                    | "draw"
                    | "counter"
                    | "destroy"
                    | "exile"
                    | "untap"
                    | "scry"
                    | "discard"
                    | "transform"
                    | "regenerate"
                    | "mill"
                    | "get"
                    | "reveal"
                    | "lose"
                    | "gain"
                    | "put"
                    | "sacrifice"
                    | "create"
                    | "investigate"
                    | "remove"
                    | "return"
                    | "exchange"
                    | "become"
                    | "switch"
                    | "skip"
                    | "surveil"
                    | "pay"
            )
    }

    fn is_keyword_ability_name(name: &str) -> bool {
        matches!(
            name,
            "flying"
                | "first strike"
                | "double strike"
                | "vigilance"
                | "trample"
                | "haste"
                | "deathtouch"
                | "lifelink"
                | "reach"
                | "defender"
                | "hexproof"
                | "shroud"
                | "menace"
                | "indestructible"
                | "flash"
                | "ward"
        )
    }

    fn preceded_by_named_keyword(bytes: &[u8], mut idx: usize) -> bool {
        while idx > 0 && !bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        let end = idx;
        while idx > 0 && bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        idx < end && &bytes[idx..end] == b"named"
    }

    fn previous_word(bytes: &[u8], mut idx: usize) -> Option<&[u8]> {
        while idx > 0 && !bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        let end = idx;
        while idx > 0 && bytes[idx - 1].is_ascii_alphanumeric() {
            idx -= 1;
        }
        (idx < end).then_some(&bytes[idx..end])
    }

    fn preceded_by_ability_grant_word(bytes: &[u8], idx: usize) -> bool {
        previous_word(bytes, idx)
            .is_some_and(|word| matches!(word, b"has" | b"have" | b"gain" | b"gains"))
    }

    fn token_word_appears_before_sentence_end(bytes: &[u8], mut idx: usize) -> bool {
        while idx < bytes.len() {
            if bytes[idx] == b'.' || bytes[idx] == b';' {
                break;
            }
            if bytes[idx..].starts_with(b"token")
                && has_word_boundaries_at(bytes, idx, "token".len())
            {
                return true;
            }
            if bytes[idx..].starts_with(b"tokens")
                && has_word_boundaries_at(bytes, idx, "tokens".len())
            {
                return true;
            }
            idx += 1;
        }
        false
    }

    fn appears_to_be_created_token_name(bytes: &[u8], idx: usize, name_len: usize) -> bool {
        let Some(prev_word) = previous_word(bytes, idx) else {
            return false;
        };
        if prev_word != b"create" && prev_word != b"creates" {
            return false;
        }
        token_word_appears_before_sentence_end(bytes, idx + name_len)
    }

    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let full_bytes = full_name.as_bytes();
    let short_bytes = short_name.as_bytes();

    let mut out = String::new();
    let mut map = Vec::new();
    let mut idx = 0;

    while idx < bytes.len() {
        if !full_bytes.is_empty()
            && bytes[idx..].starts_with(full_bytes)
            && has_word_boundaries_at(bytes, idx, full_bytes.len())
            && !(idx == 0 && is_single_word_keyword_verb(full_name))
            && !(is_keyword_ability_name(full_name) && preceded_by_ability_grant_word(bytes, idx))
            && !preceded_by_named_keyword(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, full_bytes.len())
        {
            let name_len = full_bytes.len().max(1);
            for j in 0..4 {
                out.push("this".chars().nth(j).unwrap());
                let mapped = base_offset + idx + (j * name_len / 4);
                map.push(mapped);
            }
            idx += full_bytes.len();
            continue;
        }
        if !short_bytes.is_empty()
            && bytes[idx..].starts_with(short_bytes)
            && has_word_boundaries_at(bytes, idx, short_bytes.len())
            && !(idx == 0 && is_single_word_keyword_verb(short_name))
            && !(is_keyword_ability_name(short_name) && preceded_by_ability_grant_word(bytes, idx))
            && !preceded_by_named_keyword(bytes, idx)
            && !appears_to_be_created_token_name(bytes, idx, short_bytes.len())
        {
            let name_len = short_bytes.len().max(1);
            for j in 0..4 {
                out.push("this".chars().nth(j).unwrap());
                let mapped = base_offset + idx + (j * name_len / 4);
                map.push(mapped);
            }
            idx += short_bytes.len();
            continue;
        }

        let ch = lower[idx..].chars().next().unwrap();
        out.push(ch);
        map.push(base_offset + idx);
        idx += ch.len_utf8();
    }

    (out, map)
}

pub(crate) fn strip_parenthetical_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
    let mut out = String::new();
    let mut out_map = Vec::new();
    let mut depth = 0u32;
    let mut char_idx = 0usize;

    for ch in text.chars() {
        if ch == '(' {
            depth += 1;
            char_idx += 1;
            continue;
        }
        if ch == ')' {
            depth = depth.saturating_sub(1);
            char_idx += 1;
            continue;
        }
        if depth == 0 {
            out.push(ch);
            if let Some(mapped) = map.get(char_idx).copied() {
                out_map.push(mapped);
            }
        }
        char_idx += 1;
    }

    (out, out_map)
}

pub(crate) fn is_labeled_ability_word_prefix(prefix: &str) -> bool {
    let words: Vec<&str> = prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() {
        return false;
    }

    if words.len() == 2 && words[0] == "descend" && words[1].chars().all(|ch| ch.is_ascii_digit()) {
        return true;
    }

    if matches!(
        words.as_slice(),
        ["spell", "mastery"]
            | ["totem", "armor"]
            | ["fateful", "hour"]
            | ["join", "forces"]
            | ["pack", "tactics"]
            | ["max", "speed"]
            | ["leading", "from", "the", "front"]
            | ["summary", "execution"]
            | ["will", "of", "the", "council"]
            | ["guardian", "protocols"]
            | ["jolly", "gutpipes"]
            | ["protection", "fighting", "style"]
            | ["relentless", "march"]
            | ["secret", "of", "the", "soul"]
            | ["secrets", "of", "the", "soul"]
            | ["flurry", "of", "blows"]
            | ["gust", "of", "wind"]
            | ["reverberating", "summons"]
    ) {
        return true;
    }

    matches!(
        words[0],
        "adamant"
            | "addendum"
            | "alliance"
            | "ascend"
            | "battalion"
            | "boast"
            | "buyback"
            | "cycling"
            | "bloodrush"
            | "channel"
            | "chroma"
            | "cohort"
            | "constellation"
            | "converge"
            | "corrupted"
            | "coven"
            | "eerie"
            | "equip"
            | "escape"
            | "exhaust"
            | "flashback"
            | "delirium"
            | "domain"
            | "ferocious"
            | "flurry"
            | "formidable"
            | "hellbent"
            | "heroic"
            | "imprint"
            | "inspired"
            | "landfall"
            | "lieutenant"
            | "magecraft"
            | "metalcraft"
            | "morbid"
            | "parley"
            | "partner"
            | "protector"
            | "radiance"
            | "raid"
            | "renew"
            | "replicate"
            | "revolt"
            | "suspend"
            | "spectacle"
            | "strive"
            | "surge"
            | "threshold"
            | "undergrowth"
            | "ward"
    )
}

pub(crate) fn preserve_keyword_prefix_for_parse(prefix: &str) -> bool {
    let words: Vec<&str> = prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    let Some(first) = words.first().copied() else {
        return false;
    };

    matches!(
        first,
        // These are full keyword mechanics that carry their own parseable payload
        // after an em dash. Stripping the prefix corrupts the mechanic line.
        "buyback"
            | "cycling"
            | "equip"
            | "escape"
            | "flashback"
            | "boast"
            | "replicate"
            | "renew"
            | "spectacle"
            | "strive"
            | "surge"
            | "suspend"
            | "ward"
    )
}

pub(crate) fn starts_with_if_clause(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed == "if" || trimmed.starts_with("if ")
}

pub(crate) fn is_generic_ability_label_prefix(prefix: &str) -> bool {
    let words: Vec<&str> = prefix
        .split_whitespace()
        .map(|word| word.trim_matches(|ch: char| !ch.is_ascii_alphanumeric()))
        .filter(|word| !word.is_empty())
        .collect();
    if words.is_empty() || words.len() > 4 {
        return false;
    }

    words.iter().all(|word| {
        word.chars().all(|ch| ch.is_ascii_alphanumeric())
            && word.chars().any(|ch| ch.is_ascii_alphabetic())
    })
}

pub(crate) fn strip_labeled_ability_word_prefix_with_map(
    text: &str,
    map: &[usize],
) -> (String, Vec<usize>) {
    let separator = text
        .find('—')
        .map(|idx| (idx, '—'.len_utf8()))
        .or_else(|| text.find(" - ").map(|idx| (idx, " - ".len())));
    let Some((sep_idx, sep_len)) = separator else {
        return (text.to_string(), map.to_vec());
    };

    let prefix = text[..sep_idx].trim();
    if preserve_keyword_prefix_for_parse(prefix) {
        return (text.to_string(), map.to_vec());
    }

    let mut remainder_start = sep_idx + sep_len;
    while remainder_start < text.len() {
        let ch = text[remainder_start..]
            .chars()
            .next()
            .expect("character must exist");
        if ch.is_whitespace() {
            remainder_start += ch.len_utf8();
        } else {
            break;
        }
    }
    if remainder_start >= text.len() {
        return (text.to_string(), map.to_vec());
    }

    let remainder = text[remainder_start..].to_string();
    let strip_known_label = is_labeled_ability_word_prefix(prefix);
    let strip_generic_conditional_label =
        starts_with_if_clause(&remainder) && is_generic_ability_label_prefix(prefix);
    if !strip_known_label && !strip_generic_conditional_label {
        return (text.to_string(), map.to_vec());
    }

    let remainder_char_start = text[..remainder_start].chars().count();
    let remainder_map = if remainder_char_start < map.len() {
        map[remainder_char_start..].to_vec()
    } else {
        Vec::new()
    };
    (remainder, remainder_map)
}

pub(crate) fn normalize_line_for_parse(
    line: &str,
    full_name: &str,
    short_name: &str,
) -> Option<NormalizedLine> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (replaced, map) = replace_names_with_map(trimmed, full_name, short_name, 0);
    let (label_stripped, label_map) = strip_labeled_ability_word_prefix_with_map(&replaced, &map);
    let (stripped, stripped_map) = strip_parenthetical_with_map(&label_stripped, &label_map);

    if stripped.trim().is_empty() {
        let is_wrapped = trimmed.starts_with('(') && trimmed.ends_with(')');
        if !is_wrapped {
            return None;
        }
        let inner = trimmed.trim_start_matches('(').trim_end_matches(')').trim();
        if inner.is_empty() {
            return None;
        }
        // Parse wrapped parentheticals only when they look like a real ability line
        // (e.g. "({T}: Add ... )"). Mana-symbol reminders like "({W/U} can be paid ...)"
        // should be ignored.
        let should_parse = inner.contains(':');
        if !should_parse {
            return None;
        }
        let base_offset = trimmed.find(inner).unwrap_or(0);
        let (inner_replaced, inner_map) =
            replace_names_with_map(inner, full_name, short_name, base_offset);
        return Some(NormalizedLine {
            original: trimmed.to_string(),
            normalized: inner_replaced,
            char_map: inner_map,
        });
    }

    Some(NormalizedLine {
        original: trimmed.to_string(),
        normalized: stripped,
        char_map: stripped_map,
    })
}

pub(crate) fn is_ignorable_unparsed_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.starts_with('(') && trimmed.ends_with(')')
}

pub(crate) fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
    if byte_idx == 0 {
        return 0;
    }
    let clamped = byte_idx.min(text.len());
    text[..clamped].chars().count()
}

pub(crate) fn map_span_to_original(
    span: TextSpan,
    normalized_line: &str,
    original_line: &str,
    char_map: &[usize],
) -> TextSpan {
    let start_char = byte_to_char_index(normalized_line, span.start);
    let end_char = byte_to_char_index(normalized_line, span.end);
    if start_char >= char_map.len() {
        return span;
    }
    let start_orig = char_map[start_char];
    let end_orig = if end_char == 0 || end_char - 1 >= char_map.len() {
        start_orig
    } else {
        let last_char_idx = end_char - 1;
        let last_orig = char_map[last_char_idx];
        let last_len = original_line[last_orig..]
            .chars()
            .next()
            .map(|ch| ch.len_utf8())
            .unwrap_or(0);
        last_orig + last_len
    };

    TextSpan {
        line: span.line,
        start: start_orig,
        end: end_orig,
    }
}

pub(crate) fn split_on_period(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Period(_)) {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn split_on_comma(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Comma(_)) {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn split_on_comma_or_semicolon(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Comma(_) | Token::Semicolon(_)) {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn split_on_and(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if token.is_word("and") {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(token.clone());
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn is_basic_color_word(word: &str) -> bool {
    matches!(
        word,
        "white" | "blue" | "black" | "red" | "green" | "colorless"
    )
}

pub(crate) fn starts_with_inline_token_rules_tail(words: &[&str]) -> bool {
    words.starts_with(&["when"])
        || words.starts_with(&["whenever"])
        || words.starts_with(&["when", "this", "token"])
        || words.starts_with(&["whenever", "this", "token"])
        || words.starts_with(&["this", "token"])
        || words.starts_with(&["that", "token"])
        || words.starts_with(&["those", "tokens"])
        || words.starts_with(&["except", "it"])
        || words.starts_with(&["except", "they"])
        || words.starts_with(&["except", "its"])
        || words.starts_with(&["except", "their"])
        || words.starts_with(&["this", "creature"])
        || words.starts_with(&["that", "creature"])
        || words.starts_with(&["at", "the", "beginning"])
        || words.starts_with(&["at", "beginning"])
        || words.starts_with(&["sacrifice", "this", "token"])
        || words.starts_with(&["sacrifice", "that", "token"])
        || words.starts_with(&["sacrifice", "this", "permanent"])
        || words.starts_with(&["sacrifice", "that", "permanent"])
        || words.starts_with(&["sacrifice", "it"])
        || words.starts_with(&["sacrifice", "them"])
        || words.starts_with(&["it", "has"])
        || words.starts_with(&["it", "gains"])
        || words.starts_with(&["they", "have"])
        || words.starts_with(&["they", "gain"])
        || words.starts_with(&["equip"])
        || words.starts_with(&["equipped", "creature"])
        || words.starts_with(&["enchanted", "creature"])
        || words.starts_with(&["r"])
        || words.starts_with(&["t"])
}

pub(crate) fn starts_with_inline_token_rules_continuation(words: &[&str]) -> bool {
    matches!(
        words.first().copied(),
        Some(
            "it" | "they"
                | "that"
                | "those"
                | "this"
                | "gain"
                | "gains"
                | "draw"
                | "draws"
                | "add"
                | "deal"
                | "deals"
                | "destroy"
                | "destroys"
                | "exile"
                | "exiles"
                | "return"
                | "returns"
                | "tap"
                | "untap"
                | "sacrifice"
                | "create"
                | "put"
                | "fights"
                | "fight"
        )
    )
}

pub(crate) fn is_token_creation_context(words: &[&str]) -> bool {
    words.first().copied() == Some("create")
        && words.iter().any(|word| matches!(*word, "token" | "tokens"))
}

pub(crate) fn has_inline_token_rules_context(words: &[&str]) -> bool {
    words.windows(3).any(|window| {
        matches!(
            window,
            ["when", "this", "token"] | ["whenever", "this", "token"]
        )
    }) || words
        .windows(4)
        .any(|window| window == ["at", "the", "beginning", "of"])
        || (words.contains(&"except") && words.contains(&"copy") && words.contains(&"token"))
}

pub(crate) fn should_keep_and_for_token_rules(current: &[Token], remaining: &[Token]) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = words(current);
    if current_words.is_empty() {
        return false;
    }
    if !is_token_creation_context(&current_words) && !has_inline_token_rules_context(&current_words)
    {
        return false;
    }
    let remaining_words = words(remaining);
    starts_with_inline_token_rules_tail(&remaining_words)
}

pub(crate) fn should_keep_and_for_attachment_object_list(
    current: &[Token],
    remaining: &[Token],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = words(current);
    let remaining_words = words(remaining);
    if current_words.is_empty() || remaining_words.is_empty() {
        return false;
    }

    let starts_attachment_subject = remaining_words.first().is_some_and(|word| {
        matches!(
            *word,
            "aura"
                | "auras"
                | "equipment"
                | "equipments"
                | "enchantment"
                | "enchantments"
                | "artifact"
                | "artifacts"
        )
    });
    if !starts_attachment_subject || !remaining_words.contains(&"attached") {
        return false;
    }

    current_words.starts_with(&["destroy", "all"])
        || current_words.starts_with(&["exile", "all"])
        || current_words.starts_with(&["gain", "control", "of", "all"])
}

pub(crate) fn should_keep_and_for_each_player_may_clause(
    current: &[Token],
    remaining: &[Token],
) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = words(current);
    if current_words.is_empty() || !current_words.contains(&"may") {
        return false;
    }

    let starts_for_each_player_or_opponent = current_words.starts_with(&["each", "player"])
        || current_words.starts_with(&["each", "players"])
        || current_words.starts_with(&["each", "opponent"])
        || current_words.starts_with(&["each", "opponents"])
        || current_words.starts_with(&["for", "each", "player"])
        || current_words.starts_with(&["for", "each", "players"])
        || current_words.starts_with(&["for", "each", "opponent"])
        || current_words.starts_with(&["for", "each", "opponents"]);
    if !starts_for_each_player_or_opponent {
        return false;
    }

    let remaining_words = words(remaining);
    if remaining_words.is_empty() {
        return false;
    }
    if remaining_words.starts_with(&["for", "each"]) || remaining_words.starts_with(&["each"]) {
        return false;
    }

    true
}

pub(crate) fn should_keep_and_for_put_rest_clause(current: &[Token], remaining: &[Token]) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }

    let current_words = words(current);
    let remaining_words = words(remaining);
    if current_words.is_empty() || remaining_words.is_empty() {
        return false;
    }

    let starts_with_rest =
        remaining_words.starts_with(&["the", "rest"]) || remaining_words.starts_with(&["rest"]);
    if !starts_with_rest {
        return false;
    }

    // Keep "and the rest ..." attached to a preceding put-into-hand clause so
    // multi-destination patterns parse as one effect.
    current_words.contains(&"put")
        && current_words.contains(&"into")
        && current_words.contains(&"hand")
}

pub(crate) fn split_effect_chain_on_and(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("and") {
            let prev_word = current.last().and_then(Token::as_word);
            let next_word = tokens.get(idx + 1).and_then(Token::as_word);
            let is_color_pair = prev_word.zip(next_word).is_some_and(|(left, right)| {
                is_basic_color_word(left) && is_basic_color_word(right)
            });
            if is_color_pair
                || should_keep_and_for_token_rules(&current, &tokens[idx + 1..])
                || should_keep_and_for_attachment_object_list(&current, &tokens[idx + 1..])
                || should_keep_and_for_each_player_may_clause(&current, &tokens[idx + 1..])
                || should_keep_and_for_put_rest_clause(&current, &tokens[idx + 1..])
            {
                current.push(token.clone());
                continue;
            }
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn has_effect_head_without_verb(tokens: &[Token]) -> bool {
    parse_prevent_next_damage_clause(tokens)
        .ok()
        .flatten()
        .is_some()
        || parse_prevent_all_damage_clause(tokens)
            .ok()
            .flatten()
            .is_some()
        || parse_can_attack_as_though_no_defender_clause(tokens)
            .ok()
            .flatten()
            .is_some()
}

pub(crate) fn segment_has_effect_head(tokens: &[Token]) -> bool {
    find_verb(tokens).is_some() || has_effect_head_without_verb(tokens)
}

pub(crate) fn join_sentences_with_period(sentences: &[Vec<Token>]) -> Vec<Token> {
    let mut joined = Vec::new();
    for (idx, sentence) in sentences.iter().enumerate() {
        if idx > 0 {
            joined.push(Token::Period(TextSpan::synthetic()));
        }
        joined.extend(sentence.clone());
    }
    joined
}

/// Splits segments on ", then" when the part after "then" is an independent
/// clause (doesn't back-reference the first part with "that", "it", "them", "its").
/// This handles patterns like "discard your hand, then draw four cards" without
/// breaking cross-referencing patterns like "exile X, then return that card".
pub(crate) fn split_segments_on_comma_then(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    let back_ref_words = ["that", "it", "them", "its"];
    let mut result = Vec::new();
    for segment in segments {
        let segment_words = words(&segment);
        let starts_with_for_each_player_or_opponent = segment_words
            .starts_with(&["each", "player"])
            || segment_words.starts_with(&["each", "players"])
            || segment_words.starts_with(&["each", "opponent"])
            || segment_words.starts_with(&["each", "opponents"])
            || segment_words.starts_with(&["for", "each", "player"])
            || segment_words.starts_with(&["for", "each", "players"])
            || segment_words.starts_with(&["for", "each", "opponent"])
            || segment_words.starts_with(&["for", "each", "opponents"]);
        let mut split_point = None;
        for i in 0..segment.len().saturating_sub(1) {
            if matches!(segment[i], Token::Comma(_))
                && segment.get(i + 1).is_some_and(|t| t.is_word("then"))
            {
                let before_then = &segment[..i];
                let before_words = words(before_then);
                let starts_with_clash =
                    before_words.starts_with(&["clash"]) || before_words.starts_with(&["clashes"]);
                let after_then = &segment[i + 2..];
                let after_words = words(after_then);
                let has_back_ref = after_words.iter().any(|w| back_ref_words.contains(w));
                let has_nonverb_effect_head = after_then
                    .first()
                    .and_then(Token::as_word)
                    .is_some_and(|word| {
                        matches!(
                            word,
                            "double"
                                | "distribute"
                                | "support"
                                | "bolster"
                                | "adapt"
                                | "open"
                                | "manifest"
                                | "connive"
                                | "earthbend"
                        )
                    });
                let has_effect_head = find_verb(after_then).is_some()
                    || parse_ability_line(after_then).is_some()
                    || has_nonverb_effect_head;
                let allow_backref_split = has_back_ref
                    && after_words.first().is_some_and(|word| *word == "put")
                    && after_words
                        .iter()
                        .any(|word| *word == "counter" || *word == "counters");
                let allow_attach_followup = after_words
                    .first()
                    .is_some_and(|word| matches!(*word, "attach" | "attaches"));
                let allow_that_many_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && (after_words.starts_with(&["draw", "that", "many"])
                        || after_words.starts_with(&["draws", "that", "many"])
                        || after_words.starts_with(&["create", "that", "many"])
                        || after_words.starts_with(&["creates", "that", "many"]));
                let allow_gain_or_lose_life_equal_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && (after_words.starts_with(&["gain", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["gains", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["lose", "life", "equal", "to", "that"])
                            || after_words.starts_with(&["loses", "life", "equal", "to", "that"]));
                let allow_deal_damage_equal_power_followup =
                    !starts_with_for_each_player_or_opponent
                        && has_back_ref
                        && (after_words.starts_with(&["it", "deal", "damage", "equal", "to"])
                            || after_words.starts_with(&["it", "deals", "damage", "equal", "to"])
                            || after_words.starts_with(&[
                                "that", "creature", "deal", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "creature", "deals", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "objects", "deal", "damage", "equal", "to",
                            ])
                            || after_words.starts_with(&[
                                "that", "objects", "deals", "damage", "equal", "to",
                            ]));
                let allow_for_each_damage_followup = has_back_ref
                    && (after_words.starts_with(&["each"])
                        || after_words.starts_with(&["for", "each"]))
                    && after_words
                        .iter()
                        .any(|word| *word == "deal" || *word == "deals")
                    && after_words.iter().any(|word| *word == "damage");
                let allow_return_with_counter_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && after_words.first().is_some_and(|word| *word == "return")
                    && after_words
                        .iter()
                        .any(|word| *word == "counter" || *word == "counters")
                    && after_words
                        .windows(2)
                        .any(|window| window == ["on", "it"] || window == ["on", "them"]);
                let allow_put_into_hand_followup = has_back_ref
                    && (after_words.starts_with(&["put"]) || after_words.starts_with(&["puts"]))
                    && after_words.contains(&"into")
                    && after_words.contains(&"hand");
                let allow_put_back_in_any_order_followup = has_back_ref
                    && (after_words.starts_with(&["put", "it", "back"])
                        || after_words.starts_with(&["put", "them", "back"])
                        || after_words.starts_with(&["puts", "it", "back"])
                        || after_words.starts_with(&["puts", "them", "back"]))
                    && after_words.contains(&"any")
                    && after_words.contains(&"order");
                let allow_clash_followup = starts_with_clash;
                if has_effect_head && (!has_back_ref || allow_backref_split) {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_clash_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_attach_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_that_many_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_gain_or_lose_life_equal_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_deal_damage_equal_power_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_for_each_damage_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_return_with_counter_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_put_into_hand_followup {
                    split_point = Some(i);
                    break;
                } else if has_effect_head && allow_put_back_in_any_order_followup {
                    split_point = Some(i);
                    break;
                }
            }
        }
        if let Some(idx) = split_point {
            let first_part = segment[..idx].to_vec();
            let second_part = segment[idx + 2..].to_vec(); // skip comma and "then"
            if !first_part.is_empty() {
                result.push(first_part);
            }
            if !second_part.is_empty() {
                result.push(second_part);
            }
        } else {
            result.push(segment);
        }
    }
    result
}

pub(crate) fn split_segments_on_comma_effect_head(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    let mut result = Vec::new();
    for segment in segments {
        let mut start = 0usize;
        let mut split_any = false;

        for idx in 0..segment.len() {
            if !matches!(segment[idx], Token::Comma(_)) {
                continue;
            }
            let before = trim_commas(&segment[start..idx]);
            let after = trim_commas(&segment[idx + 1..]);
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let before_has_verb = find_verb(before.as_slice()).is_some();
            let after_starts_effect = find_verb(after.as_slice())
                .is_some_and(|(_, verb_idx)| verb_idx == 0)
                || has_effect_head_without_verb(after.as_slice());
            let before_words = words(before.as_slice());
            let after_words = words(after.as_slice());
            let duration_trigger_prefix = (before_words.first() == Some(&"until")
                || before_words.first() == Some(&"during"))
                && (before_words.contains(&"whenever")
                    || before_words.contains(&"when")
                    || before_words
                        .windows(2)
                        .any(|window| window == ["at", "the"]));
            if before_words.first() == Some(&"unless") {
                continue;
            }
            // Keep "until ... whenever ..., <effect>" in one segment so the duration-triggered
            // clause parser can consume the full trigger+effect text.
            if duration_trigger_prefix {
                continue;
            }
            // Keep search instructions in a single segment so the dedicated
            // search-library parser can see "reveal/put/shuffle" tails.
            if before_words.contains(&"search") && before_words.contains(&"library") {
                continue;
            }
            let is_inline_token_rules_split = (is_token_creation_context(&before_words)
                || has_inline_token_rules_context(&before_words))
                && (starts_with_inline_token_rules_tail(&after_words)
                    || starts_with_inline_token_rules_continuation(&after_words));
            if is_inline_token_rules_split {
                continue;
            }
            if !before_has_verb || !after_starts_effect {
                continue;
            }

            let part = before.to_vec();
            if !part.is_empty() {
                result.push(part);
                split_any = true;
            }
            start = idx + 1;
        }

        let tail = trim_commas(&segment[start..]).to_vec();
        if !tail.is_empty() {
            result.push(tail);
        } else if !split_any && !segment.is_empty() {
            result.push(segment);
        }
    }
    result
}

pub(crate) fn split_cost_segments(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Comma(_)) || token.is_word("and") {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }

    if !current.is_empty() {
        segments.push(current);
    }

    segments
}

pub(crate) fn alternative_cast_parts_from_total_cost(
    total_cost: &crate::cost::TotalCost,
) -> (Option<ManaCost>, Vec<Effect>) {
    let mut mana_cost: Option<ManaCost> = None;
    let mut cost_effects = Vec::new();

    for cost in total_cost.costs() {
        if let Some(mana) = cost.mana_cost_ref() {
            if mana_cost.is_none() {
                mana_cost = Some(mana.clone());
            }
            continue;
        }
        if let Some(effect) = cost.effect_ref() {
            cost_effects.push(effect.clone());
            continue;
        }
        if cost.is_life_cost() {
            if let Some(amount) = cost.life_amount() {
                cost_effects.push(Effect::pay_life(amount));
            }
            continue;
        }
        if cost.is_discard() {
            let (count, card_types) = match cost.processing_mode() {
                crate::costs::CostProcessingMode::DiscardCards { count, card_types } => {
                    (count, card_types)
                }
                _ => {
                    let (count, card_type) = cost.discard_details().unwrap_or((1, None));
                    (count, card_type.into_iter().collect())
                }
            };
            let card_filter = if card_types.is_empty() {
                None
            } else {
                Some(ObjectFilter {
                    card_types,
                    ..Default::default()
                })
            };
            cost_effects.push(Effect::discard_player_filtered(
                Value::Fixed(count as i32),
                PlayerFilter::You,
                false,
                card_filter,
            ));
            continue;
        }
        if cost.is_exile_from_hand() {
            if let Some((count, color_filter)) = cost.exile_from_hand_details() {
                cost_effects.push(Effect::exile_from_hand_as_cost(count, color_filter));
            }
            continue;
        }
        if cost.is_sacrifice_self() {
            cost_effects.push(Effect::sacrifice_source());
            continue;
        }
    }

    (
        mana_cost,
        normalize_alternative_cast_cost_effects(cost_effects),
    )
}

pub(crate) fn normalize_alternative_cast_cost_effects(cost_effects: Vec<Effect>) -> Vec<Effect> {
    use crate::filter::TaggedOpbjectRelation;

    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < cost_effects.len() {
        if idx + 1 < cost_effects.len()
            && let Some(choose) =
                cost_effects[idx].downcast_ref::<crate::effects::ChooseObjectsEffect>()
            && let Some(sacrifice) =
                cost_effects[idx + 1].downcast_ref::<crate::effects::SacrificeEffect>()
            && sacrifice.player == PlayerFilter::You
        {
            let references_chosen = sacrifice.filter.tagged_constraints.len() == 1
                && sacrifice.filter.tagged_constraints[0].tag == choose.tag
                && sacrifice.filter.tagged_constraints[0].relation
                    == TaggedOpbjectRelation::IsTaggedObject;
            if references_chosen {
                out.push(Effect::sacrifice(
                    choose.filter.clone(),
                    sacrifice.count.clone(),
                ));
                idx += 2;
                continue;
            }
        }

        out.push(cost_effects[idx].clone());
        idx += 1;
    }

    out
}

pub(crate) fn parse_mana_output_options_tokens(
    tokens: &[Token],
) -> Result<Vec<Vec<ManaSymbol>>, CardTextError> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Comma(_)) || token.is_word("or") {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        segments.push(current);
    }
    if segments.is_empty() {
        segments.push(tokens.to_vec());
    }

    let mut options: Vec<Vec<ManaSymbol>> = Vec::new();
    for segment in segments {
        let segment_words = words(&segment);
        let mut groups: Vec<Vec<ManaSymbol>> = Vec::new();
        for token in &segment {
            let Some(word) = token.as_word() else {
                continue;
            };
            if matches!(word, "mana" | "to" | "your" | "pool" | "and") {
                continue;
            }
            if word.contains('/') {
                groups.push(parse_mana_symbol_group(word)?);
                continue;
            }
            if let Ok(symbol) = parse_mana_symbol(word) {
                groups.push(vec![symbol]);
            }
        }
        if groups.is_empty() {
            if segment_words.is_empty() {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported mana output option segment (clause: '{}')",
                words(tokens).join(" ")
            )));
        }

        let mut expanded = vec![Vec::new()];
        for group in groups {
            let mut next = Vec::new();
            for partial in &expanded {
                for symbol in &group {
                    let mut option = partial.clone();
                    option.push(*symbol);
                    next.push(option);
                }
            }
            expanded = next;
        }
        for option in expanded {
            if !options.contains(&option) {
                options.push(option);
            }
        }
    }

    Ok(options)
}

pub(crate) fn parse_mana_output_options_for_line(
    line: &str,
    line_index: usize,
) -> Result<Option<Vec<Vec<ManaSymbol>>>, CardTextError> {
    let tokens = tokenize_line(line, line_index);
    let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    else {
        return Ok(None);
    };
    let effect_tokens = &tokens[colon_idx + 1..];
    let sentences = split_on_period(effect_tokens);
    let Some(primary_sentence) = sentences.first() else {
        return Ok(None);
    };
    let Some(add_idx) = primary_sentence
        .iter()
        .position(|token| token.is_word("add"))
    else {
        return Ok(None);
    };
    let output_tokens = &primary_sentence[add_idx + 1..];
    let has_explicit_symbols = output_tokens.iter().any(|token| {
        let Some(word) = token.as_word() else {
            return false;
        };
        if parse_mana_symbol(word).is_ok() {
            return true;
        }
        word.contains('/') && parse_mana_symbol_group(word).is_ok()
    });
    if !has_explicit_symbols {
        return Ok(None);
    }

    let options = parse_mana_output_options_tokens(output_tokens)?;
    if options.is_empty() {
        return Ok(None);
    }
    Ok(Some(options))
}

pub(crate) fn parse_saga_chapter_prefix(line: &str) -> Option<(Vec<u32>, &str)> {
    let (prefix, rest) = line.split_once('—').or_else(|| line.split_once(" - "))?;

    let mut chapters = Vec::new();
    for part in prefix.split(',') {
        let roman = part.trim();
        if roman.is_empty() {
            continue;
        }
        let value = roman_to_int(roman)?;
        chapters.push(value);
    }

    if chapters.is_empty() {
        return None;
    }

    Some((chapters, rest.trim()))
}

pub(crate) fn roman_to_int(roman: &str) -> Option<u32> {
    match roman {
        "i" => Some(1),
        "ii" => Some(2),
        "iii" => Some(3),
        "iv" => Some(4),
        "v" => Some(5),
        "vi" => Some(6),
        _ => None,
    }
}

pub(crate) fn parse_level_header(line: &str) -> Option<(u32, Option<u32>)> {
    let lower = line.trim().to_ascii_lowercase();
    let rest = lower.strip_prefix("level ")?;
    let token = rest.split_whitespace().next()?;
    if let Some(without_plus) = token.strip_suffix('+') {
        let min = without_plus.parse::<u32>().ok()?;
        return Some((min, None));
    }
    if let Some((start, end)) = token.split_once('-') {
        let min = start.parse::<u32>().ok()?;
        let max = end.parse::<u32>().ok()?;
        return Some((min, Some(max)));
    }
    let value = token.parse::<u32>().ok()?;
    Some((value, Some(value)))
}

pub(crate) fn is_untap_during_each_other_players_untap_step_words(words: &[&str]) -> bool {
    if words.first().copied() != Some("untap") {
        return false;
    }
    words.windows(6).any(|window| {
        window == ["during", "each", "other", "player", "untap", "step"]
            || window == ["during", "each", "other", "players", "untap", "step"]
    })
}

pub(crate) fn is_non_mana_additional_cost_modifier_line(normalized_line: &str) -> bool {
    let has_additional_cost = normalized_line.contains(" cost an additional ")
        || normalized_line.contains(" costs an additional ");
    if !has_additional_cost {
        return false;
    }
    let has_activation_or_cast_tail =
        normalized_line.contains(" to activate") || normalized_line.contains(" to cast");
    if !has_activation_or_cast_tail {
        return false;
    }
    normalized_line.contains('"') || normalized_line.contains('“') || normalized_line.contains('”')
}

pub(crate) fn dash_labeled_remainder_starts_with_trigger(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    let remainder = lower
        .split_once('—')
        .map(|(_, rest)| rest.trim())
        .or_else(|| lower.split_once(" - ").map(|(_, rest)| rest.trim()));
    let Some(rest) = remainder else {
        return false;
    };
    rest.starts_with("whenever ") || rest.starts_with("when ") || rest.starts_with("at ")
}
