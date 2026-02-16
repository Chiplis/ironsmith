fn tokenize_line(line: &str, line_index: usize) -> Vec<Token> {
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

fn parse_metadata_line(line: &str) -> Result<Option<MetadataLine>, CardTextError> {
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
enum MetadataLine {
    ManaCost(String),
    TypeLine(String),
    PowerToughness(String),
    Loyalty(String),
    Defense(String),
}

fn words(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().filter_map(Token::as_word).collect()
}

fn parser_stacktrace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_STACKTRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn parser_trace_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_TRACE")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn parser_allow_unsupported_enabled() -> bool {
    std::env::var("IRONSMITH_PARSER_ALLOW_UNSUPPORTED")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn parser_trace(stage: &str, tokens: &[Token]) {
    if !parser_trace_enabled() {
        return;
    }
    eprintln!(
        "[parser-flow] stage={stage} clause='{}'",
        words(tokens).join(" ")
    );
}

fn parser_trace_line(stage: &str, line: &str) {
    if !parser_trace_enabled() {
        return;
    }
    eprintln!("[parser-flow] stage={stage} line='{}'", line.trim());
}

fn parser_trace_stack(stage: &str, tokens: &[Token]) {
    if !parser_stacktrace_enabled() {
        return;
    }
    eprintln!(
        "[parser-trace] stage={stage} clause='{}'",
        words(tokens).join(" ")
    );
    eprintln!("{}", std::backtrace::Backtrace::force_capture());
}

fn span_from_tokens(tokens: &[Token]) -> Option<TextSpan> {
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
struct NormalizedLine {
    original: String,
    normalized: String,
    char_map: Vec<usize>,
}

#[derive(Debug, Clone)]
struct LineInfo {
    line_index: usize,
    raw_line: String,
    normalized: NormalizedLine,
}

fn replace_names_with_map(
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
                    | "skip"
                    | "surveil"
                    | "pay"
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
            && !preceded_by_named_keyword(bytes, idx)
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
            && !preceded_by_named_keyword(bytes, idx)
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

fn strip_parenthetical_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
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

fn is_labeled_ability_word_prefix(prefix: &str) -> bool {
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

fn preserve_keyword_prefix_for_parse(prefix: &str) -> bool {
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
            | "replicate"
            | "spectacle"
            | "strive"
            | "surge"
            | "suspend"
            | "ward"
    )
}

fn strip_labeled_ability_word_prefix_with_map(text: &str, map: &[usize]) -> (String, Vec<usize>) {
    let separator = text
        .find('—')
        .map(|idx| (idx, '—'.len_utf8()))
        .or_else(|| text.find(" - ").map(|idx| (idx, " - ".len())));
    let Some((sep_idx, sep_len)) = separator else {
        return (text.to_string(), map.to_vec());
    };

    let prefix = text[..sep_idx].trim();
    if !is_labeled_ability_word_prefix(prefix) {
        return (text.to_string(), map.to_vec());
    }
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
    let remainder_char_start = text[..remainder_start].chars().count();
    let remainder_map = if remainder_char_start < map.len() {
        map[remainder_char_start..].to_vec()
    } else {
        Vec::new()
    };
    (remainder, remainder_map)
}

fn normalize_line_for_parse(
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

fn is_ignorable_unparsed_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.starts_with('(') && trimmed.ends_with(')')
}

fn byte_to_char_index(text: &str, byte_idx: usize) -> usize {
    if byte_idx == 0 {
        return 0;
    }
    let clamped = byte_idx.min(text.len());
    text[..clamped].chars().count()
}

fn map_span_to_original(
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

fn split_on_period(tokens: &[Token]) -> Vec<Vec<Token>> {
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

fn split_on_comma(tokens: &[Token]) -> Vec<Vec<Token>> {
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

fn split_on_comma_or_semicolon(tokens: &[Token]) -> Vec<Vec<Token>> {
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

fn split_on_and(tokens: &[Token]) -> Vec<Vec<Token>> {
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

fn is_basic_color_word(word: &str) -> bool {
    matches!(word, "white" | "blue" | "black" | "red" | "green" | "colorless")
}

fn starts_with_inline_token_rules_tail(words: &[&str]) -> bool {
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

fn starts_with_inline_token_rules_continuation(words: &[&str]) -> bool {
    matches!(
        words.first().copied(),
        Some(
            "it"
                | "they"
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

fn is_token_creation_context(words: &[&str]) -> bool {
    words.first().copied() == Some("create")
        && words.iter().any(|word| matches!(*word, "token" | "tokens"))
}

fn has_inline_token_rules_context(words: &[&str]) -> bool {
    words
        .windows(3)
        .any(|window| matches!(window, ["when", "this", "token"] | ["whenever", "this", "token"]))
        || words
            .windows(4)
            .any(|window| window == ["at", "the", "beginning", "of"])
        || (words.contains(&"except") && words.contains(&"copy") && words.contains(&"token"))
}

fn should_keep_and_for_token_rules(current: &[Token], remaining: &[Token]) -> bool {
    if current.is_empty() || remaining.is_empty() {
        return false;
    }
    let current_words = words(current);
    if current_words.is_empty() {
        return false;
    }
    if !is_token_creation_context(&current_words) && !has_inline_token_rules_context(&current_words) {
        return false;
    }
    let remaining_words = words(remaining);
    starts_with_inline_token_rules_tail(&remaining_words)
}

fn should_keep_and_for_attachment_object_list(current: &[Token], remaining: &[Token]) -> bool {
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

fn split_effect_chain_on_and(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("and") {
            let prev_word = current.last().and_then(Token::as_word);
            let next_word = tokens.get(idx + 1).and_then(Token::as_word);
            let is_color_pair = prev_word
                .zip(next_word)
                .is_some_and(|(left, right)| is_basic_color_word(left) && is_basic_color_word(right));
            if is_color_pair
                || should_keep_and_for_token_rules(&current, &tokens[idx + 1..])
                || should_keep_and_for_attachment_object_list(&current, &tokens[idx + 1..])
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

fn has_effect_head_without_verb(tokens: &[Token]) -> bool {
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

fn segment_has_effect_head(tokens: &[Token]) -> bool {
    find_verb(tokens).is_some() || has_effect_head_without_verb(tokens)
}

fn join_sentences_with_period(sentences: &[Vec<Token>]) -> Vec<Token> {
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
fn split_segments_on_comma_then(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    let back_ref_words = ["that", "it", "them", "its"];
    let mut result = Vec::new();
    for segment in segments {
        let segment_words = words(&segment);
        let starts_with_for_each_player_or_opponent =
            segment_words.starts_with(&["each", "player"])
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
                let allow_gain_or_lose_life_equal_followup = !starts_with_for_each_player_or_opponent
                    && has_back_ref
                    && (after_words.starts_with(&["gain", "life", "equal", "to", "that"])
                        || after_words.starts_with(&["gains", "life", "equal", "to", "that"])
                        || after_words.starts_with(&["lose", "life", "equal", "to", "that"])
                        || after_words.starts_with(&["loses", "life", "equal", "to", "that"]));
                if has_effect_head && (!has_back_ref || allow_backref_split) {
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

fn split_segments_on_comma_effect_head(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
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
            let after_starts_effect = find_verb(after.as_slice()).is_some_and(|(_, verb_idx)| {
                verb_idx == 0
            }) || has_effect_head_without_verb(after.as_slice());
            let before_words = words(before.as_slice());
            let after_words = words(after.as_slice());
            if before_words.first() == Some(&"unless") {
                continue;
            }
            let is_inline_token_rules_split =
                (is_token_creation_context(&before_words)
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

fn split_cost_segments(tokens: &[Token]) -> Vec<Vec<Token>> {
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

fn parse_mana_output_options_tokens(
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

fn parse_mana_output_options_for_line(
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

fn parse_saga_chapter_prefix(line: &str) -> Option<(Vec<u32>, &str)> {
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

fn roman_to_int(roman: &str) -> Option<u32> {
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

fn parse_level_header(line: &str) -> Option<(u32, Option<u32>)> {
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

fn is_untap_during_each_other_players_untap_step_words(words: &[&str]) -> bool {
    if words.first().copied() != Some("untap") {
        return false;
    }
    words.windows(6).any(|window| {
        window == ["during", "each", "other", "player", "untap", "step"]
            || window == ["during", "each", "other", "players", "untap", "step"]
    })
}

fn is_non_mana_additional_cost_modifier_line(normalized_line: &str) -> bool {
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

fn dash_labeled_remainder_starts_with_trigger(line: &str) -> bool {
    let lower = line.trim().to_ascii_lowercase();
    let remainder = lower
        .split_once('—')
        .map(|(_, rest)| rest.trim())
        .or_else(|| lower.split_once(" - ").map(|(_, rest)| rest.trim()));
    let Some(rest) = remainder else {
        return false;
    };
    rest.starts_with("whenever ")
        || rest.starts_with("when ")
        || rest.starts_with("at ")
}

fn parse_line(line: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    parser_trace_line("parse_line:entry", line);
    let normalized = line
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    if normalized.starts_with("activate only") {
        return Ok(LineAst::StaticAbility(StaticAbility::custom(
            "activation_restriction",
            line.trim().to_string(),
        )));
    }
    if normalized.starts_with("this ability triggers only") {
        return Ok(LineAst::StaticAbility(StaticAbility::custom(
            "trigger_restriction",
            line.trim().to_string(),
        )));
    }
    if normalized.starts_with("as this land enters")
        && normalized.contains("reveal")
        && normalized.contains("from your hand")
    {
        let mut abilities = vec![StaticAbility::custom(
            "as_enters_reveal",
            line.trim().to_string(),
        )];
        if normalized.contains("enters tapped") || normalized.contains("enter tapped") {
            abilities.push(StaticAbility::enters_tapped_ability());
        }
        return Ok(LineAst::StaticAbilities(abilities));
    }
    if let Some((chapters, rest)) = parse_saga_chapter_prefix(&normalized) {
        let tokens = tokenize_line(rest, line_index);
        parser_trace("parse_line:branch=saga", &tokens);
        let effects = parse_effect_sentences(&tokens)?;
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(chapters),
            effects,
            once_each_turn: false,
        });
    }

    let tokens = tokenize_line(line, line_index);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty line".to_string()));
    }

    if normalized.starts_with("as an additional cost to cast this spell") {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let effect_start = if let Some(idx) = comma_idx {
            idx + 1
        } else if let Some(idx) = tokens.iter().position(|token| token.is_word("spell")) {
            idx + 1
        } else {
            tokens.len()
        };
        let effect_tokens = tokens.get(effect_start..).unwrap_or_default();
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(
                "additional cost line missing effect clause".to_string(),
            ));
        }
        parser_trace("parse_line:branch=additional-cost", effect_tokens);
        if let Some(options) = parse_additional_cost_choice_options(effect_tokens)? {
            return Ok(LineAst::AdditionalCostChoice { options });
        }
        let effects = parse_effect_sentences(effect_tokens)?;
        return Ok(LineAst::AdditionalCost { effects });
    }

    if is_non_mana_additional_cost_modifier_line(&normalized) {
        return Ok(LineAst::StaticAbility(StaticAbility::custom(
            "additional_cost_modifier",
            line.trim().to_string(),
        )));
    }

    if tokens.first().is_some_and(|token| token.is_word("you"))
        && tokens.get(1).is_some_and(|token| token.is_word("may"))
        && let Some(rather_idx) = tokens.iter().position(|token| token.is_word("rather"))
    {
        let rather_tail = words(tokens.get(rather_idx + 1..).unwrap_or_default());
        let is_spell_cost_clause = rather_tail.starts_with(&["than", "pay", "this"])
            && rather_tail.contains(&"mana")
            && rather_tail.contains(&"cost")
            && (rather_tail.contains(&"spell") || rather_tail.contains(&"spells"));
        if is_spell_cost_clause {
            let cost_clause_end = (rather_idx + 1..tokens.len())
                .rfind(|idx| tokens[*idx].is_word("cost") || tokens[*idx].is_word("costs"))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "alternative cost line missing terminal cost word (line: '{}')",
                        line
                    ))
                })?;
            let trailing_words = words(&tokens[cost_clause_end + 1..]);
            if !trailing_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing clause after alternative cost (line: '{}', trailing: '{}')",
                    line,
                    trailing_words.join(" ")
                )));
            }
            let cost_tokens = tokens.get(2..rather_idx).unwrap_or_default();
            if cost_tokens.is_empty() {
                return Err(CardTextError::ParseError(
                    "alternative cost line missing cost clause".to_string(),
                ));
            }
            let (total_cost, mut cost_effects) = parse_activation_cost(cost_tokens)?;
            let mana_cost = total_cost.mana_cost().cloned();
            let unsupported_non_mana = total_cost
                .costs()
                .iter()
                .any(|cost| cost.mana_cost_ref().is_none());
            if unsupported_non_mana {
                return Err(CardTextError::ParseError(format!(
                    "unsupported non-mana alternative cost components (clause: '{}')",
                    words(cost_tokens).join(" ")
                )));
            }
            // Keep cost effects stable for deterministic snapshots.
            if !cost_effects.is_empty() {
                cost_effects.shrink_to_fit();
            }
            parser_trace("parse_line:branch=alternative-cost", cost_tokens);
            return Ok(LineAst::AlternativeCost {
                mana_cost,
                cost_effects,
            });
        }
    }

    if let Some(ability) = parse_equip_line(&tokens)? {
        parser_trace("parse_line:branch=equip", &tokens);
        return Ok(LineAst::Ability(ability));
    }

    if let Some(ability) = parse_level_up_line(&tokens)? {
        parser_trace("parse_line:branch=level-up", &tokens);
        return Ok(LineAst::Ability(ability));
    }

    if let Some(ability) = parse_cycling_line(&tokens)? {
        parser_trace("parse_line:branch=cycling", &tokens);
        return Ok(LineAst::Ability(ability));
    }

    if let Some(ability) = parse_morph_keyword_line(&tokens)? {
        parser_trace("parse_line:branch=morph", &tokens);
        return Ok(LineAst::Ability(ability));
    }

    if let Some(method) = parse_escape_line(&tokens)? {
        parser_trace("parse_line:branch=escape", &tokens);
        return Ok(LineAst::AlternativeCastingMethod(method));
    }

    if let Some(method) = parse_madness_line(&tokens)? {
        parser_trace("parse_line:branch=madness", &tokens);
        return Ok(LineAst::AlternativeCastingMethod(method));
    }

    if let Some((trigger_idx, _)) = tokens.iter().enumerate().find(|(idx, token)| {
        token.is_word("whenever")
            || token.is_word("when")
            || is_at_trigger_intro(&tokens, *idx)
    }) && (trigger_idx <= 2
        || (trigger_idx > 2 && dash_labeled_remainder_starts_with_trigger(line)))
    {
        parser_trace("parse_line:branch=triggered", &tokens[trigger_idx..]);
        return parse_triggered_line(&tokens[trigger_idx..]);
    }

    if tokens.first().is_some_and(|token| token.is_word("waterbend"))
        && let Some(ability) = parse_activated_line(&tokens[1..])?
    {
        parser_trace("parse_line:branch=waterbend-activated", &tokens[1..]);
        return Ok(LineAst::Ability(ability));
    }

    if let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    {
        let cost_tokens = &tokens[..colon_idx];
        if starts_with_activation_cost(cost_tokens) {
            if let Some(ability) = parse_activated_line(&tokens)? {
                parser_trace("parse_line:branch=activated", &tokens);
                return Ok(LineAst::Ability(ability));
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activated ability line (line: '{line}')"
            )));
        } else if (line.contains('—') || line.contains(" - "))
            && let Some(start_idx) = find_activation_cost_start(cost_tokens)
            && let Some(ability) = parse_activated_line(&tokens[start_idx..])?
        {
            parser_trace("parse_line:branch=activated-labeled", &tokens[start_idx..]);
            return Ok(LineAst::Ability(ability));
        }
    }

    let line_words = words(&tokens);
    let has_token_mana_reminder_tail = line_words.contains(&"create")
        && line_words.contains(&"sacrifice")
        && line_words.contains(&"add")
        && line_words
            .windows(2)
            .any(|window| window == ["it", "has"] || window == ["they", "have"]);
    if has_token_mana_reminder_tail
        && let Ok(effects) = parse_effect_sentences(&tokens)
        && !effects.is_empty()
    {
        parser_trace("parse_line:branch=statement-token-mana-reminder", &tokens);
        return Ok(LineAst::Statement { effects });
    }

    let is_each_other_player_untap_static =
        is_untap_during_each_other_players_untap_step_words(&line_words);
    let starts_with_statement_effect_head = find_verb(&tokens).is_some_and(|(_, idx)| idx == 0)
        || tokens.first().is_some_and(|token| token.is_word("choose"));
    if starts_with_statement_effect_head && !is_each_other_player_untap_static {
        match parse_effect_sentences(&tokens) {
            Ok(effects) if !effects.is_empty() => {
                parser_trace("parse_line:branch=statement-verb-leading", &tokens);
                return Ok(LineAst::Statement { effects });
            }
            Ok(_) => {}
            Err(err) => {
                return Err(err);
            }
        }
    }

    if let Some(abilities) = parse_static_ability_line(&tokens)? {
        parser_trace("parse_line:branch=static", &tokens);
        if abilities.len() == 1 {
            return Ok(LineAst::StaticAbility(
                abilities.into_iter().next().expect("single static ability"),
            ));
        }
        return Ok(LineAst::StaticAbilities(abilities));
    }

    if let Some(actions) = parse_ability_line(&tokens) {
        parser_trace("parse_line:branch=keyword-ability-line", &tokens);
        return Ok(LineAst::Abilities(actions));
    }

    parser_trace("parse_line:branch=statement", &tokens);
    let effects = parse_effect_sentences(&tokens)?;
    if effects.is_empty() {
        parser_trace("parse_line:branch=statement-empty", &tokens);
        return Err(CardTextError::ParseError(format!(
            "unsupported line: {line}"
        )));
    }

    Ok(LineAst::Statement { effects })
}

fn parse_additional_cost_choice_options(
    tokens: &[Token],
) -> Result<Option<Vec<AdditionalCostChoiceOptionAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let option_tokens = split_on_or(tokens);
    if option_tokens.len() < 2 {
        return Ok(None);
    }

    let mut options = Vec::new();
    for mut option in option_tokens {
        while option
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            option.remove(0);
        }
        let option = trim_commas(&option).to_vec();
        if option.is_empty() {
            continue;
        }

        if find_verb(&option).is_none() {
            return Ok(None);
        }

        let effects = parse_effect_sentences(&option)?;
        if effects.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "additional cost option parsed to no effects (clause: '{}')",
                words(&option).join(" ")
            )));
        }
        options.push(AdditionalCostChoiceOptionAst {
            description: words(&option).join(" "),
            effects,
        });
    }

    if options.len() < 2 {
        return Ok(None);
    }

    Ok(Some(options))
}

fn is_at_trigger_intro(tokens: &[Token], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return false;
    }

    let second = tokens.get(idx + 1).and_then(Token::as_word);
    let third = tokens.get(idx + 2).and_then(Token::as_word);
    matches!(
        (second, third),
        (Some("beginning"), _)
            | (Some("end"), _)
            | (Some("the"), Some("beginning"))
            | (Some("the"), Some("end"))
    )
}

fn starts_with_activation_cost(tokens: &[Token]) -> bool {
    let Some(word) = tokens.first().and_then(Token::as_word) else {
        return false;
    };
    if matches!(
        word,
        "tap"
            | "t"
            | "pay"
            | "discard"
            | "mill"
            | "sacrifice"
            | "put"
            | "remove"
            | "exile"
            | "return"
            | "e"
    ) {
        return true;
    }
    if word.contains('/') {
        return parse_mana_symbol_group(word).is_ok();
    }
    parse_mana_symbol(word).is_ok()
}

fn find_activation_cost_start(tokens: &[Token]) -> Option<usize> {
    (0..tokens.len()).find(|idx| starts_with_activation_cost(&tokens[*idx..]))
}

fn parse_flashback_keyword_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let words_all = words(tokens);
    if words_all.first().copied() != Some("flashback") {
        return None;
    }
    let (cost, consumed) = leading_mana_symbols_to_oracle(&words_all[1..])?;
    let mut text = format!("Flashback {cost}");
    let tail = &words_all[1 + consumed..];
    if !tail.is_empty() {
        let mut tail_text = tail.join(" ");
        if let Some(first) = tail_text.chars().next() {
            let upper = first.to_ascii_uppercase().to_string();
            let rest = &tail_text[first.len_utf8()..];
            tail_text = format!("{upper}{rest}");
        }
        text.push_str(", ");
        text.push_str(&tail_text);
    }
    Some(vec![KeywordAction::MarkerText(text)])
}

fn parse_ability_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    if let Some(actions) = parse_flashback_keyword_line(tokens) {
        return Some(actions);
    }

    let segments = split_on_comma_or_semicolon(tokens);
    let mut actions = Vec::new();

    for segment in segments {
        if segment.is_empty() {
            continue;
        }

        if let Some(protection_actions) = parse_protection_chain(&segment) {
            actions.extend(protection_actions);
            continue;
        }

        // Try the segment as-is first, then split on "and" for compound keywords
        if let Some(action) = parse_ability_phrase(&segment) {
            actions.push(action);
        } else {
            // Split on "and" to handle "menace and deathtouch", "trample and haste", etc.
            let and_parts = split_on_and(&segment);
            if and_parts.len() > 1 {
                let mut all_ok = true;
                for part in &and_parts {
                    if part.is_empty() {
                        continue;
                    }
                    if let Some(action) = parse_ability_phrase(part) {
                        actions.push(action);
                    } else {
                        all_ok = false;
                        break;
                    }
                }
                if !all_ok {
                    return None;
                }
            } else {
                return None;
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn reject_unimplemented_keyword_actions(
    _actions: &[KeywordAction],
    _clause: &str,
) -> Result<(), CardTextError> {
    Ok(())
}

fn parse_protection_chain(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let mut words = words(tokens);
    if words.first().copied() == Some("and") {
        words.remove(0);
    }
    if words.len() < 3 {
        return None;
    }
    if words[0] != "protection" || words[1] != "from" {
        return None;
    }

    let mut actions = Vec::new();
    for idx in 0..words.len().saturating_sub(1) {
        if words[idx] != "from" {
            continue;
        }
        let color = match words[idx + 1] {
            "white" => Some(ColorSet::WHITE),
            "blue" => Some(ColorSet::BLUE),
            "black" => Some(ColorSet::BLACK),
            "red" => Some(ColorSet::RED),
            "green" => Some(ColorSet::GREEN),
            _ => None,
        };
        if let Some(color) = color {
            actions.push(KeywordAction::ProtectionFrom(color));
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn keyword_action_to_static_ability(action: KeywordAction) -> Option<StaticAbility> {
    match action {
        KeywordAction::Flying => Some(StaticAbility::flying()),
        KeywordAction::Menace => Some(StaticAbility::menace()),
        KeywordAction::Hexproof => Some(StaticAbility::hexproof()),
        KeywordAction::Haste => Some(StaticAbility::haste()),
        KeywordAction::Improvise => Some(StaticAbility::improvise()),
        KeywordAction::Convoke => Some(StaticAbility::convoke()),
        KeywordAction::AffinityForArtifacts => Some(StaticAbility::affinity_for_artifacts()),
        KeywordAction::Delve => Some(StaticAbility::delve()),
        KeywordAction::FirstStrike => Some(StaticAbility::first_strike()),
        KeywordAction::DoubleStrike => Some(StaticAbility::double_strike()),
        KeywordAction::Deathtouch => Some(StaticAbility::deathtouch()),
        KeywordAction::Lifelink => Some(StaticAbility::lifelink()),
        KeywordAction::Vigilance => Some(StaticAbility::vigilance()),
        KeywordAction::Trample => Some(StaticAbility::trample()),
        KeywordAction::Reach => Some(StaticAbility::reach()),
        KeywordAction::Defender => Some(StaticAbility::defender()),
        KeywordAction::Flash => Some(StaticAbility::flash()),
        KeywordAction::Phasing => Some(StaticAbility::phasing()),
        KeywordAction::Indestructible => Some(StaticAbility::indestructible()),
        KeywordAction::Shroud => Some(StaticAbility::shroud()),
        KeywordAction::Ward(amount) => Some(StaticAbility::custom(
            "ward",
            format!("ward {amount}"),
        )),
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying => Some(StaticAbility::custom("undying", "undying".to_string())),
        KeywordAction::Persist => Some(StaticAbility::custom("persist", "persist".to_string())),
        KeywordAction::Prowess => Some(StaticAbility::custom("prowess", "prowess".to_string())),
        KeywordAction::Exalted => Some(StaticAbility::custom("exalted", "exalted".to_string())),
        KeywordAction::Storm => Some(StaticAbility::custom("storm", "storm".to_string())),
        KeywordAction::Toxic(amount) => Some(StaticAbility::custom(
            "toxic",
            format!("toxic {amount}"),
        )),
        KeywordAction::Fear => Some(StaticAbility::fear()),
        KeywordAction::Intimidate => Some(StaticAbility::intimidate()),
        KeywordAction::Shadow => Some(StaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(StaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(StaticAbility::flanking()),
        KeywordAction::Landwalk(subtype) => Some(StaticAbility::landwalk(subtype)),
        KeywordAction::Bloodthirst(amount) => Some(StaticAbility::bloodthirst(amount)),
        KeywordAction::Rampage(amount) => Some(StaticAbility::custom(
            "rampage",
            format!("rampage {amount}"),
        )),
        KeywordAction::Bushido(amount) => Some(StaticAbility::custom(
            "bushido",
            format!("bushido {amount}"),
        )),
        KeywordAction::Changeling => Some(StaticAbility::changeling()),
        KeywordAction::ProtectionFrom(colors) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Color(colors),
        )),
        KeywordAction::ProtectionFromAllColors => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::AllColors,
        )),
        KeywordAction::ProtectionFromColorless => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::Colorless,
        )),
        KeywordAction::ProtectionFromCardType(card_type) => Some(StaticAbility::protection(
            crate::ability::ProtectionFrom::CardType(card_type),
        )),
        KeywordAction::ProtectionFromSubtype(subtype) => Some(StaticAbility::custom(
            "protection_from_subtype",
            format!("protection from {:?}", subtype),
        )),
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::Devoid => Some(StaticAbility::make_colorless(ObjectFilter::source())),
        KeywordAction::Marker(name) => Some(StaticAbility::custom(name, name.to_string())),
        KeywordAction::MarkerText(text) => {
            Some(StaticAbility::custom("keyword_marker", text))
        }
    }
}

fn parse_static_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if let Some(ability) = parse_static_text_marker_line(tokens) {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_enters_tapped_with_choose_color_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_damage_not_removed_cleanup_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_choose_color_as_enters_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_damage_redirect_to_source_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_characteristic_defining_pt_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_no_maximum_hand_size_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_library_of_leng_discard_replacement_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_toph_first_metalbender_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_discard_or_redirect_replacement_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_pay_life_or_enter_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_copy_activated_abilities_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_spend_mana_as_any_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_source_activation_spend_mana_as_any_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enchanted_has_activated_ability_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_filter_has_granted_ability_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_equipped_gets_and_has_activated_ability_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_shuffle_into_library_from_graveyard_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_permanents_enter_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_cant_cycle_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_starting_life_bonus_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_buyback_cost_reduction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_spell_cost_increase_per_target_beyond_first_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_flashback_cost_modifier_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_spells_cost_modifier_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_players_skip_upkeep_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_legend_rule_doesnt_apply_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_permanents_are_artifacts_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_permanents_colorless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_cards_spells_permanents_colorless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_all_are_color_and_type_addition_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_creatures_are_color_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_blood_moon_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_remove_snow_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_granted_keyword_static_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_lose_all_abilities_and_transform_base_pt_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_lose_all_abilities_and_base_pt_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_creatures_lose_flying_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_anthem_and_type_color_addition_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_anthem_and_keyword_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_all_have_indestructible_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_subject_cant_be_blocked_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_may_choose_not_to_untap_during_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_untap_during_each_other_players_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_doesnt_untap_during_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_equipped_creature_has_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_attached_has_keywords_and_triggered_ability_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_attached_gets_and_has_ability_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(abilities) = parse_gets_and_attacks_each_combat_if_able_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_attacks_each_combat_if_able_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_composed_anthem_effects_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_has_base_power_toughness_static_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_anthem_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_flying_restriction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_can_block_only_flying_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_assign_damage_as_unblocked_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_grant_flash_to_noncreature_spells_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_creatures_cant_block_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_enters_tapped_with_counters_line(tokens)? {
        return Ok(Some(abilities));
    }
    if let Some(ability) = parse_enters_with_counters_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_with_additional_counter_for_filter_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_conditional_enters_tapped_unless_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_tapped_for_filter_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_enters_tapped_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_additional_land_play_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_play_lands_from_graveyard_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_cost_reduction_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_all_creatures_able_to_block_source_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(abilities) = parse_cant_clauses(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}

fn parse_composed_anthem_effects_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words
        .windows(4)
        .any(|window| window == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }

    let comma_segments = split_on_comma(tokens);
    if comma_segments.len() < 2 {
        return Ok(None);
    }

    let Some(first_action_idx) = tokens.iter().position(|token| {
        token.is_word("get")
            || token.is_word("gets")
            || token.is_word("have")
            || token.is_word("has")
    }) else {
        return Ok(None);
    };

    let subject_tokens = trim_commas(&tokens[..first_action_idx]);
    if subject_tokens.is_empty() || parse_anthem_subject(&subject_tokens).is_err() {
        return Ok(None);
    }

    let mut saw_omitted_subject_clause = false;
    let mut compiled = Vec::new();

    for (idx, raw_segment) in comma_segments.into_iter().enumerate() {
        let mut segment = trim_commas(&raw_segment).to_vec();
        if segment.is_empty() {
            continue;
        }

        if segment.first().is_some_and(|token| token.is_word("and")) {
            let trimmed = trim_commas(&segment[1..]);
            if trimmed.first().is_some_and(|token| {
                token.is_word("get")
                    || token.is_word("gets")
                    || token.is_word("have")
                    || token.is_word("has")
            }) {
                segment = trimmed.to_vec();
            }
        }

        let starts_with_action = segment.first().is_some_and(|token| {
            token.is_word("get")
                || token.is_word("gets")
                || token.is_word("have")
                || token.is_word("has")
        });
        if starts_with_action {
            if idx > 0 {
                saw_omitted_subject_clause = true;
            }
            let mut expanded = subject_tokens.clone();
            expanded.extend(segment);
            segment = expanded;
        }

        let parsed_segment = if let Some(abilities) = parse_anthem_and_type_color_addition_line(&segment)? {
            abilities
        } else if let Some(abilities) = parse_anthem_and_keyword_line(&segment)? {
            abilities
        } else if let Some(abilities) = parse_granted_keyword_static_line(&segment)? {
            abilities
        } else if let Some(ability) = parse_anthem_line(&segment)? {
            vec![ability]
        } else {
            return Ok(None);
        };
        compiled.extend(parsed_segment);
    }

    if !saw_omitted_subject_clause || compiled.len() < 2 {
        return Ok(None);
    }

    Ok(Some(compiled))
}

fn parse_static_text_marker_line(tokens: &[Token]) -> Option<StaticAbility> {
    let words = words(tokens);
    if words.is_empty() {
        return None;
    }

    let is_once_each_turn_play_from_exile = words.starts_with(&["once", "each", "turn", "you", "may", "play"])
        && words.contains(&"from")
        && words.contains(&"exile")
        && words.contains(&"cast")
        && words
            .windows(2)
            .any(|window| window == ["spend", "mana"])
        && words
            .windows(4)
            .any(|window| window == ["as", "though", "it", "were"])
        && words
            .windows(3)
            .any(|window| window == ["any", "color", "to"]);
    if is_once_each_turn_play_from_exile {
        let mut text = "Once each turn, you may play a card from exile".to_string();
        if words
            .windows(3)
            .any(|window| window == ["collection", "counter", "on"])
        {
            text.push_str(" with a collection counter on it");
        }
        if words
            .windows(4)
            .any(|window| window == ["if", "it", "was", "exiled"])
        {
            text.push_str(" if it was exiled by an ability you controlled");
        }
        text.push_str(", and you may spend mana as though it were mana of any color to cast it");
        return Some(StaticAbility::custom(
            "once_each_turn_play_from_exile",
            text,
        ));
    }

    if words == ["you", "have", "shroud"] {
        return Some(StaticAbility::custom(
            "you_have_shroud",
            "You have shroud".to_string(),
        ));
    }

    if words == ["creatures", "without", "flying", "cant", "attack"] {
        return Some(StaticAbility::custom(
            "creatures_without_flying_cant_attack",
            "Creatures without flying can't attack".to_string(),
        ));
    }

    if words == ["this", "creature", "cant", "attack", "alone"] {
        return Some(StaticAbility::custom(
            "cant_attack_alone",
            "This creature can't attack alone".to_string(),
        ));
    }

    if words.len() == 4
        && words[0] == "ward"
        && words[1] == "pay"
        && words[3] == "life"
        && words[2].parse::<u32>().is_ok()
    {
        return Some(StaticAbility::custom(
            "ward_pay_life",
            format!("Ward—Pay {} life", words[2]),
        ));
    }

    if words.starts_with(&["lands", "dont", "untap", "during", "their", "controllers", "untap"])
        && matches!(words.last(), Some(&"step") | Some(&"steps"))
    {
        return Some(StaticAbility::custom(
            "lands_dont_untap",
            "Lands don't untap during their controllers' untap steps".to_string(),
        ));
    }

    None
}

fn parse_subject_cant_be_blocked_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();
    if normalized.len() < 4 || !normalized.ends_with(&["cant", "be", "blocked"]) {
        return Ok(None);
    }

    let tail_start = token_index_for_word_index(tokens, normalized.len() - 3).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unable to map cant-be-blocked tail (clause: '{}')",
            normalized.join(" ")
        ))
    })?;
    let subject_tokens = trim_commas(&tokens[..tail_start]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    if subject_tokens
        .iter()
        .any(|token| matches!(token, Token::Comma(_)) || token.is_word("and"))
    {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words
        .first()
        .is_some_and(|word| *word == "this" || *word == "it")
    {
        return Ok(None);
    }
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "as" | "long"
                | "if"
                | "when"
                | "whenever"
                | "get"
                | "gets"
                | "gain"
                | "gains"
                | "have"
                | "has"
        )
    }) {
        return Ok(None);
    }
    if subject_words.windows(3).any(|window| {
        window == ["power", "or", "toughness"] || window == ["toughness", "or", "power"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness cant-be-blocked subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let ability = match subject {
        AnthemSubjectAst::Source => StaticAbility::unblockable(),
        AnthemSubjectAst::Filter(filter) => {
            StaticAbility::grant_ability(filter, StaticAbility::unblockable())
        }
    };
    Ok(Some(ability))
}

fn parse_enters_tapped_with_choose_color_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("this")
        || !clause_words.contains(&"enters")
        || !clause_words.contains(&"tapped")
    {
        return Ok(None);
    }
    let tapped_word_idx = clause_words
        .iter()
        .position(|word| *word == "tapped")
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing tapped keyword in enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let tapped_token_idx =
        token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let trailing = &tokens[tapped_token_idx + 1..];
    if trailing.is_empty() {
        return Ok(None);
    }
    let Some(color_choice) = parse_choose_color_as_enters_line(trailing)? else {
        return Ok(None);
    };
    Ok(Some(vec![
        StaticAbility::enters_tapped_ability(),
        color_choice,
    ]))
}

fn parse_damage_not_removed_cleanup_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() != 9 {
        return Ok(None);
    }
    if words[0] != "damage" || words[2] != "removed" {
        return Ok(None);
    }
    let is_not = words[1] == "isnt" || words[1] == "isn't";
    let matches = is_not
        && words[3] == "from"
        && words[4] == "this"
        && words[5] == "creature"
        && words[6] == "during"
        && words[7] == "cleanup"
        && words[8] == "steps";
    if matches {
        return Ok(Some(StaticAbility::damage_not_removed_during_cleanup()));
    }
    Ok(None)
}

fn parse_choose_color_as_enters_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 || words[0] != "as" {
        return Ok(None);
    }

    let mut idx = 1;
    let subject = if words.get(idx) == Some(&"this") {
        idx += 1;
        if words.get(idx) == Some(&"land") {
            idx += 1;
            "this land"
        } else {
            "this"
        }
    } else if words.get(idx) == Some(&"it") {
        idx += 1;
        "it"
    } else {
        return Ok(None);
    };

    if words.get(idx) != Some(&"enters") {
        return Ok(None);
    }
    idx += 1;

    if words.get(idx) == Some(&"the") && words.get(idx + 1) == Some(&"battlefield") {
        idx += 2;
    }

    if words.get(idx) != Some(&"choose") {
        return Ok(None);
    }
    idx += 1;
    if words.get(idx) == Some(&"a") {
        idx += 1;
    }
    if words.get(idx) != Some(&"color") {
        return Ok(None);
    }
    idx += 1;

    let mut excluded = None;
    if words.get(idx) == Some(&"other") && words.get(idx + 1) == Some(&"than") {
        let Some(color_word) = words.get(idx + 2) else {
            return Ok(None);
        };
        let color_set = parse_color(color_word).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported color choice '{}' (clause: '{}')",
                color_word,
                words.join(" ")
            ))
        })?;
        let color = color_from_color_set(color_set).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "ambiguous color choice '{}' (clause: '{}')",
                color_word,
                words.join(" ")
            ))
        })?;
        excluded = Some(color);
        idx += 3;
    }

    if idx != words.len() {
        return Ok(None);
    }

    let display = if subject == "this land" {
        match excluded {
            Some(color) => format!(
                "As this land enters, choose a color other than {}.",
                format!("{color:?}").to_ascii_lowercase()
            ),
            None => "As this land enters, choose a color.".to_string(),
        }
    } else {
        match excluded {
            Some(color) => format!(
                "As it enters, choose a color other than {}.",
                format!("{color:?}").to_ascii_lowercase()
            ),
            None => "As it enters, choose a color.".to_string(),
        }
    };

    Ok(Some(StaticAbility::choose_color_as_enters(
        excluded, display,
    )))
}

fn parse_damage_redirect_to_source_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() != 19 {
        return Ok(None);
    }
    let matches = words[0] == "all"
        && words[1] == "damage"
        && words[2] == "that"
        && words[3] == "would"
        && words[4] == "be"
        && words[5] == "dealt"
        && words[6] == "to"
        && words[7] == "you"
        && words[8] == "and"
        && words[9] == "other"
        && (words[10] == "permanents" || words[10] == "permanent")
        && words[11] == "you"
        && words[12] == "control"
        && words[13] == "is"
        && words[14] == "dealt"
        && words[15] == "to"
        && words[16] == "this"
        && words[17] == "creature"
        && words[18] == "instead";
    if matches {
        return Ok(Some(
            StaticAbility::redirect_damage_from_you_and_other_permanents_to_source(),
        ));
    }
    Ok(None)
}

fn parse_characteristic_defining_pt_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    let has_this_pt = line_words.windows(4).any(|window| {
        window == ["this", "power", "and", "toughness"]
            || window == ["thiss", "power", "and", "toughness"]
    });
    if !has_this_pt {
        return Ok(None);
    }
    if !(line_words.contains(&"equal")
        && line_words.contains(&"number")
        && line_words.contains(&"of"))
    {
        return Ok(None);
    }

    let mut tail_tokens = tokens;
    if let Some(equal_word_idx) = line_words
        .windows(2)
        .position(|window| window == ["equal", "to"])
    {
        let start_word_idx = equal_word_idx + 2;
        if let Some(start_token_idx) = token_index_for_word_index(tokens, start_word_idx) {
            tail_tokens = &tokens[start_token_idx..];
        }
    }
    while tail_tokens
        .last()
        .is_some_and(|token| token.is_word("respectively") || matches!(token, Token::Period(_)))
    {
        tail_tokens = &tail_tokens[..tail_tokens.len().saturating_sub(1)];
    }
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let value = parse_characteristic_defining_pt_value(tail_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported characteristic defining P/T value (value: '{}')",
            words(tail_tokens).join(" ")
        ))
    })?;
    Ok(Some(StaticAbility::characteristic_defining_pt(
        value.clone(),
        value,
    )))
}

fn parse_characteristic_defining_pt_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if words.is_empty() {
        return None;
    }

    let plus_positions: Vec<usize> = words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| (*word == "plus").then_some(idx))
        .collect();
    if plus_positions.is_empty() {
        return parse_characteristic_defining_pt_term(tokens);
    }

    let mut values = Vec::new();
    let mut start_word_idx = 0usize;
    for plus_word_idx in plus_positions {
        let start_token_idx = token_index_for_word_index(tokens, start_word_idx)?;
        let end_token_idx = token_index_for_word_index(tokens, plus_word_idx)?;
        values.push(parse_characteristic_defining_pt_term(
            &tokens[start_token_idx..end_token_idx],
        )?);
        start_word_idx = plus_word_idx + 1;
    }
    let final_start_token_idx = token_index_for_word_index(tokens, start_word_idx)?;
    values.push(parse_characteristic_defining_pt_term(
        &tokens[final_start_token_idx..],
    )?);

    let mut iter = values.into_iter();
    let mut acc = iter.next()?;
    for value in iter {
        acc = Value::Add(Box::new(acc), Box::new(value));
    }
    Some(acc)
}

fn parse_characteristic_defining_pt_term(tokens: &[Token]) -> Option<Value> {
    if tokens.is_empty() {
        return None;
    }

    if let Some((number, used)) = parse_number(tokens) {
        if tokens.len() == used {
            return Some(Value::Fixed(number as i32));
        }
    }

    let mut start = tokens;
    while start
        .first()
        .is_some_and(|token| token.as_word().is_some_and(is_article))
    {
        start = &start[1..];
    }
    if start.is_empty() {
        return None;
    }

    if start
        .first()
        .is_some_and(|token| token.is_word("number"))
        && start
            .get(1)
            .is_some_and(|token| token.is_word("of"))
    {
        start = &start[2..];
    }
    if start.is_empty() {
        return None;
    }

    let filter = parse_object_filter(start, false).ok()?;
    Some(Value::Count(filter))
}

fn parse_shuffle_into_library_from_graveyard_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_would_be_put = words
        .windows(3)
        .any(|window| window == ["would", "be", "put"]);
    let has_graveyard = words.contains(&"graveyard");
    let has_anywhere = words.contains(&"anywhere");
    let has_shuffle = words.contains(&"shuffle");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");

    if has_would_be_put
        && has_graveyard
        && has_anywhere
        && has_shuffle
        && has_library
        && has_instead
    {
        return Ok(Some(StaticAbility::shuffle_into_library_from_graveyard()));
    }

    Ok(None)
}

fn parse_permanents_enter_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["permanents", "enter", "tapped"]
        || words.as_slice() == ["permanents", "enters", "tapped"]
    {
        return Ok(Some(StaticAbility::permanents_enter_tapped()));
    }
    Ok(None)
}

fn parse_players_cant_cycle_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "cant", "cycle", "cards"] {
        return Ok(Some(StaticAbility::players_cant_cycle()));
    }
    Ok(None)
}

fn parse_starting_life_bonus_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 8 || !words.starts_with(&["you", "start", "the", "game"]) {
        return Ok(None);
    }
    if !words.contains(&"additional") || !words.contains(&"life") {
        return Ok(None);
    }
    let mut amount = None;
    for (idx, _token) in tokens.iter().enumerate() {
        if let Some((value, _)) = parse_number(&tokens[idx..]) {
            amount = Some(value);
            break;
        }
    }
    let amount = amount
        .ok_or_else(|| CardTextError::ParseError("missing starting life amount".to_string()))?;
    Ok(Some(StaticAbility::starting_life_bonus(amount as i32)))
}

fn parse_buyback_cost_reduction_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 5 || !words.starts_with(&["buyback", "costs", "cost"]) {
        return Ok(None);
    }
    let (amount, _) = parse_number(&tokens[3..])
        .ok_or_else(|| CardTextError::ParseError("missing buyback reduction amount".to_string()))?;
    if !words.contains(&"less") {
        return Ok(None);
    }
    Ok(Some(StaticAbility::buyback_cost_reduction(amount)))
}

fn parse_spell_cost_increase_per_target_beyond_first_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }
    if !words.contains(&"more") || !words.contains(&"target") || !words.contains(&"beyond") {
        return Ok(None);
    }

    let costs_idx = tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
    let (amount_value, _) =
        parse_cost_modifier_amount(amount_tokens).unwrap_or((Value::Fixed(1), 0));
    let amount = if let Value::Fixed(v) = amount_value {
        v.max(0) as u32
    } else {
        1
    };

    Ok(Some(StaticAbility::cost_increase_per_target_beyond_first(
        amount,
    )))
}

fn parse_spells_cost_modifier_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }
    if clause_words.contains(&"first")
        && clause_words.contains(&"each")
        && clause_words.contains(&"turn")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported first-spell cost modifier mechanic (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let spells_idx = clause_words
        .iter()
        .position(|word| *word == "spell" || *word == "spells");
    let Some(spells_idx) = spells_idx else {
        return Ok(None);
    };
    let cost_idx = clause_words
        .iter()
        .position(|word| *word == "cost" || *word == "costs");
    let Some(cost_idx) = cost_idx else {
        return Ok(None);
    };
    if cost_idx <= spells_idx {
        return Ok(None);
    }

    let mut filter = parse_spell_filter(&tokens[..spells_idx]);

    let between_tokens = &tokens[spells_idx + 1..cost_idx];
    let between_words = &clause_words[spells_idx + 1..cost_idx];
    for (idx, token) in between_tokens.iter().enumerate() {
        if !token.is_word("spell") && !token.is_word("spells") {
            continue;
        }
        let mut start = idx;
        while start > 0 {
            if between_tokens[start - 1].is_word("and")
                || between_tokens[start - 1].is_word("or")
                || matches!(between_tokens[start - 1], Token::Comma(_))
            {
                break;
            }
            start -= 1;
        }
        let descriptor_tokens = trim_commas(&between_tokens[start..idx]);
        if descriptor_tokens.is_empty() {
            continue;
        }
        let extra_filter = parse_spell_filter(&descriptor_tokens);
        if spell_filter_has_identity(&extra_filter) {
            merge_spell_filters(&mut filter, extra_filter);
        }
    }
    let between_filter = parse_spell_filter(between_tokens);
    if spell_filter_has_identity(&between_filter) {
        merge_spell_filters(&mut filter, between_filter);
    }
    if between_words
        .windows(2)
        .any(|window| window == ["you", "cast"])
    {
        filter.controller = Some(PlayerFilter::You);
    }
    if between_words
        .iter()
        .any(|word| *word == "opponent" || *word == "opponents")
        && between_words
            .iter()
            .any(|word| *word == "cast" || *word == "casts")
    {
        filter.controller = Some(PlayerFilter::Opponent);
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut targets_idx = None;
    for (idx, token) in between_tokens.iter().enumerate() {
        if token.is_word("target") || token.is_word("targets") {
            if idx > 0 && between_tokens[idx - 1].is_word("that") {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let target_tokens = &between_tokens[targets_idx + 1..];
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target in spells-cost modifier clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target_words = words(target_tokens);
        if target_words.starts_with(&["you"]) {
            target_player = Some(PlayerFilter::You);
        } else if target_words.starts_with(&["opponent"])
            || target_words.starts_with(&["opponents"])
        {
            target_player = Some(PlayerFilter::Opponent);
        } else if target_words.starts_with(&["player"]) || target_words.starts_with(&["players"]) {
            target_player = Some(PlayerFilter::Any);
        } else {
            target_object = Some(parse_object_filter(target_tokens, false)?);
        }
        filter.targets_player = target_player;
        filter.targets_object = target_object;
    }

    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (mut amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = words(remaining_tokens);
    let is_less = remaining_words.contains(&"less");
    let is_more = remaining_words.contains(&"more");
    if !is_less && !is_more {
        return Ok(None);
    }

    if let Some(dynamic_value) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        amount_value = dynamic_value;
    } else if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing cost modifier amount".to_string(),
        ));
    }

    parse_trailing_targets_condition_in_cost_modifier(
        &mut filter,
        remaining_tokens,
        &clause_words,
    )?;

    if is_less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }

    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

fn parse_trailing_targets_condition_in_cost_modifier(
    filter: &mut crate::ability::SpellFilter,
    remaining_tokens: &[Token],
    clause_words: &[&str],
) -> Result<(), CardTextError> {
    let remaining_words = words(remaining_tokens);
    let Some(if_word_idx) = remaining_words.iter().position(|word| *word == "if") else {
        return Ok(());
    };
    let condition_words = &remaining_words[if_word_idx..];
    if condition_words.len() < 4
        || condition_words[0] != "if"
        || condition_words[1] != "it"
        || (condition_words[2] != "targets" && condition_words[2] != "target")
    {
        return Ok(());
    }

    let target_word_idx = if_word_idx + 3;
    let target_token_idx =
        token_index_for_word_index(remaining_tokens, target_word_idx).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unable to map trailing target condition in spells-cost modifier (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_tokens = &remaining_tokens[target_token_idx..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing target in trailing spells-cost condition (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = words(target_tokens);
    if target_words.starts_with(&["you"]) {
        filter.targets_player = Some(PlayerFilter::You);
        filter.targets_object = None;
        return Ok(());
    }
    if target_words.starts_with(&["opponent"]) || target_words.starts_with(&["opponents"]) {
        filter.targets_player = Some(PlayerFilter::Opponent);
        filter.targets_object = None;
        return Ok(());
    }
    if target_words.starts_with(&["player"]) || target_words.starts_with(&["players"]) {
        filter.targets_player = Some(PlayerFilter::Any);
        filter.targets_object = None;
        return Ok(());
    }

    filter.targets_object = Some(parse_object_filter(target_tokens, false)?);
    filter.targets_player = None;
    Ok(())
}

fn parse_flashback_cost_modifier_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 || clause_words.first().copied() != Some("flashback") {
        return Ok(None);
    }
    let cost_idx = tokens
        .iter()
        .rposition(|token| token.is_word("cost") || token.is_word("costs"));
    let Some(cost_idx) = cost_idx else {
        return Ok(None);
    };
    let amount_tokens = &tokens[cost_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount
        .clone()
        .map(|(value, used)| (value, used))
        .unwrap_or((Value::Fixed(1), 0));
    let remaining_tokens = &amount_tokens[used..];
    let remaining_words = words(remaining_tokens);
    let is_less = remaining_words.contains(&"less");
    let is_more = remaining_words.contains(&"more");
    if !is_less && !is_more {
        return Ok(None);
    }
    if parsed_amount.is_none() {
        return Err(CardTextError::ParseError(
            "missing flashback cost modifier amount".to_string(),
        ));
    }

    let mut filter = crate::ability::SpellFilter::default();
    filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Flashback);
    if clause_words
        .windows(2)
        .any(|window| window == ["you", "pay"])
    {
        filter.controller = Some(PlayerFilter::You);
    } else if clause_words
        .windows(3)
        .any(|window| window == ["your", "opponents", "pay"])
        || clause_words
            .windows(2)
            .any(|window| window == ["opponents", "pay"] || window == ["opponent", "pays"])
    {
        filter.controller = Some(PlayerFilter::Opponent);
    }

    if is_less {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::CostReduction::new(filter, amount_value),
        )));
    }
    Ok(Some(StaticAbility::new(
        crate::static_abilities::CostIncrease::new(filter, amount_value),
    )))
}

fn parse_cost_modifier_amount(tokens: &[Token]) -> Option<(Value, usize)> {
    if let Some((amount, used)) = parse_number(tokens) {
        return Some((Value::Fixed(amount as i32), used));
    }

    let word = tokens.first().and_then(Token::as_word)?;
    let symbol = parse_mana_symbol(word).ok()?;
    if let ManaSymbol::Generic(amount) = symbol {
        return Some((Value::Fixed(amount as i32), 1));
    }
    None
}

fn parse_dynamic_cost_modifier_value(tokens: &[Token]) -> Result<Option<Value>, CardTextError> {
    let words_all = words(tokens);
    let Some(each_idx) = words_all.iter().position(|word| *word == "each") else {
        return Ok(None);
    };

    let filter_tokens = &tokens[each_idx + 1..];
    let filter_words = words(filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }
    if filter_words.starts_with(&["creature", "that", "died", "this", "turn"])
        || filter_words.starts_with(&["creatures", "that", "died", "this", "turn"])
    {
        return Ok(Some(Value::CreaturesDiedThisTurn));
    }

    if filter_words.windows(2).any(|pair| pair == ["card", "type"])
        && filter_words.contains(&"graveyard")
    {
        let player = if filter_words
            .windows(2)
            .any(|pair| pair == ["your", "graveyard"])
        {
            PlayerFilter::You
        } else if filter_words
            .windows(2)
            .any(|pair| pair == ["opponents", "graveyard"] || pair == ["opponent", "graveyard"])
        {
            PlayerFilter::Opponent
        } else {
            PlayerFilter::You
        };
        return Ok(Some(Value::CardTypesInGraveyard(player)));
    }

    if filter_words.starts_with(&["color", "of", "mana", "spent", "to", "cast", "this", "spell"])
        || filter_words
            .starts_with(&["colors", "of", "mana", "spent", "to", "cast", "this", "spell"])
        || filter_words.starts_with(&["color", "of", "mana", "used", "to", "cast", "this", "spell"])
        || filter_words
            .starts_with(&["colors", "of", "mana", "used", "to", "cast", "this", "spell"])
    {
        return Ok(Some(Value::ColorsOfManaSpentToCastThisSpell));
    }

    if let Ok(filter) = parse_object_filter(filter_tokens, false) {
        return Ok(Some(Value::Count(filter)));
    }

    Ok(None)
}

fn parse_add_mana_equal_amount_value(tokens: &[Token]) -> Option<Value> {
    let words_all = words(tokens);
    let equal_idx = words_all
        .windows(2)
        .position(|window| window == ["equal", "to"])?;
    let tail = &words_all[equal_idx + 2..];
    if tail.is_empty() {
        return None;
    }

    if matches!(
        tail,
        ["this", "creature", "power"]
            | ["this", "creatures", "power"]
            | ["its", "power"]
            | ["that", "creature", "power"]
            | ["that", "creatures", "power"]
            | ["that", "objects", "power"]
    ) {
        let source = if tail[0] == "that" {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::PowerOf(Box::new(source)));
    }

    if matches!(
        tail,
        ["this", "creature", "toughness"]
            | ["this", "creatures", "toughness"]
            | ["its", "toughness"]
            | ["that", "creature", "toughness"]
            | ["that", "creatures", "toughness"]
            | ["that", "objects", "toughness"]
    ) {
        let source = if tail[0] == "that" {
            ChooseSpec::Tagged(TagKey::from(IT_TAG))
        } else {
            ChooseSpec::Source
        };
        return Some(Value::ToughnessOf(Box::new(source)));
    }

    if matches!(
        tail,
        ["that", "spell", "mana", "value"]
            | ["that", "spells", "mana", "value"]
            | ["that", "card", "mana", "value"]
            | ["that", "cards", "mana", "value"]
            | ["the", "sacrificed", "creatures", "mana", "value"]
            | ["the", "sacrificed", "artifacts", "mana", "value"]
            | ["the", "sacrificed", "permanents", "mana", "value"]
            | ["sacrificed", "creatures", "mana", "value"]
            | ["sacrificed", "artifacts", "mana", "value"]
            | ["sacrificed", "permanents", "mana", "value"]
            | ["its", "mana", "value"]
    ) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(
            TagKey::from(IT_TAG),
        ))));
    }
    if matches!(
        tail,
        ["this", "spell", "mana", "value"]
            | ["this", "creature", "mana", "value"]
            | ["this", "permanent", "mana", "value"]
            | ["this", "card", "mana", "value"]
    ) {
        return Some(Value::ManaValueOf(Box::new(ChooseSpec::Source)));
    }

    None
}

fn parse_add_mana_that_much_value(tokens: &[Token]) -> Option<Value> {
    let words_all = words(tokens);
    if words_all.starts_with(&["that", "much"]) {
        return Some(Value::EventValue(EventValueSpec::Amount));
    }
    None
}

fn parse_players_skip_upkeep_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["players", "skip", "their", "upkeep", "steps"] {
        return Ok(Some(StaticAbility::players_skip_upkeep()));
    }
    Ok(None)
}

fn parse_legend_rule_doesnt_apply_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"legend") && words.contains(&"rule") && words.contains(&"doesnt") {
        return Ok(Some(StaticAbility::legend_rule_doesnt_apply()));
    }
    Ok(None)
}

fn parse_all_permanents_colorless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "permanents", "are", "colorless"] {
        return Ok(Some(StaticAbility::make_colorless(
            ObjectFilter::permanent(),
        )));
    }
    Ok(None)
}

fn parse_all_permanents_are_artifacts_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&["all", "permanents", "are", "artifacts"]) {
        return Ok(Some(StaticAbility::add_card_types(
            ObjectFilter::permanent(),
            vec![CardType::Artifact],
        )));
    }
    Ok(None)
}

fn parse_all_cards_spells_permanents_colorless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"colorless")
        && words.contains(&"cards")
        && words.contains(&"spells")
        && words.contains(&"permanents")
    {
        return Ok(Some(StaticAbility::make_colorless(ObjectFilter::default())));
    }
    Ok(None)
}

fn parse_all_are_color_and_type_addition_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 10 {
        return Ok(None);
    }
    let Some(are_idx) = words.iter().position(|word| *word == "are") else {
        return Ok(None);
    };
    if are_idx == 0 || are_idx + 4 >= words.len() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..are_idx];
    let filter = parse_object_filter(subject_tokens, false)?;

    let Some(base_color) = words.get(are_idx + 1).and_then(|word| parse_color(word)) else {
        return Ok(None);
    };

    // Pattern: "<subject> are <color> and are <subtype>... in addition to their other creature types"
    if words.get(are_idx + 2) != Some(&"and") || words.get(are_idx + 3) != Some(&"are") {
        return Ok(None);
    }

    let tail = &words[are_idx + 4..];
    let Some(addition_idx) = tail
        .windows(5)
        .position(|window| window == ["in", "addition", "to", "their", "other"])
    else {
        return Ok(None);
    };
    if addition_idx == 0 {
        return Ok(None);
    }

    let scope = &tail[addition_idx + 5..];
    if !matches!(scope, ["creature", "type"] | ["creature", "types"]) {
        return Ok(None);
    }

    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in &tail[..addition_idx] {
        if is_article(descriptor) || matches!(*descriptor, "and" | "or" | "and/or") {
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !card_types.contains(&card_type) {
                card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !subtypes.contains(&subtype) {
                subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in are-color-and-type-addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if card_types.is_empty() && subtypes.is_empty() {
        return Ok(None);
    }

    let mut abilities = vec![StaticAbility::set_colors(filter.clone(), base_color)];
    if !card_types.is_empty() {
        abilities.push(StaticAbility::add_card_types(filter.clone(), card_types));
    }
    if !subtypes.is_empty() {
        abilities.push(StaticAbility::add_subtypes(filter, subtypes));
    }
    Ok(Some(abilities))
}

fn parse_all_creatures_are_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }
    let are_idx = words.iter().position(|word| *word == "are");
    let Some(are_idx) = are_idx else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..are_idx];
    let filter = parse_object_filter(subject_tokens, false)?;

    let color_word = words.get(are_idx + 1).copied();
    let Some(color_word) = color_word else {
        return Ok(None);
    };
    let Some(color) = parse_color(color_word) else {
        return Ok(None);
    };
    if words.len() != are_idx + 2 {
        return Ok(None);
    }

    Ok(Some(StaticAbility::set_colors(filter, color)))
}

fn parse_blood_moon_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["nonbasic", "lands", "are", "mountains"] {
        return Ok(Some(StaticAbility::blood_moon()));
    }
    Ok(None)
}

fn parse_remove_snow_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "lands", "are", "no", "longer", "snow"] {
        return Ok(Some(StaticAbility::remove_supertypes(
            ObjectFilter::land(),
            vec![Supertype::Snow],
        )));
    }
    Ok(None)
}

fn parse_granted_keyword_static_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "have" || *word == "has")
    {
        return Ok(None);
    }

    let have_token_idx = tokens
        .iter()
        .position(|token| token.is_word("have") || token.is_word("has"))
        .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    if words(&tokens[..have_token_idx])
        .iter()
        .any(|word| *word == "get" || *word == "gets")
    {
        return Ok(None);
    }

    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, have_token_idx)?;
    let subject_tokens = trim_commas(&tokens[subject_start..have_token_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"equipped")
        || subject_words.contains(&"enchanted")
        || subject_words.contains(&"mana")
    {
        return Ok(None);
    }
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "can"
                | "cant"
                | "cannot"
                | "attack"
                | "attacks"
                | "block"
                | "blocks"
                | "blocked"
                | "blocking"
                | "cast"
                | "spell"
                | "spells"
                | "during"
                | "until"
                | "unless"
                | "when"
                | "whenever"
                | "if"
                | "though"
        )
    }) {
        return Ok(None);
    }

    let tail_tokens = trim_commas(&tokens[have_token_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(None);
    }

    let mut tail_tokens = tail_tokens;
    if let Some(period_idx) = tail_tokens
        .iter()
        .position(|token| matches!(token, Token::Period(_)))
    {
        let leading = trim_commas(&tail_tokens[..period_idx]);
        let trailing = trim_commas(&tail_tokens[period_idx + 1..]);
        let trailing_words = words(&trailing);
        let is_supported_flashback_tail = trailing_words
            == ["its", "flashback", "cost", "is", "equal", "to", "its", "mana", "cost"];
        if !trailing_words.is_empty() && !is_supported_flashback_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        tail_tokens = leading;
    }

    let mut keyword_tokens = tail_tokens.clone();
    let mut suffix_condition = None;
    if let Some(idx) = words(&tail_tokens)
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    {
        if idx + 3 >= tail_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        keyword_tokens = trim_commas(&tail_tokens[..idx]);
        suffix_condition = Some(parse_static_condition_clause(&tail_tokens[idx + 3..])?);
    }
    if keyword_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing granted keyword list (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut grants_must_attack = false;
    let keyword_words = words(&keyword_tokens);
    if let Some(and_idx) = keyword_words
        .windows(6)
        .position(|window| window == ["and", "attack", "each", "combat", "if", "able"])
        .or_else(|| {
            keyword_words
                .windows(6)
                .position(|window| window == ["and", "attacks", "each", "combat", "if", "able"])
        })
    {
        keyword_tokens = trim_commas(&keyword_tokens[..and_idx]);
        grants_must_attack = true;
    }
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let Some(actions) = parse_ability_line(&keyword_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    if actions.is_empty() {
        return Ok(None);
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple static conditions are not supported in granted-keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        (Some(cond), None) | (None, Some(cond)) => Some(cond),
        (None, None) => None,
    };

    let mapped = actions
        .into_iter()
        .filter_map(keyword_action_to_static_ability)
        .collect::<Vec<_>>();
    if mapped.is_empty() && !grants_must_attack {
        return Ok(None);
    }

    let subject = parse_anthem_subject(&subject_tokens)?;
    let mut compiled = Vec::new();
    let mut granted_abilities = mapped;
    if grants_must_attack {
        granted_abilities.push(StaticAbility::must_attack());
    }
    for ability in granted_abilities {
        match &subject {
            AnthemSubjectAst::Source => {
                if let Some(condition) = &condition {
                    let granted = GrantAbility::source(ability).with_condition(condition.clone());
                    compiled.push(StaticAbility::new(granted));
                } else {
                    compiled.push(ability);
                }
            }
            AnthemSubjectAst::Filter(filter) => {
                let granted = if let Some(condition) = &condition {
                    GrantAbility::new(filter.clone(), ability).with_condition(condition.clone())
                } else {
                    GrantAbility::new(filter.clone(), ability)
                };
                compiled.push(StaticAbility::new(granted));
            }
        }
    }
    Ok(Some(compiled))
}

fn parse_all_creatures_lose_flying_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["all", "creatures", "lose", "flying"] {
        return Ok(Some(StaticAbility::remove_ability(
            ObjectFilter::creature(),
            StaticAbility::flying(),
        )));
    }
    Ok(None)
}

fn parse_lose_all_abilities_and_transform_base_pt_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    fn title_case_words(words: &[&str]) -> String {
        words
            .iter()
            .map(|word| {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(chars.as_str());
                    out
                } else {
                    String::new()
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    let words = words(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let Some(is_idx) = words.iter().position(|word| *word == "is" || *word == "are") else {
        return Ok(None);
    };
    let Some(with_idx) = words
        .windows(5)
        .position(|window| window == ["with", "base", "power", "and", "toughness"])
    else {
        return Ok(None);
    };
    if with_idx <= is_idx {
        return Ok(None);
    }

    let Some(pt_word) = words.get(with_idx + 5) else {
        return Ok(None);
    };
    let (power, toughness) = parse_pt_modifier(pt_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let has_lose_all = words
        .windows(3)
        .any(|window| matches!(window, ["lose", "all", "abilities"] | ["loses", "all", "abilities"]));
    if !has_lose_all {
        return Ok(None);
    }

    let subject_end = is_idx.min(
        words
            .iter()
            .position(|word| *word == "lose" || *word == "loses")
            .unwrap_or(is_idx),
    );
    if subject_end == 0 {
        return Ok(None);
    }
    let subject_tokens = trim_commas(&tokens[..subject_end]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities transform clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let mut descriptor_words = words[is_idx + 1..with_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word) && *word != "and")
        .collect::<Vec<_>>();
    if descriptor_words.is_empty() {
        return Ok(None);
    }
    if descriptor_words.first().copied() == Some("all") {
        descriptor_words.remove(0);
    }
    if descriptor_words.is_empty() {
        return Ok(None);
    }

    let mut set_colors = ColorSet::new();
    let mut set_card_types: Vec<CardType> = Vec::new();
    let mut creature_subtypes: Vec<Subtype> = Vec::new();

    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            set_colors = set_colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(descriptor) {
            if !set_card_types.contains(&card_type) {
                set_card_types.push(card_type);
            }
            continue;
        }
        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !creature_subtypes.contains(&subtype) {
                creature_subtypes.push(subtype);
            }
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported transform descriptor '{}' (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if !creature_subtypes.is_empty() && !set_card_types.contains(&CardType::Creature) {
        set_card_types.push(CardType::Creature);
    }

    let mut set_name: Option<String> = None;
    let tail_words = &words[with_idx + 6..];
    if let Some(named_idx) = tail_words.iter().position(|word| *word == "named") {
        let end_idx = (named_idx + 1..tail_words.len())
            .find(|idx| {
                matches!(
                    tail_words[*idx],
                    "and" | "lose" | "loses" | "with" | "it" | "that" | "those" | "this"
                )
            })
            .unwrap_or(tail_words.len());
        if end_idx > named_idx + 1 {
            set_name = Some(title_case_words(&tail_words[named_idx + 1..end_idx]));
        }
    }

    let mut abilities = vec![StaticAbility::remove_all_abilities(filter.clone())];

    if !set_card_types.is_empty() {
        abilities.push(StaticAbility::set_card_types(
            filter.clone(),
            set_card_types,
        ));
    }
    if !creature_subtypes.is_empty() {
        abilities.push(StaticAbility::set_creature_subtypes(
            filter.clone(),
            creature_subtypes,
        ));
    }
    if !set_colors.is_empty() {
        abilities.push(StaticAbility::set_colors(filter.clone(), set_colors));
    }
    if let Some(name) = set_name {
        abilities.push(StaticAbility::set_name(filter.clone(), name));
    }
    abilities.push(StaticAbility::set_base_power_toughness(
        filter,
        power,
        toughness,
    ));

    Ok(Some(abilities))
}

fn parse_lose_all_abilities_and_base_pt_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    let lose_idx = words
        .iter()
        .position(|word| *word == "lose" || *word == "loses");
    let Some(lose_idx) = lose_idx else {
        return Ok(None);
    };

    if !words[lose_idx + 1..]
        .windows(2)
        .any(|window| window == ["all", "abilities"])
    {
        return Ok(None);
    }
    if words.contains(&"becomes") {
        return Err(CardTextError::ParseError(format!(
            "unsupported lose-all-abilities static becomes clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let subject_tokens = &tokens[..lose_idx];
    let filter = parse_object_filter(subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in lose-all-abilities clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let mut abilities = vec![StaticAbility::remove_all_abilities(filter.clone())];

    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    if let Some(have_idx) = have_idx {
        let after_have = &words[have_idx + 1..];
        if after_have.starts_with(&["base", "power", "and", "toughness"])
            && let Some(modifier_token) = after_have.iter().find(|word| word.contains('/'))
            && let Ok((power, toughness)) = parse_pt_modifier(modifier_token)
        {
            abilities.push(StaticAbility::set_base_power_toughness(
                filter, power, toughness,
            ));
        }
    }

    Ok(Some(abilities))
}

fn parse_all_have_indestructible_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let have_idx = words
        .iter()
        .position(|word| *word == "have" || *word == "has");
    let Some(have_idx) = have_idx else {
        return Ok(None);
    };
    if words[..have_idx]
        .iter()
        .any(|word| *word == "get" || *word == "gets")
    {
        return Ok(None);
    }

    let have_token_idx = tokens
        .iter()
        .position(|token| token.is_word("have") || token.is_word("has"))
        .ok_or_else(|| CardTextError::ParseError("missing granted-keyword verb".to_string()))?;
    let tail = trim_commas(&tokens[have_token_idx + 1..]);
    let Some(actions) = parse_ability_line(&tail) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &words.join(" "))?;
    if actions.len() != 1
        || !actions
            .first()
            .is_some_and(|action| matches!(action, KeywordAction::Indestructible))
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..have_token_idx], false)?;
    Ok(Some(StaticAbility::grant_ability(
        filter,
        StaticAbility::indestructible(),
    )))
}

#[derive(Debug, Clone)]
enum AnthemSubjectAst {
    Source,
    Filter(ObjectFilter),
}

#[derive(Debug, Clone)]
struct ParsedAnthemClause {
    subject: AnthemSubjectAst,
    power: AnthemValue,
    toughness: AnthemValue,
    condition: Option<StaticCondition>,
}

fn words_start_with(tokens: &[Token], expected: &[&str]) -> bool {
    words(tokens).starts_with(expected)
}

fn find_source_reference_start(tokens: &[Token]) -> Option<usize> {
    let mut token_indices = Vec::new();
    let mut token_words = Vec::new();
    for (idx, token) in tokens.iter().enumerate() {
        if let Some(word) = token.as_word() {
            token_indices.push(idx);
            token_words.push(word);
        }
    }

    for word_start in 0..token_words.len() {
        if is_source_reference_words(&token_words[word_start..]) {
            return token_indices.get(word_start).copied();
        }
    }
    None
}

fn object_filter_specificity_score(filter: &ObjectFilter) -> usize {
    let mut score = 0usize;
    score += filter.tagged_constraints.len() * 20;
    score += filter.card_types.len() * 10;
    score += filter.all_card_types.len() * 10;
    score += filter.subtypes.len() * 8;
    score += filter.excluded_subtypes.len() * 8;
    score += usize::from(filter.controller.is_some()) * 6;
    score += usize::from(filter.owner.is_some()) * 6;
    score += usize::from(filter.zone.is_some()) * 4;
    score += usize::from(filter.other) * 3;
    score += usize::from(filter.token || filter.nontoken) * 3;
    score += usize::from(filter.tapped || filter.untapped) * 2;
    score += usize::from(filter.attacking || filter.blocking) * 2;
    score += usize::from(filter.colorless || filter.multicolored || filter.monocolored) * 2;
    score += usize::from(filter.with_counter.is_some() || filter.without_counter.is_some()) * 4;
    score += usize::from(!filter.excluded_card_types.is_empty()) * 2;
    score += usize::from(!filter.excluded_supertypes.is_empty()) * 2;
    score += usize::from(!filter.excluded_colors.is_empty()) * 2;
    score += usize::from(!filter.excluded_static_abilities.is_empty()) * 2;
    score += usize::from(!filter.excluded_custom_static_markers.is_empty()) * 2;
    score += usize::from(filter.colors.is_some()) * 2;
    score += usize::from(filter.power.is_some() || filter.toughness.is_some()) * 2;
    score
}

fn parse_best_object_filter_suffix(tokens: &[Token]) -> Option<ObjectFilter> {
    let mut best: Option<(usize, ObjectFilter)> = None;
    for start in 0..tokens.len() {
        if tokens[start].as_word().is_none() {
            continue;
        }
        let mut other = false;
        let mut candidate = &tokens[start..];
        if candidate
            .first()
            .is_some_and(|token| token.is_word("other") || token.is_word("another"))
        {
            other = true;
            candidate = &candidate[1..];
        }
        if candidate.is_empty() {
            continue;
        }
        let candidate_words = words(candidate);
        if matches!(candidate_words.as_slice(), ["it"] | ["them"]) {
            continue;
        }
        let Ok(filter) = parse_object_filter(candidate, other) else {
            continue;
        };
        let score = object_filter_specificity_score(&filter);
        if best
            .as_ref()
            .is_none_or(|(best_score, _)| score > *best_score)
        {
            best = Some((score, filter));
        }
    }
    best.map(|(_, filter)| filter)
}

fn parse_anthem_subject(tokens: &[Token]) -> Result<AnthemSubjectAst, CardTextError> {
    let subject_words = words(tokens);
    if subject_words.as_slice() == ["it"] {
        return Ok(AnthemSubjectAst::Source);
    }
    if find_source_reference_start(tokens).is_some() {
        return Ok(AnthemSubjectAst::Source);
    }
    parse_best_object_filter_suffix(tokens)
        .map(AnthemSubjectAst::Filter)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported anthem subject (clause: '{}')",
                words(tokens).join(" ")
            ))
        })
}

fn parse_static_quantity_prefix(
    tokens: &[Token],
    allow_default_one: bool,
) -> Result<(crate::effect::Comparison, usize), CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing quantity in static condition".to_string(),
        ));
    }

    if tokens[0].is_word("no") {
        return Ok((crate::effect::Comparison::LessThanOrEqual(0), 1));
    }

    if let Some((value, used)) = parse_number(tokens) {
        let value = value as i32;
        let first_word = tokens.first().and_then(Token::as_word);
        if matches!(first_word, Some("a" | "an")) {
            return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), used));
        }
        if tokens.get(used).is_some_and(|token| token.is_word("or"))
            && tokens
                .get(used + 1)
                .is_some_and(|token| token.is_word("more") || token.is_word("greater"))
        {
            return Ok((
                crate::effect::Comparison::GreaterThanOrEqual(value),
                used + 2,
            ));
        }
        if tokens.get(used).is_some_and(|token| token.is_word("or"))
            && tokens
                .get(used + 1)
                .is_some_and(|token| token.is_word("less") || token.is_word("fewer"))
        {
            return Ok((crate::effect::Comparison::LessThanOrEqual(value), used + 2));
        }
        return Ok((crate::effect::Comparison::Equal(value), used));
    }

    if allow_default_one {
        return Ok((crate::effect::Comparison::GreaterThanOrEqual(1), 0));
    }

    Err(CardTextError::ParseError(
        "missing quantity in static condition".to_string(),
    ))
}

fn parse_permanent_card_count_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let token_words = words(tokens);
    if !token_words.starts_with(&["permanent", "card"])
        && !token_words.starts_with(&["permanent", "cards"])
    {
        return None;
    }

    let mut filter = ObjectFilter::default();
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];

    for (idx, word) in token_words.iter().enumerate() {
        if let Some(zone) = parse_zone_word(word) {
            filter.zone = Some(zone);
            if idx > 0 {
                match token_words[idx - 1] {
                    "your" => filter.owner = Some(PlayerFilter::You),
                    "opponent" | "opponents" => filter.owner = Some(PlayerFilter::Opponent),
                    _ => {}
                }
            }
        }
    }

    filter.zone.map(|_| filter)
}

fn parse_static_condition_clause(tokens: &[Token]) -> Result<StaticCondition, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    let clause_words = words(&tokens);
    if clause_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing condition clause after 'as long as'".to_string(),
        ));
    }

    if clause_words == ["this", "creature", "is", "equipped"]
        || clause_words == ["it", "is", "equipped"]
    {
        return Ok(StaticCondition::SourceIsEquipped);
    }
    if clause_words == ["equipped", "creature", "is", "tapped"] {
        return Ok(StaticCondition::EquippedCreatureTapped);
    }
    if clause_words == ["equipped", "creature", "is", "untapped"] {
        return Ok(StaticCondition::EquippedCreatureUntapped);
    }
    if clause_words == ["it", "is", "your", "turn"] || clause_words == ["its", "your", "turn"] {
        return Ok(StaticCondition::YourTurn);
    }
    if clause_words == ["it", "is", "not", "your", "turn"]
        || clause_words == ["its", "not", "your", "turn"]
    {
        return Ok(StaticCondition::NotYourTurn);
    }

    if clause_words.starts_with(&["there", "are"]) {
        let quantified = &tokens[2..];
        let (comparison, used) = parse_static_quantity_prefix(quantified, false)?;
        let mut filter_tokens = &quantified[used..];
        if filter_tokens
            .first()
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            filter_tokens = &filter_tokens[1..];
        }
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing object phrase in static condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let filter = parse_permanent_card_count_filter(filter_tokens)
            .or_else(|| parse_object_filter(filter_tokens, false).ok())
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counted object phrase in static condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        return Ok(StaticCondition::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    let control_prefix_len = if clause_words.starts_with(&["you", "control"])
        || clause_words.starts_with(&["you", "controls"])
        || clause_words.starts_with(&["opponent", "control"])
        || clause_words.starts_with(&["opponent", "controls"])
        || clause_words.starts_with(&["opponents", "control"])
        || clause_words.starts_with(&["opponents", "controls"])
    {
        2
    } else {
        0
    };
    if control_prefix_len > 0 {
        let quantified = &tokens[control_prefix_len..];
        let (comparison, used) = parse_static_quantity_prefix(quantified, true)?;
        let mut filter_tokens: Vec<Token> = tokens[..control_prefix_len].to_vec();
        filter_tokens.extend_from_slice(&quantified[used..]);
        let filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported control condition filter (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        return Ok(StaticCondition::CountComparison {
            count: AnthemCountExpression::MatchingFilter(filter),
            comparison,
            display: Some(clause_words.join(" ")),
        });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported static condition clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_anthem_for_each_expression(
    tokens: &[Token],
) -> Result<AnthemCountExpression, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if !words_start_with(&tokens, &["for", "each"]) {
        return Err(CardTextError::ParseError(
            "missing 'for each' in anthem scaling clause".to_string(),
        ));
    }
    let rest = &tokens[2..];
    if rest.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object phrase after 'for each'".to_string(),
        ));
    }

    if words_start_with(rest, &["basic", "land", "type", "among"]) {
        let filter_tokens = &rest[4..];
        let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported domain count filter (clause: '{}')",
                words(&tokens).join(" ")
            ))
        })?;
        return Ok(AnthemCountExpression::BasicLandTypesAmong(filter));
    }

    if let Some(attached_idx) = rest.iter().position(|token| token.is_word("attached")) {
        let filter_tokens = &rest[..attached_idx];
        let tail_words = words(&rest[attached_idx + 1..]);
        let attached_to_source = tail_words == ["to", "it"]
            || tail_words == ["to", "this", "creature"]
            || tail_words == ["to", "this", "permanent"];
        if attached_to_source {
            let filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported attached-object filter in anthem scaling clause (clause: '{}')",
                    words(&tokens).join(" ")
                ))
            })?;
            return Ok(AnthemCountExpression::AttachedToSource(filter));
        }
    }

    let filter = parse_object_filter(rest, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported 'for each' filter in anthem clause (clause: '{}')",
            words(&tokens).join(" ")
        ))
    })?;
    Ok(AnthemCountExpression::MatchingFilter(filter))
}

fn parse_anthem_prefix_condition(
    tokens: &[Token],
    get_idx: usize,
) -> Result<(Option<StaticCondition>, usize), CardTextError> {
    if words_start_with(tokens, &["during", "turns", "other", "than", "yours"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(5);
        return Ok((Some(StaticCondition::NotYourTurn), subject_start));
    }
    if words_start_with(tokens, &["during", "your", "turn"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .unwrap_or(3);
        return Ok((Some(StaticCondition::YourTurn), subject_start));
    }

    if words_start_with(tokens, &["as", "long", "as"]) {
        let subject_start = tokens[..get_idx]
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
            .map(|idx| idx + 1)
            .or_else(|| find_source_reference_start(&tokens[..get_idx]))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing subject boundary in leading static condition clause (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
        if subject_start <= 3 {
            return Err(CardTextError::ParseError(format!(
                "missing condition after leading 'as long as' clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let condition_tokens = trim_commas(&tokens[3..subject_start]);
        let condition = parse_static_condition_clause(&condition_tokens)?;
        return Ok((Some(condition), subject_start));
    }

    Ok((None, 0))
}

fn parse_anthem_clause(
    tokens: &[Token],
    get_idx: usize,
    tail_end: usize,
) -> Result<ParsedAnthemClause, CardTextError> {
    let (prefix_condition, subject_start) = parse_anthem_prefix_condition(tokens, get_idx)?;
    let subject_tokens = trim_commas(&tokens[subject_start..get_idx]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing anthem subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;

    let modifier_token = tokens
        .get(get_idx + 1)
        .and_then(Token::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in anthem clause (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier(modifier_token).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in anthem clause (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let tail_tokens = trim_edge_punctuation(&tokens[get_idx + 2..tail_end]);
    let mut scale: Option<AnthemCountExpression> = None;
    let mut suffix_condition: Option<StaticCondition> = None;
    if !tail_tokens.is_empty() {
        if words_start_with(&tail_tokens, &["for", "each"]) {
            scale = Some(parse_anthem_for_each_expression(&tail_tokens)?);
        } else if words_start_with(&tail_tokens, &["as", "long", "as"]) {
            suffix_condition = Some(parse_static_condition_clause(&tail_tokens[3..])?);
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing anthem clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }

    let condition = match (prefix_condition, suffix_condition) {
        (Some(_prefix), Some(_)) => {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        (Some(condition), None) | (None, Some(condition)) => Some(condition),
        (None, None) => None,
    };

    let (power, toughness) = if let Some(scale_expr) = scale {
        (
            AnthemValue::scaled(base_power, scale_expr.clone()),
            AnthemValue::scaled(base_toughness, scale_expr),
        )
    } else {
        (
            AnthemValue::Fixed(base_power),
            AnthemValue::Fixed(base_toughness),
        )
    };

    parser_trace_stack("parse_static:anthem-clause:matched", tokens);
    Ok(ParsedAnthemClause {
        subject,
        power,
        toughness,
        condition,
    })
}

fn build_anthem_static_ability(clause: &ParsedAnthemClause) -> StaticAbility {
    let mut anthem = match &clause.subject {
        AnthemSubjectAst::Source => Anthem::for_source(0, 0),
        AnthemSubjectAst::Filter(filter) => Anthem::new(filter.clone(), 0, 0),
    }
    .with_values(clause.power.clone(), clause.toughness.clone());

    if let Some(condition) = &clause.condition {
        anthem = anthem.with_condition(condition.clone());
    }

    StaticAbility::new(anthem)
}

#[derive(Debug)]
struct TypeColorAdditionClause {
    added_colors: ColorSet,
    set_colors: ColorSet,
    card_types: Vec<CardType>,
    subtypes: Vec<Subtype>,
}

fn parse_type_color_addition_clause(
    tokens: &[Token],
) -> Result<Option<TypeColorAdditionClause>, CardTextError> {
    let words = words(tokens);
    if words.len() < 7 || words.first() != Some(&"is") {
        return Ok(None);
    }

    let Some(addition_idx) = words
        .windows(5)
        .position(|window| window == ["in", "addition", "to", "its", "other"])
    else {
        return Ok(None);
    };
    if addition_idx <= 1 {
        return Ok(None);
    }

    let scope_words = &words[addition_idx + 5..];
    let mut allow_colors = false;
    let mut allow_types = false;
    let mut segment_start = 0usize;
    for idx in 0..=scope_words.len() {
        let is_boundary = idx == scope_words.len() || scope_words[idx] == "and";
        if !is_boundary {
            continue;
        }
        if segment_start == idx {
            segment_start = idx + 1;
            continue;
        }
        let segment = &scope_words[segment_start..idx];
        segment_start = idx + 1;
        if segment.len() == 1 && (segment[0] == "color" || segment[0] == "colors") {
            allow_colors = true;
            continue;
        }
        if matches!(segment.last().copied(), Some("type" | "types"))
            && segment[..segment.len() - 1]
                .iter()
                .all(|word| is_type_scope_qualifier_word(word))
        {
            allow_types = true;
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported in-addition scope in type/color clause (clause: '{}')",
            words.join(" ")
        )));
    }
    if !allow_colors && !allow_types {
        return Ok(None);
    }

    let descriptor_words = words[1..addition_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word) && *word != "and")
        .collect::<Vec<_>>();
    if descriptor_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing type/color descriptors in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut added_colors = ColorSet::new();
    let mut set_colors = ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();
    for descriptor in descriptor_words {
        if let Some(color) = parse_color(descriptor) {
            if allow_colors {
                added_colors = added_colors.union(color);
            } else if allow_types {
                // "is black Zombie in addition to its other creature types"
                // sets color while only preserving existing types.
                set_colors = set_colors.union(color);
            } else {
                return Err(CardTextError::ParseError(format!(
                    "color descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                    descriptor,
                    words.join(" ")
                )));
            }
            continue;
        }

        if let Some(card_type) = parse_card_type(descriptor) {
            if allow_types {
                if !card_types.contains(&card_type) {
                    card_types.push(card_type);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "card type descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        if let Some(subtype) = parse_subtype_word(descriptor)
            .or_else(|| descriptor.strip_suffix('s').and_then(parse_subtype_word))
        {
            if allow_types {
                if !subtypes.contains(&subtype) {
                    subtypes.push(subtype);
                }
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "subtype descriptor '{}' not allowed by in-addition scope (clause: '{}')",
                descriptor,
                words.join(" ")
            )));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported descriptor '{}' in type/color addition clause (clause: '{}')",
            descriptor,
            words.join(" ")
        )));
    }

    if added_colors.is_empty()
        && set_colors.is_empty()
        && card_types.is_empty()
        && subtypes.is_empty()
    {
        return Err(CardTextError::ParseError(format!(
            "missing type/color additions in in-addition clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(Some(TypeColorAdditionClause {
        added_colors,
        set_colors,
        card_types,
        subtypes,
    }))
}

fn is_type_scope_qualifier_word(word: &str) -> bool {
    parse_card_type(word).is_some()
        || matches!(
            word,
            "card" | "creature" | "permanent" | "basic" | "legendary" | "snow" | "nonbasic"
        )
}

fn parse_anthem_and_type_color_addition_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let words = words(tokens);
    if words
        .windows(4)
        .any(|window| window == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }

    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };

    let and_idx = tokens
        .iter()
        .enumerate()
        .find_map(|(idx, token)| (idx > get_idx && token.is_word("and")).then_some(idx));
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };

    let addition_tokens = &tokens[and_idx + 1..];
    let Some(additions) = parse_type_color_addition_clause(addition_tokens)? else {
        return Ok(None);
    };

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let AnthemSubjectAst::Filter(filter) = &clause.subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported source-only type/color addition clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let mut result = vec![build_anthem_static_ability(&clause)];
    if !additions.set_colors.is_empty() {
        result.push(StaticAbility::set_colors(
            filter.clone(),
            additions.set_colors,
        ));
    }
    if !additions.added_colors.is_empty() {
        result.push(StaticAbility::add_colors(
            filter.clone(),
            additions.added_colors,
        ));
    }
    if !additions.card_types.is_empty() {
        result.push(StaticAbility::add_card_types(
            filter.clone(),
            additions.card_types,
        ));
    }
    if !additions.subtypes.is_empty() {
        result.push(StaticAbility::add_subtypes(
            filter.clone(),
            additions.subtypes,
        ));
    }
    Ok(Some(result))
}

fn parse_anthem_and_keyword_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    // "until end of turn" indicates a temporary effect, not a permanent anthem.
    if clause_words
        .windows(4)
        .any(|w| w == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }
    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let have_idx = clause_words
        .iter()
        .position(|word| *word == "have" || *word == "has");

    let (Some(get_idx), Some(have_idx)) = (get_idx, have_idx) else {
        return Ok(None);
    };

    if have_idx <= get_idx {
        return Ok(None);
    }

    let have_token_idx = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > get_idx && (token.is_word("have") || token.is_word("has"))).then_some(idx)
    });
    let Some(have_token_idx) = have_token_idx else {
        return Ok(None);
    };

    let mut ability_tokens = trim_commas(&tokens[have_token_idx + 1..]);
    let mut trailing_condition: Option<StaticCondition> = None;
    if let Some(as_long_idx) = words(&ability_tokens)
        .windows(3)
        .position(|window| window == ["as", "long", "as"])
    {
        let as_token_idx = token_index_for_word_index(&ability_tokens, as_long_idx)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map trailing 'as long as' keyword condition (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let condition_start_idx = token_index_for_word_index(&ability_tokens, as_long_idx + 3)
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let ability_head = trim_commas(&ability_tokens[..as_token_idx]);
        if ability_head.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing granted keyword list before trailing condition (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let condition_tokens = trim_commas(&ability_tokens[condition_start_idx..]);
        if condition_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing condition after trailing 'as long as' keyword clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        trailing_condition = Some(parse_static_condition_clause(&condition_tokens)?);
        ability_tokens = ability_head;
    }

    let Some(actions) = parse_ability_line(&ability_tokens) else {
        return Ok(None);
    };
    reject_unimplemented_keyword_actions(&actions, &clause_words.join(" "))?;
    let abilities: Vec<StaticAbility> = actions
        .into_iter()
        .filter_map(keyword_action_to_static_ability)
        .collect();
    if abilities.is_empty() {
        return Ok(None);
    }

    let clause_tail_end = if have_token_idx > get_idx + 2
        && tokens
            .get(have_token_idx - 1)
            .is_some_and(|token| token.is_word("and"))
    {
        have_token_idx - 1
    } else {
        have_token_idx
    };
    let mut clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
    if let Some(condition) = trailing_condition {
        if clause.condition.is_some() {
            return Err(CardTextError::ParseError(format!(
                "multiple anthem conditions are not supported (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        clause.condition = Some(condition);
    }
    let mut result = vec![build_anthem_static_ability(&clause)];
    for ability in abilities {
        let granted = match &clause.subject {
            AnthemSubjectAst::Source => GrantAbility::source(ability),
            AnthemSubjectAst::Filter(filter) => GrantAbility::new(filter.clone(), ability),
        };
        let granted = if let Some(condition) = &clause.condition {
            granted.with_condition(condition.clone())
        } else {
            granted
        };
        result.push(StaticAbility::new(granted));
    }
    Ok(Some(result))
}

fn parse_gets_and_attacks_each_combat_if_able_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(and_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > get_idx && token.is_word("and")).then_some(idx)
    }) else {
        return Ok(None);
    };
    let Some(attack_idx) = tokens.iter().enumerate().find_map(|(idx, token)| {
        (idx > and_idx && (token.is_word("attack") || token.is_word("attacks"))).then_some(idx)
    }) else {
        return Ok(None);
    };

    let attack_tail = words(&tokens[attack_idx..]);
    if attack_tail.as_slice() != ["attacks", "each", "combat", "if", "able"]
        && attack_tail.as_slice() != ["attack", "each", "combat", "if", "able"]
    {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let mut result = vec![build_anthem_static_ability(&clause)];
    let granted = match &clause.subject {
        AnthemSubjectAst::Source => GrantAbility::source(StaticAbility::must_attack()),
        AnthemSubjectAst::Filter(filter) => {
            GrantAbility::new(filter.clone(), StaticAbility::must_attack())
        }
    };
    let granted = if let Some(condition) = &clause.condition {
        granted.with_condition(condition.clone())
    } else {
        granted
    };
    result.push(StaticAbility::new(granted));

    if result.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "failed to parse gets-and-attacks clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(result))
}

fn parse_anthem_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    // Targeted "gets +N/+N" text is usually a one-shot spell/ability effect,
    // not a global/static anthem.
    if words.contains(&"target") {
        return Ok(None);
    }
    // "until end of turn" indicates a temporary effect, not a permanent anthem.
    if words
        .windows(4)
        .any(|w| w == ["until", "end", "of", "turn"])
    {
        return Ok(None);
    }

    let get_idx = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"));
    let Some(get_idx) = get_idx else {
        return Ok(None);
    };
    let clause = parse_anthem_clause(tokens, get_idx, tokens.len())?;
    Ok(Some(build_anthem_static_ability(&clause)))
}

fn parse_has_base_power_toughness_static_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words_all = words(tokens);
    let Some(has_idx) = words_all
        .iter()
        .position(|word| *word == "has" || *word == "have")
    else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.contains(&"target") {
        return Ok(None);
    }
    if subject_words.starts_with(&["until", "end", "of", "turn"])
        || subject_words.starts_with(&["until", "your", "next", "turn"])
    {
        return Ok(None);
    }

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5 || !rest_words.starts_with(&["base", "power", "and", "toughness"]) {
        return Ok(None);
    }
    let tail = &rest_words[5..];
    if !tail.is_empty() {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let subject = parse_anthem_subject(&subject_tokens)?;
    let filter = match subject {
        AnthemSubjectAst::Source => ObjectFilter::source(),
        AnthemSubjectAst::Filter(filter) => filter,
    };

    Ok(Some(StaticAbility::set_base_power_toughness(
        filter, power, toughness,
    )))
}

fn parse_enters_tapped_with_counters_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let enters_idx = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enters_idx) = enters_idx else {
        return Ok(None);
    };
    let with_idx = clause_words.iter().position(|word| *word == "with");
    let Some(with_idx) = with_idx else {
        return Ok(None);
    };
    if with_idx <= enters_idx {
        return Ok(None);
    }

    let tapped_between = clause_words[enters_idx + 1..with_idx]
        .iter()
        .any(|word| *word == "tapped");
    if !tapped_between {
        return Ok(None);
    }
    if !clause_words
        .iter()
        .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }
    if !is_source_reference_words(&clause_words[..enters_idx]) {
        return Ok(None);
    }

    let Some(counters) = parse_enters_with_counters_line(tokens)? else {
        return Ok(None);
    };

    Ok(Some(vec![StaticAbility::enters_tapped_ability(), counters]))
}

fn parse_enters_with_counters_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let enters_idx = words
        .iter()
        .position(|word| *word == "enters")
        .unwrap_or(usize::MAX);
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enters_idx) else {
        return Ok(None);
    };
    if tokens[..enter_token_idx].iter().any(|token| {
        matches!(
            token,
            Token::Period(_) | Token::Colon(_) | Token::Semicolon(_)
        )
    }) {
        return Ok(None);
    }
    let subject_words = words.get(..enters_idx).unwrap_or_default();
    if !is_source_reference_words(subject_words) {
        return Ok(None);
    }
    if !words.contains(&"with")
        || !words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let with_idx = tokens
        .iter()
        .position(|token| token.is_word("with"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'with' in enters-with-counters clause".to_string())
        })?;
    let after_with = &tokens[with_idx + 1..];
    let (mut count, used) = parse_value(after_with).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter count in self ETB counters (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let counter_type = parse_counter_type_from_tokens(&after_with[used..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for self ETB counters (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let counter_idx = after_with
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter keyword for self ETB counters (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let mut tail = &after_with[counter_idx + 1..];
    if tail.first().is_some_and(|token| token.is_word("on")) {
        tail = &tail[1..];
    }
    if tail.first().is_some_and(|token| token.is_word("it")) {
        tail = &tail[1..];
    } else if tail
        .first()
        .is_some_and(|token| token.is_word("this") || token.is_word("thiss"))
    {
        tail = &tail[1..];
        if let Some(word) = tail.first().and_then(Token::as_word)
            && (matches!(word, "source" | "spell" | "card")
                || word == "creature"
                || word == "permanent"
                || parse_card_type(word).is_some())
        {
            tail = &tail[1..];
        }
    }
    let tail = trim_commas(tail);
    let tail_has_words = tail.iter().any(|token| token.as_word().is_some());
    if tail_has_words {
        count = parse_where_x_value_clause(&tail).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported trailing self ETB counter clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    }

    Ok(Some(StaticAbility::enters_with_counters_value(
        counter_type,
        count,
    )))
}

fn parse_where_x_value_clause(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    // where X is the total number of cards in all players' hands
    if words.contains(&"cards")
        && words.contains(&"in")
        && words.contains(&"all")
        && words.contains(&"players")
        && (words.contains(&"hand") || words.contains(&"hands"))
    {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Hand);
        return Some(Value::Count(filter));
    }

    // where X is the number of cards in your hand
    if words.contains(&"cards")
        && words.contains(&"in")
        && words.contains(&"your")
        && (words.contains(&"hand") || words.contains(&"hands"))
    {
        return Some(Value::CardsInHand(PlayerFilter::You));
    }

    // where X is the number of <objects>
    if let Some(value) = parse_where_x_is_number_of_filter_value(tokens) {
        return Some(value);
    }

    None
}

fn parse_where_x_is_number_of_filter_value(tokens: &[Token]) -> Option<Value> {
    let words = words(tokens);
    if !words.starts_with(&["where", "x", "is"]) {
        return None;
    }

    if words.contains(&"creature") && words.contains(&"type") && words.contains(&"common") {
        return None;
    }

    let number_idx = words.iter().position(|word| *word == "number")?;
    if words.get(number_idx + 1).copied() != Some("of") {
        return None;
    }

    let object_start_word_idx = number_idx + 2;
    let mut seen_words = 0usize;
    let mut object_start_token_idx = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_words == object_start_word_idx {
            object_start_token_idx = Some(idx);
            break;
        }
        seen_words += 1;
    }
    let object_start_token_idx = object_start_token_idx?;
    let filter_tokens = &tokens[object_start_token_idx..];
    let filter = parse_object_filter(filter_tokens, false).ok()?;
    Some(Value::Count(filter))
}

fn token_index_for_word_index(tokens: &[Token], word_index: usize) -> Option<usize> {
    let mut seen_words = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_none() {
            continue;
        }
        if seen_words == word_index {
            return Some(idx);
        }
        seen_words += 1;
    }
    None
}

fn parse_enters_tapped_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped = clause_words.contains(&"enter") || clause_words.contains(&"enters");
        let has_tapped = clause_words.contains(&"tapped");
        if has_enters_tapped && has_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if clause_words.contains(&"unless") {
        return Ok(None);
    }
    let enter_word_idx = clause_words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if !clause_words
        .iter()
        .skip(enter_word_idx + 1)
        .any(|word| *word == "tapped")
    {
        return Ok(None);
    }
    if clause_words.first().copied() == Some("this") {
        return Ok(None);
    }
    let before_enter = &tokens[..enter_token_idx];
    let before_words = words(before_enter);
    let mut controller_override: Option<PlayerFilter> = None;
    let mut filter_end = before_enter.len();
    let find_suffix_cut = |suffix_len: usize| {
        token_index_for_word_index(before_enter, before_words.len().saturating_sub(suffix_len))
            .unwrap_or(before_enter.len())
    };
    if before_words.ends_with(&["played", "by", "your", "opponents"]) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if before_words.ends_with(&["played", "by", "an", "opponent"])
        || before_words.ends_with(&["played", "by", "a", "opponent"])
    {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(4);
    } else if before_words.ends_with(&["played", "by", "opponents"]) {
        controller_override = Some(PlayerFilter::Opponent);
        filter_end = find_suffix_cut(3);
    }
    let mut filter = parse_object_filter(&before_enter[..filter_end], false)?;
    if let Some(controller) = controller_override {
        filter.controller = Some(controller);
    }
    Ok(Some(StaticAbility::enters_tapped_for_filter(filter)))
}

fn parse_conditional_enters_tapped_unless_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"enters") && !clause_words.contains(&"enter") {
        return Ok(None);
    }
    if !clause_words.contains(&"tapped") || !clause_words.contains(&"unless") {
        return Ok(None);
    }

    let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) else {
        return Ok(None);
    };
    let condition_words = words(&tokens[unless_idx + 1..]);
    if condition_words.starts_with(&["you", "control", "two", "or", "more", "other", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_other_lands(),
        ));
    }
    if condition_words.starts_with(&["you", "control", "two", "or", "fewer", "other", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_fewer_other_lands(),
        ));
    }
    if condition_words.starts_with(&["you", "control", "two", "or", "more", "basic", "lands"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_control_two_or_more_basic_lands(),
        ));
    }
    if condition_words.starts_with(&["a", "player", "has", "13", "or", "less", "life"])
        || condition_words.starts_with(&["a", "player", "has", "thirteen", "or", "less", "life"])
    {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_a_player_has_13_or_less_life(),
        ));
    }
    if condition_words.starts_with(&["you", "have", "two", "or", "more", "opponents"]) {
        return Ok(Some(
            StaticAbility::enters_tapped_unless_two_or_more_opponents(),
        ));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported enters tapped unless condition (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_enters_with_additional_counter_for_filter_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let enter_word_idx = words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if tokens[..enter_token_idx].iter().any(|token| {
        matches!(
            token,
            Token::Period(_) | Token::Colon(_) | Token::Semicolon(_)
        )
    }) {
        return Ok(None);
    }

    if words.first().copied() == Some("this") {
        return Ok(None);
    }
    if !words.contains(&"battlefield")
        || !words.contains(&"with")
        || !words.contains(&"additional")
        || !words
            .iter()
            .any(|word| *word == "counter" || *word == "counters")
    {
        return Ok(None);
    }

    let filter = parse_object_filter(&tokens[..enter_token_idx], false)?;

    let additional_idx = tokens
        .iter()
        .position(|token| token.is_word("additional"))
        .ok_or_else(|| {
            CardTextError::ParseError("missing 'additional' keyword for ETB counters".to_string())
        })?;
    let mut count = 1u32;
    if additional_idx > 0
        && let Some((parsed, _)) = parse_number(&tokens[additional_idx - 1..additional_idx])
    {
        count = parsed;
    } else if let Some((parsed, _)) = parse_number(&tokens[additional_idx + 1..]) {
        count = parsed;
    }

    let counter_type = parse_counter_type_from_tokens(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type for ETB replacement (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(StaticAbility::enters_with_counters_for_filter(
        filter,
        counter_type,
        count,
    )))
}

fn parse_creatures_cant_block_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["creatures", "cant", "block"] {
        return Ok(Some(StaticAbility::grant_ability(
            ObjectFilter::creature(),
            StaticAbility::cant_block(),
        )));
    }
    Ok(None)
}

fn parse_equipped_creature_has_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let clause_text = words.join(" ");
    if words.len() < 4 || words[0] != "equipped" || words[1] != "creature" || words[2] != "has" {
        return Ok(None);
    }

    let ability_tokens = &tokens[3..];
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let mut abilities = Vec::new();
    for segment in split_on_and(ability_tokens) {
        if segment.is_empty() {
            continue;
        }
        let Some(action) = parse_ability_phrase(&segment) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        if let Some(ability) = keyword_action_to_static_ability(action) {
            abilities.push(ability);
        }
    }

    if abilities.is_empty() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::equipment_grant(abilities)))
}

fn parse_attached_has_keywords_and_triggered_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }

    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let Some(and_idx) = ability_tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if and_idx == 0 || and_idx + 1 >= ability_tokens.len() {
        return Ok(None);
    }

    let trigger_starts = ability_tokens
        .get(and_idx + 1)
        .is_some_and(|token| token.is_word("when") || token.is_word("whenever"))
        || is_at_trigger_intro(&ability_tokens, and_idx + 1);
    if !trigger_starts {
        return Ok(None);
    }

    let keyword_tokens = trim_edge_punctuation(&ability_tokens[..and_idx]);
    if keyword_tokens.is_empty() {
        return Ok(None);
    }

    let clause_text = line_words.join(" ");
    let mut keyword_statics = Vec::new();
    for segment in split_on_and(&keyword_tokens) {
        if segment.is_empty() {
            continue;
        }
        let Some(action) = parse_ability_phrase(&segment) else {
            return Ok(None);
        };
        reject_unimplemented_keyword_actions(std::slice::from_ref(&action), &clause_text)?;
        let Some(static_ability) = keyword_action_to_static_ability(action) else {
            continue;
        };
        keyword_statics.push(static_ability);
    }
    if keyword_statics.is_empty() {
        return Ok(None);
    }

    let trigger_tokens = trim_edge_punctuation(&ability_tokens[and_idx + 1..]);
    if trigger_tokens.is_empty() {
        return Ok(None);
    }
    let triggered = match parse_triggered_line(&trigger_tokens)? {
        LineAst::Triggered {
            trigger,
            effects,
            once_each_turn,
        } => {
            let (compiled_effects, choices) = compile_trigger_effects(Some(&trigger), &effects)?;
            Ability {
                kind: AbilityKind::Triggered(TriggeredAbility {
                    trigger: compile_trigger_spec(trigger),
                    effects: compiled_effects,
                    choices,
                    intervening_if: None,
                    once_each_turn,
                }),
                functional_zones: vec![Zone::Battlefield],
                text: Some(words(&trigger_tokens).join(" ")),
            }
        }
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported attached triggered grant clause (clause: '{}')",
                clause_text
            )));
        }
    };

    let subject = match parse_anthem_subject(&tokens[..has_idx]) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let filter = match subject {
        AnthemSubjectAst::Filter(filter) => filter,
        AnthemSubjectAst::Source => ObjectFilter::source(),
    };

    let mut static_abilities = Vec::new();
    for ability in keyword_statics {
        static_abilities.push(StaticAbility::grant_ability(filter.clone(), ability));
    }
    let subject_text = words(&tokens[..has_idx]).join(" ");
    let display = format!("{subject_text} has {}", words(&trigger_tokens).join(" "));
    static_abilities.push(StaticAbility::attached_ability_grant(triggered, display));

    Ok(Some(static_abilities))
}

fn parse_attached_gets_and_has_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 6 {
        return Ok(None);
    }
    let is_enchanted = line_words.starts_with(&["enchanted", "creature"]);
    let is_equipped = line_words.starts_with(&["equipped", "creature"]);
    if !is_enchanted && !is_equipped {
        return Ok(None);
    }

    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    if !(get_idx < and_idx && and_idx < has_idx) {
        return Ok(None);
    }

    let clause = parse_anthem_clause(tokens, get_idx, and_idx)?;
    let anthem = build_anthem_static_ability(&clause);

    let ability_tokens = trim_edge_punctuation(&tokens[has_idx + 1..]);
    if ability_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing attached ability after 'has' (clause: '{}')",
            line_words.join(" ")
        )));
    }
    let ability_text = words(&ability_tokens).join(" ");
    let ability = Ability {
        kind: AbilityKind::Static(StaticAbility::custom(
            "attached_grant",
            ability_text.clone(),
        )),
        functional_zones: vec![Zone::Battlefield],
        text: Some(ability_text.clone()),
    };
    let subject = if is_enchanted {
        "enchanted creature"
    } else {
        "equipped creature"
    };
    let display = format!("{subject} has {ability_text}");
    let grant = StaticAbility::attached_ability_grant(ability, display);

    Ok(Some(vec![anthem, grant]))
}

fn parse_equipped_gets_and_has_activated_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 4 || line_words[0] != "equipped" || line_words[1] != "creature" {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    if has_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = &tokens[has_idx + 1..];
    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let Some(parsed) = parse_activated_line(ability_tokens)? else {
        if has_colon {
            return Err(CardTextError::ParseError(format!(
                "unsupported equipped activated-ability grant (clause: '{}')",
                line_words.join(" ")
            )));
        }
        return Ok(None);
    };

    let mut static_abilities = Vec::new();
    if let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
        && get_idx < has_idx
    {
        let clause_tail_end = if has_idx > get_idx + 2
            && tokens
                .get(has_idx - 1)
                .is_some_and(|token| token.is_word("and"))
        {
            has_idx - 1
        } else {
            has_idx
        };
        let clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
        static_abilities.push(build_anthem_static_ability(&clause));
    }

    let mut ability = parsed.ability;
    if ability.text.is_none() {
        ability.text = Some(words(ability_tokens).join(" "));
    }
    static_abilities.push(StaticAbility::attached_ability_grant(
        ability,
        line_words.join(" "),
    ));

    Ok(Some(static_abilities))
}

fn parse_may_choose_not_to_untap_during_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["you", "may", "choose", "not", "to", "untap"]) {
        return Ok(None);
    }
    if !words.ends_with(&["during", "your", "untap", "step"]) {
        return Ok(None);
    }
    if words.len() <= 10 {
        return Ok(None);
    }

    let subject_words = &words[6..words.len() - 4];
    let subject_allowed = matches!(
        subject_words,
        ["this"]
            | ["it"]
            | ["this", "artifact"]
            | ["this", "creature"]
            | ["this", "land"]
            | ["this", "permanent"]
            | ["this", "card"]
    );
    if !subject_allowed {
        return Ok(None);
    }

    let subject = subject_words.join(" ");
    let text = format!("You may choose not to untap {subject} during your untap step");
    Ok(Some(StaticAbility::custom(
        "may_choose_not_to_untap_during_your_untap_step",
        text,
    )))
}

fn parse_untap_during_each_other_players_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if !is_untap_during_each_other_players_untap_step_words(&line_words) {
        return Ok(None);
    }
    Ok(Some(StaticAbility::custom(
        "untap_during_each_other_players_untap_step",
        line_words.join(" "),
    )))
}

fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 {
        return Ok(None);
    }

    let prefix_len = [
        ["this", "doesnt", "untap", "during", "your", "untap", "step"].as_slice(),
        [
            "this", "land", "doesnt", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this",
            "artifact",
            "doesnt",
            "untap",
            "during",
            "your",
            "untap",
            "step",
        ]
        .as_slice(),
        [
            "this",
            "creature",
            "doesnt",
            "untap",
            "during",
            "your",
            "untap",
            "step",
        ]
        .as_slice(),
        ["this", "doesn't", "untap", "during", "your", "untap", "step"].as_slice(),
        [
            "this", "land", "doesn't", "untap", "during", "your", "untap", "step",
        ]
        .as_slice(),
        [
            "this",
            "artifact",
            "doesn't",
            "untap",
            "during",
            "your",
            "untap",
            "step",
        ]
        .as_slice(),
        [
            "this",
            "creature",
            "doesn't",
            "untap",
            "during",
            "your",
            "untap",
            "step",
        ]
        .as_slice(),
        ["this", "does", "not", "untap", "during", "your", "untap", "step"].as_slice(),
    ]
    .iter()
    .find(|pattern| clause_words.starts_with(pattern))
    .map(|pattern| pattern.len());

    if let Some(prefix_len) = prefix_len {
        let tail_tokens = trim_commas(&tokens[prefix_len..]);
        if tail_tokens.is_empty() {
            return Ok(Some(StaticAbility::doesnt_untap()));
        }
        let tail_words = words(&tail_tokens);
        if tail_words.first().copied() == Some("if") {
            let condition_tokens = trim_commas(&tail_tokens[1..]);
            if condition_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing condition after untap-step if-clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            let condition = parse_static_condition_clause(&condition_tokens)?;
            let granted =
                crate::static_abilities::GrantAbility::source(StaticAbility::doesnt_untap())
                    .with_condition(condition);
            return Ok(Some(StaticAbility::new(granted)));
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing untap-step clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let attached_subject = clause_words.starts_with(&["enchanted", "creature"])
        || clause_words.starts_with(&["equipped", "creature"]);
    if attached_subject {
        let remainder = &clause_words[2..];
        let attached_matches = matches!(
            remainder,
            ["doesnt", "untap", "during", "its", "controller", "untap", "step"]
                | ["doesnt", "untap", "during", "its", "controllers", "untap", "step"]
                | ["does", "not", "untap", "during", "its", "controller", "untap", "step"]
                | ["does", "not", "untap", "during", "its", "controllers", "untap", "step"]
        );

        if attached_matches {
            let text = clause_words.join(" ");
            let granted = Ability::static_ability(StaticAbility::doesnt_untap()).with_text(&text);
            return Ok(Some(StaticAbility::attached_ability_grant(granted, text)));
        }
    }

    Ok(None)
}

fn parse_flying_restriction_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let flying_only_matches = normalized.as_slice()
        == [
            "this",
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "flying",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
            ];

    if flying_only_matches {
        return Ok(Some(StaticAbility::flying_only_restriction()));
    }

    let flying_or_reach_matches = normalized.as_slice()
        == [
            "this",
            "cant",
            "be",
            "blocked",
            "except",
            "by",
            "creatures",
            "with",
            "flying",
            "or",
            "reach",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "cant",
                "be",
                "blocked",
                "except",
                "by",
                "creatures",
                "with",
                "flying",
                "or",
                "reach",
            ];

    if flying_or_reach_matches {
        return Ok(Some(StaticAbility::flying_restriction()));
    }

    Ok(None)
}

fn parse_can_block_only_flying_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let matches = normalized.as_slice()
        == [
            "this",
            "can",
            "block",
            "only",
            "creatures",
            "with",
            "flying",
        ]
        || normalized.as_slice()
            == [
                "this",
                "creature",
                "can",
                "block",
                "only",
                "creatures",
                "with",
                "flying",
            ]
        || normalized.as_slice() == ["can", "block", "only", "creatures", "with", "flying"]
        || normalized.as_slice() == ["this", "can", "block", "only", "creature", "with", "flying"]
        || normalized.as_slice()
            == [
                "this", "creature", "can", "block", "only", "creature", "with", "flying",
            ];

    if matches {
        return Ok(Some(StaticAbility::can_block_only_flying()));
    }

    Ok(None)
}

fn parse_assign_damage_as_unblocked_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    if normalized.first().copied() != Some("you") {
        return Ok(None);
    }

    let mut idx = 1;
    if normalized.get(idx) == Some(&"may") {
        idx += 1;
    }
    if normalized.get(idx) != Some(&"have") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) != Some(&"this") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) == Some(&"creature") {
        idx += 1;
    }

    let tail = &normalized[idx..];
    let matches =
        tail == [
            "assign", "its", "combat", "damage", "as", "though", "it", "werent", "blocked",
        ] || tail
            == [
                "assign", "its", "combat", "damage", "as", "though", "it", "wasnt", "blocked",
            ];

    if matches {
        return Ok(Some(StaticAbility::may_assign_damage_as_unblocked()));
    }

    Ok(None)
}

fn parse_grant_flash_to_noncreature_spells_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let mut idx = 0;
    if normalized.get(idx) != Some(&"you") {
        return Ok(None);
    }
    idx += 1;
    if normalized.get(idx) == Some(&"may") {
        idx += 1;
    }
    if normalized.get(idx) != Some(&"cast") {
        return Ok(None);
    }
    idx += 1;

    let tail = &normalized[idx..];
    let matches =
        tail == [
            "noncreature",
            "spells",
            "as",
            "though",
            "they",
            "had",
            "flash",
        ] || tail
            == [
                "noncreature",
                "spells",
                "as",
                "though",
                "they",
                "have",
                "flash",
            ];

    if matches {
        return Ok(Some(StaticAbility::grants(
            crate::grant::GrantSpec::flash_to_noncreature_spells(),
        )));
    }

    Ok(None)
}

fn parse_attacks_each_combat_if_able_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let Some(attack_idx) = words
        .iter()
        .position(|word| *word == "attack" || *word == "attacks")
    else {
        return Ok(None);
    };
    if words[attack_idx..] != ["attacks", "each", "combat", "if", "able"]
        && words[attack_idx..] != ["attack", "each", "combat", "if", "able"]
    {
        return Ok(None);
    }

    if attack_idx == 0 {
        return Ok(Some(StaticAbility::must_attack()));
    }

    let subject_tokens = trim_commas(&tokens[..attack_idx]);
    if subject_tokens.is_empty() {
        return Ok(Some(StaticAbility::must_attack()));
    }
    let subject = parse_anthem_subject(&subject_tokens)?;
    match subject {
        AnthemSubjectAst::Source => Ok(Some(StaticAbility::must_attack())),
        AnthemSubjectAst::Filter(filter) => Ok(Some(StaticAbility::grant_ability(
            filter,
            StaticAbility::must_attack(),
        ))),
    }
}

fn parse_additional_land_play_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice()
        == [
            "you",
            "may",
            "play",
            "an",
            "additional",
            "land",
            "on",
            "each",
            "of",
            "your",
            "turns",
        ]
    {
        return Ok(Some(StaticAbility::additional_land_play()));
    }
    Ok(None)
}

fn parse_play_lands_from_graveyard_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["you", "may", "play", "lands", "from", "your", "graveyard"] {
        let spec = crate::grant::GrantSpec::new(
            crate::grant::Grantable::play_from(),
            ObjectFilter::land(),
            Zone::Graveyard,
        );
        return Ok(Some(StaticAbility::grants(spec)));
    }
    Ok(None)
}

fn parse_pt_modifier(raw: &str) -> Result<(i32, i32), CardTextError> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() != 2 {
        return Err(CardTextError::ParseError(
            "missing power/toughness modifier".to_string(),
        ));
    }
    let power_str = parts[0].trim_start_matches('+');
    let toughness_str = parts[1].trim_start_matches('+');
    let power = power_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power modifier".to_string()))?;
    let toughness = toughness_str
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid toughness modifier".to_string()))?;
    Ok((power, toughness))
}

fn parse_signed_pt_component(raw: &str) -> Result<Value, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CardTextError::ParseError(
            "missing power/toughness component".to_string(),
        ));
    }

    let (sign, value_text) = if let Some(rest) = trimmed.strip_prefix('+') {
        (1, rest)
    } else if let Some(rest) = trimmed.strip_prefix('-') {
        (-1, rest)
    } else {
        (1, trimmed)
    };

    if value_text.eq_ignore_ascii_case("x") {
        return Ok(match sign {
            1 => Value::X,
            -1 => Value::XTimes(-1),
            _ => Value::XTimes(sign),
        });
    }

    let parsed = value_text
        .parse::<i32>()
        .map_err(|_| CardTextError::ParseError("invalid power/toughness component".to_string()))?;
    Ok(Value::Fixed(parsed * sign))
}

fn parse_pt_modifier_values(raw: &str) -> Result<(Value, Value), CardTextError> {
    let parts: Vec<&str> = raw.split('/').collect();
    if parts.len() != 2 {
        return Err(CardTextError::ParseError(
            "missing power/toughness modifier".to_string(),
        ));
    }

    let power = parse_signed_pt_component(parts[0])?;
    let toughness = parse_signed_pt_component(parts[1])?;
    Ok((power, toughness))
}

fn parse_no_maximum_hand_size_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["you", "have", "no", "maximum", "hand", "size"] {
        return Ok(Some(StaticAbility::no_maximum_hand_size()));
    }
    Ok(None)
}

fn parse_library_of_leng_discard_replacement_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_effect_causes = words.windows(3).any(|w| w == ["effect", "causes", "you"]);
    let has_discard = words.contains(&"discard");
    let has_top = words.contains(&"top");
    let has_library = words.contains(&"library");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_effect_causes && has_discard && has_top && has_library && has_instead && has_graveyard {
        return Ok(Some(StaticAbility::library_of_leng_discard_replacement()));
    }

    Ok(None)
}

fn parse_toph_first_metalbender_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_nontoken = words.contains(&"nontoken");
    let has_artifact = words
        .iter()
        .any(|word| *word == "artifact" || *word == "artifacts");
    let has_you_control = words
        .windows(2)
        .any(|pair| pair == ["you", "control"] || pair == ["you", "controls"]);
    let has_land = words.iter().any(|word| *word == "land" || *word == "lands");
    let has_addition = words
        .windows(4)
        .any(|pair| pair == ["in", "addition", "to", "their"]);

    if has_nontoken && has_artifact && has_you_control && has_land && has_addition {
        return Ok(Some(StaticAbility::new(
            crate::static_abilities::TophFirstMetalbender,
        )));
    }

    Ok(None)
}

fn parse_discard_or_redirect_replacement_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let has_enter = words
        .iter()
        .any(|word| *word == "enter" || *word == "enters");
    let has_battlefield = words.contains(&"battlefield");
    let has_discard = words.contains(&"discard");
    let has_land = words.contains(&"land");
    let has_instead = words.contains(&"instead");
    let has_graveyard = words.contains(&"graveyard");

    if has_enter && has_battlefield && has_discard && has_land && has_instead && has_graveyard {
        return Ok(Some(StaticAbility::discard_or_redirect_replacement(
            ObjectFilter::default().with_type(CardType::Land),
            Zone::Graveyard,
        )));
    }

    Ok(None)
}

fn parse_pay_life_or_enter_tapped_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 8 {
        return Ok(None);
    }

    let starts_with_as_this = words.starts_with(&["as", "this"]);
    let has_pay = words.contains(&"pay");
    let has_life = words.contains(&"life");
    if !starts_with_as_this || !has_pay || !has_life {
        return Ok(None);
    }

    let Some(pay_idx) = tokens.iter().position(|token| token.is_word("pay")) else {
        return Err(CardTextError::ParseError(format!(
            "missing 'pay' keyword in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };
    if !words[..pay_idx]
        .iter()
        .any(|word| *word == "enter" || *word == "enters")
    {
        return Ok(None);
    }
    if !words[..pay_idx].contains(&"may") {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB prefix (clause: '{}')",
            words.join(" ")
        )));
    }

    let Some((value, _)) = parse_number(&tokens[pay_idx + 1..]) else {
        return Err(CardTextError::ParseError(format!(
            "missing life payment amount in pay-life ETB clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let if_dont_idx = words
        .windows(3)
        .position(|window| window == ["if", "you", "dont"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported pay-life ETB trailing clause (expected 'if you don't ...') (clause: '{}')",
                words.join(" ")
            ))
        })?;

    let trailing = &words[if_dont_idx + 3..];
    let valid_trailing = trailing.starts_with(&["it", "enters", "tapped"])
        || trailing.starts_with(&["it", "enter", "tapped"])
        || trailing.starts_with(&["it", "enters", "the", "battlefield", "tapped"])
        || trailing.starts_with(&["it", "enter", "the", "battlefield", "tapped"]);
    if !valid_trailing {
        return Err(CardTextError::ParseError(format!(
            "unsupported pay-life ETB trailing clause (clause: '{}')",
            words.join(" ")
        )));
    };

    parser_trace("parse_static:pay-life-etb:matched", tokens);
    Ok(Some(StaticAbility::pay_life_or_enter_tapped(value)))
}

fn parse_copy_activated_abilities_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 {
        return Ok(None);
    }

    let mut has_idx = None;
    for idx in 0..words.len().saturating_sub(4) {
        if words[idx] == "has"
            && words[idx + 1] == "all"
            && words[idx + 2] == "activated"
            && words[idx + 3] == "abilities"
            && words[idx + 4] == "of"
        {
            has_idx = Some(idx);
            break;
        }
    }
    let Some(has_idx) = has_idx else {
        return Ok(None);
    };

    let mut condition = None;
    let prefix = &words[..has_idx];
    if prefix.starts_with(&["as", "long", "as"])
        && prefix.contains(&"own")
        && prefix.contains(&"exiled")
        && prefix.contains(&"counter")
    {
        if let Some(counter_word) = prefix
            .iter()
            .zip(prefix.iter().skip(1))
            .find_map(|(word, next)| {
                if *next == "counter" {
                    Some(*word)
                } else {
                    None
                }
            })
            .and_then(parse_counter_type_word)
        {
            condition = Some(
                crate::static_abilities::CopyActivatedAbilitiesCondition::OwnsCardExiledWithCounter(
                    counter_word,
                ),
            );
        }
    }

    let after_of = &words[(has_idx + 5)..];
    let mut filter = None;
    if after_of.contains(&"land") || after_of.contains(&"lands") {
        filter = Some(ObjectFilter::land());
    } else if after_of.contains(&"creature") || after_of.contains(&"creatures") {
        let mut base = ObjectFilter::creature();
        if after_of.contains(&"control") {
            base = base.you_control();
        }
        filter = Some(base);
    } else if after_of.contains(&"card") && after_of.contains(&"exiled") {
        filter = Some(ObjectFilter {
            zone: Some(Zone::Exile),
            ..Default::default()
        });
    }

    let Some(filter) = filter else {
        return Ok(None);
    };

    let counter = after_of
        .iter()
        .zip(after_of.iter().skip(1))
        .find_map(|(word, next)| {
            if *next == "counter" {
                parse_counter_type_word(word)
            } else {
                None
            }
        });

    let exclude_source_name = words.windows(5).any(|window| {
        window == ["same", "name", "as", "this", "creature"]
            || window == ["same", "name", "as", "thiss", "creature"]
    });

    let mut ability = crate::static_abilities::CopyActivatedAbilities::new(filter)
        .with_exclude_source_name(exclude_source_name)
        .with_exclude_source_id(true)
        .with_display(words.join(" "));
    if let Some(counter) = counter {
        ability = ability.with_counter(counter);
    }
    if let Some(condition) = condition {
        ability = ability.with_condition(condition);
    }

    Ok(Some(StaticAbility::copy_activated_abilities(ability)))
}

fn parse_players_spend_mana_as_any_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.starts_with(&[
        "players", "may", "spend", "mana", "as", "though", "it", "were", "mana", "of", "any",
        "color",
    ]) {
        return Ok(Some(StaticAbility::spend_mana_as_any_color_players()));
    }

    Ok(None)
}

fn parse_source_activation_spend_mana_as_any_color_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&[
        "you",
        "may",
        "spend",
        "mana",
        "as",
        "though",
        "it",
        "were",
        "mana",
        "of",
        "any",
        "color",
        "to",
        "pay",
        "the",
        "activation",
        "costs",
        "of",
    ]) {
        return Ok(None);
    }

    if words
        .iter()
        .any(|word| *word == "abilities" || *word == "ability")
    {
        return Ok(Some(
            StaticAbility::spend_mana_as_any_color_activation_costs(),
        ));
    }

    Ok(None)
}

fn parse_enchanted_has_activated_ability_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let token_words = words(tokens);
    if !token_words.starts_with(&["enchanted"]) || !token_words.contains(&"has") {
        return Ok(None);
    }

    let Some(has_idx) = tokens.iter().position(|token| token.is_word("has")) else {
        return Ok(None);
    };
    let ability_tokens = &tokens[has_idx + 1..];
    let Some(parsed) = parse_activated_line(ability_tokens)? else {
        return Ok(None);
    };

    let mut ability = parsed.ability;
    if ability.text.is_none() {
        let text_words = words(ability_tokens).join(" ");
        ability.text = Some(text_words);
    }

    Ok(Some(StaticAbility::attached_ability_grant(
        ability,
        token_words.join(" "),
    )))
}

fn parse_filter_has_granted_ability_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    let Some(has_idx) = tokens
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
    else {
        return Ok(None);
    };
    if has_idx == 0 || has_idx + 1 >= tokens.len() {
        return Ok(None);
    }
    if tokens[..has_idx]
        .iter()
        .any(|token| token.is_word("get") || token.is_word("gets"))
    {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..has_idx]);
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(&subject_tokens);
    if subject_words.iter().any(|word| {
        matches!(
            *word,
            "deal"
                | "deals"
                | "create"
                | "creates"
                | "draw"
                | "draws"
                | "destroy"
                | "destroys"
                | "exile"
                | "exiles"
                | "return"
                | "returns"
                | "sacrifice"
                | "sacrifices"
                | "put"
                | "puts"
                | "gain"
                | "gains"
                | "lose"
                | "loses"
                | "discard"
                | "discards"
                | "counter"
                | "counters"
                | "search"
                | "reveals"
                | "investigate"
                | "investigates"
        )
    }) {
        return Ok(None);
    }
    if subject_words.contains(&"may") {
        return Ok(None);
    }
    let ability_tokens = &tokens[has_idx + 1..];
    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.is_word("when")
            || token.is_word("whenever")
            || (token.is_word("at")
                && ability_tokens
                    .get(1)
                    .is_some_and(|next| next.is_word("the")))
    });
    let mut granted_static: Option<StaticAbility> = None;
    let mut granted_ability: Option<Ability> = if has_colon {
        let Some(parsed) = parse_activated_line(ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        Some(parsed.ability)
    } else if let Some(parsed) = parse_cycling_line(ability_tokens)? {
        Some(parsed.ability)
    } else if looks_like_trigger {
        match parse_triggered_line(ability_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                once_each_turn,
            } => {
                let (compiled_effects, choices) =
                    compile_trigger_effects(Some(&trigger), &effects)?;
                Some(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger: compile_trigger_spec(trigger),
                        effects: compiled_effects,
                        choices,
                        intervening_if: None,
                        once_each_turn,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: None,
                })
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated/triggered ability clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    } else if let Some(abilities) = parse_static_ability_line(ability_tokens)? {
        if abilities.len() != 1 {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted static ability chain (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        granted_static = Some(
            abilities
                .into_iter()
                .next()
                .expect("single granted static ability"),
        );
        None
    } else {
        return Ok(None);
    };
    let subject = match parse_anthem_subject(&subject_tokens) {
        Ok(subject) => subject,
        Err(_) => return Ok(None),
    };
    let filter = match subject {
        AnthemSubjectAst::Filter(filter) => filter,
        AnthemSubjectAst::Source => ObjectFilter::source(),
    };
    if let Some(static_ability) = granted_static {
        return Ok(Some(StaticAbility::grant_ability(filter, static_ability)));
    }
    let mut ability = granted_ability
        .take()
        .ok_or_else(|| CardTextError::ParseError("missing granted ability".to_string()))?;
    if ability.text.is_none() {
        ability.text = Some(words(ability_tokens).join(" "));
    }

    let attached_subject = subject_words
        .first()
        .is_some_and(|word| *word == "enchanted" || *word == "equipped");
    if attached_subject {
        return Ok(Some(StaticAbility::attached_ability_grant(
            ability,
            clause_words.join(" "),
        )));
    }

    Ok(Some(StaticAbility::grant_object_ability_for_filter(
        filter,
        ability,
        clause_words.join(" "),
    )))
}

fn parse_activated_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(colon_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Colon(_)))
    else {
        return Ok(None);
    };

    let cost_tokens = &tokens[..colon_idx];
    let effect_tokens = &tokens[colon_idx + 1..];
    if cost_tokens.is_empty() || effect_tokens.is_empty() {
        return Ok(None);
    }

    let mut effect_sentences = split_on_period(effect_tokens);
    let functional_zones = infer_activated_functional_zones(cost_tokens, &effect_sentences);
    let mut timing = ActivationTiming::AnyTime;
    let mut mana_activation_condition: Option<ManaAbilityCondition> = None;
    let mut additional_activation_restrictions: Vec<String> = Vec::new();
    effect_sentences.retain(|sentence| {
        if is_activate_only_restriction_sentence(sentence) {
            if let Some(parsed_timing) = parse_activate_only_timing(sentence) {
                timing = parsed_timing;
            }
            if let Some(condition) = parse_activation_condition(sentence) {
                mana_activation_condition = Some(condition);
            }
            additional_activation_restrictions.push(words(sentence).join(" "));
            return false;
        }
        if is_trigger_only_restriction_sentence(sentence) {
            return false;
        }
        true
    });
    let mana_activation_condition =
        combine_mana_activation_condition(mana_activation_condition, timing.clone());
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let effect_words = words(primary_sentence);
        let is_primary_add_clause = matches!(
            effect_words.as_slice(),
            ["add", ..]
                | ["adds", ..]
                | ["you", "add", ..]
                | ["that", "player", "add", ..]
                | ["that", "player", "adds", ..]
                | ["target", "player", "add", ..]
                | ["target", "player", "adds", ..]
        );
        if is_primary_add_clause {
            let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
            let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

            let mut extra_effects = Vec::new();
            let mut extra_effects_ast = Vec::new();
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    let ast = parse_effect_sentence(sentence)?;
                    let compiled = compile_statement_effects(&ast)?;
                    extra_effects.extend(compiled);
                    extra_effects_ast.extend(ast);
                }
            }

            let add_token_idx = primary_sentence
                .iter()
                .position(|token| token.is_word("add"))
                .unwrap_or(0);
            let mana_tokens = &primary_sentence[add_token_idx + 1..];
            let mana_words = words(mana_tokens);
            let has_for_each_tail = mana_tokens
                .windows(2)
                .any(|window| window[0].is_word("for") && window[1].is_word("each"));
            let dynamic_amount = if has_for_each_tail {
                Some(
                    parse_dynamic_cost_modifier_value(mana_tokens)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported dynamic mana amount (clause: '{}')",
                            words(primary_sentence).join(" ")
                        ))
                    })?,
                )
            } else {
                parse_devotion_value_from_add_clause(mana_tokens)?
                    .or_else(|| parse_add_mana_equal_amount_value(mana_tokens))
            };

            let has_imprinted_colors = mana_words.contains(&"exiled")
                && (mana_words.contains(&"card") || mana_words.contains(&"cards"))
                && mana_words
                    .iter()
                    .any(|word| *word == "color" || *word == "colors");
            let has_any_combination_mana = mana_words
                .windows(3)
                .any(|window| window == ["any", "combination", "of"]);
            let has_any_choice_mana = mana_words.contains(&"any")
                && (mana_words.contains(&"color")
                    || mana_words.contains(&"type")
                    || has_any_combination_mana);
            let has_or_choice_mana = mana_words.contains(&"or");
            let has_chosen_color = mana_words.contains(&"chosen") && mana_words.contains(&"color");
            let uses_commander_identity = mana_words
                .iter()
                .any(|word| *word == "commander" || *word == "commanders")
                && mana_words.contains(&"identity");
            if has_imprinted_colors
                || has_any_choice_mana
                || uses_commander_identity
                || has_chosen_color
            {
                let mana_ast = parse_add_mana(mana_tokens, None)?;
                let mut compile_ctx = CompileContext::new();
                let (mut effects, choices) = compile_effect(&mana_ast, &mut compile_ctx)?;
                if !choices.is_empty() {
                    return Err(CardTextError::ParseError(
                        "unsupported target choice in mana ability".to_string(),
                    ));
                }
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Mana(ManaAbility {
                        mana_cost,
                        mana: Vec::new(),
                        effects: Some(effects),
                        activation_condition: None,
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                if let Some(condition) = mana_activation_condition.clone()
                    && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                {
                    mana_ability.activation_condition = Some(condition);
                }
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast: Some(effects_ast),
                }));
            }

            if has_or_choice_mana && !extra_effects.is_empty() {
                let mana_ast = parse_add_mana(mana_tokens, None)?;
                let mut compile_ctx = CompileContext::new();
                let (mut effects, choices) = compile_effect(&mana_ast, &mut compile_ctx)?;
                if !choices.is_empty() {
                    return Err(CardTextError::ParseError(
                        "unsupported target choice in mana ability".to_string(),
                    ));
                }
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Mana(ManaAbility {
                        mana_cost,
                        mana: Vec::new(),
                        effects: Some(effects),
                        activation_condition: None,
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                if let Some(condition) = mana_activation_condition.clone()
                    && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                {
                    mana_ability.activation_condition = Some(condition);
                }
                let mut effects_ast = vec![mana_ast];
                effects_ast.extend(extra_effects_ast);
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast: Some(effects_ast),
                }));
            }

            let mut mana = Vec::new();
            for token in mana_tokens {
                let Some(word) = token.as_word() else {
                    continue;
                };
                if word == "mana" || word == "to" || word == "your" || word == "pool" {
                    continue;
                }
                if let Ok(symbol) = parse_mana_symbol(word) {
                    mana.push(symbol);
                }
            }

            if !mana.is_empty() {
                if let Some(amount) = dynamic_amount {
                    let amount = resolve_mana_ability_scaled_amount_from_cost(amount, &mana_cost)?;
                    let mut effects =
                        vec![Effect::new(crate::effects::mana::AddScaledManaEffect::new(
                            mana,
                            amount,
                            PlayerFilter::You,
                        ))];
                    effects.extend(extra_effects);
                    let mut ability = Ability {
                        kind: AbilityKind::Mana(ManaAbility {
                            mana_cost,
                            mana: Vec::new(),
                            effects: Some(effects),
                            activation_condition: None,
                        }),
                        functional_zones: functional_zones.clone(),
                        text: None,
                    };
                    if let Some(condition) = mana_activation_condition.clone()
                        && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                    {
                        mana_ability.activation_condition = Some(condition);
                    }
                    let effects_ast = if extra_effects_ast.is_empty() {
                        None
                    } else {
                        Some(extra_effects_ast)
                    };
                    return Ok(Some(ParsedAbility {
                        ability,
                        effects_ast,
                    }));
                }
                if extra_effects.is_empty() {
                    let mut ability = Ability {
                        kind: AbilityKind::Mana(ManaAbility {
                            mana_cost,
                            mana,
                            effects: None,
                            activation_condition: None,
                        }),
                        functional_zones: functional_zones.clone(),
                        text: None,
                    };
                    if let Some(condition) = mana_activation_condition.clone()
                        && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                    {
                        mana_ability.activation_condition = Some(condition);
                    }
                    let effects_ast = if extra_effects_ast.is_empty() {
                        None
                    } else {
                        Some(extra_effects_ast)
                    };
                    return Ok(Some(ParsedAbility {
                        ability,
                        effects_ast,
                    }));
                }
                let mut effects = vec![Effect::add_mana(mana)];
                effects.extend(extra_effects);
                let mut ability = Ability {
                    kind: AbilityKind::Mana(ManaAbility {
                        mana_cost,
                        mana: Vec::new(),
                        effects: Some(effects),
                        activation_condition: None,
                    }),
                    functional_zones: functional_zones.clone(),
                    text: None,
                };
                if let Some(condition) = mana_activation_condition
                    && let AbilityKind::Mana(ref mut mana_ability) = ability.kind
                {
                    mana_ability.activation_condition = Some(condition);
                }
                let effects_ast = if extra_effects_ast.is_empty() {
                    None
                } else {
                    Some(extra_effects_ast)
                };
                return Ok(Some(ParsedAbility {
                    ability,
                    effects_ast,
                }));
            }
        }
    }

    // Generic activated ability: parse costs and effects from "<costs>: <effects>"
    let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
    let effect_tokens_joined = join_sentences_with_period(&effect_sentences);
    let effects_ast = parse_effect_sentences(&effect_tokens_joined)?;
    if effects_ast.is_empty() {
        return Ok(None);
    }
    let (effects, choices) = compile_trigger_effects(None, &effects_ast)?;
    let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects,
                choices,
                timing,
                additional_restrictions: additional_activation_restrictions,
            }),
            functional_zones,
            text: None,
        },
        effects_ast: Some(effects_ast),
    }))
}

fn first_sacrifice_cost_choice_tag(mana_cost: &crate::cost::TotalCost) -> Option<TagKey> {
    for cost in mana_cost.costs() {
        let Some(effect) = cost.effect_ref() else {
            continue;
        };
        let Some(choose) = effect.downcast_ref::<crate::effects::ChooseObjectsEffect>() else {
            continue;
        };
        if choose.tag.as_str().starts_with("sacrifice_cost_") {
            return Some(choose.tag.clone());
        }
    }
    None
}

fn resolve_mana_ability_scaled_amount_from_cost(
    amount: Value,
    mana_cost: &crate::cost::TotalCost,
) -> Result<Value, CardTextError> {
    if let Value::ManaValueOf(spec) = &amount
        && let ChooseSpec::Tagged(tag) = spec.as_ref()
        && tag.as_str() == IT_TAG
    {
        let Some(sac_tag) = first_sacrifice_cost_choice_tag(mana_cost) else {
            return Err(CardTextError::ParseError(
                "mana-value scaling requires a sacrificed object cost reference".to_string(),
            ));
        };
        return Ok(Value::ManaValueOf(Box::new(ChooseSpec::Tagged(sac_tag))));
    }

    Ok(amount)
}

fn infer_activated_functional_zones(
    cost_tokens: &[Token],
    effect_sentences: &[Vec<Token>],
) -> Vec<Zone> {
    let cost_words: Vec<&str> = words(cost_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if contains_source_from_your_graveyard_phrase(&cost_words)
        || effect_sentences.iter().any(|sentence| {
            let clause_words: Vec<&str> = words(sentence)
                .into_iter()
                .filter(|word| !is_article(word))
                .collect();
            contains_source_from_your_graveyard_phrase(&clause_words)
        })
    {
        vec![Zone::Graveyard]
    } else {
        vec![Zone::Battlefield]
    }
}

fn parse_activate_only_timing(tokens: &[Token]) -> Option<ActivationTiming> {
    let words = words(tokens);
    if words.starts_with(&["activate", "only", "as", "a", "sorcery"]) {
        return Some(ActivationTiming::SorcerySpeed);
    }
    if words.starts_with(&["activate", "only", "once", "each", "turn"])
        || contains_word_sequence(&words, &["once", "each", "turn"])
    {
        return Some(ActivationTiming::OncePerTurn);
    }
    if words.starts_with(&["activate", "only", "during", "combat"])
        || contains_word_sequence(&words, &["during", "combat"])
    {
        return Some(ActivationTiming::DuringCombat);
    }
    if words.starts_with(&["activate", "only", "during", "your", "turn"])
        || contains_word_sequence(&words, &["during", "your", "turn"])
    {
        return Some(ActivationTiming::DuringYourTurn);
    }
    if words.starts_with(&["activate", "only", "during", "an", "opponents", "turn"])
        || words.starts_with(&["activate", "only", "during", "opponents", "turn"])
        || contains_word_sequence(&words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(&words, &["during", "opponents", "turn"])
    {
        return Some(ActivationTiming::DuringOpponentsTurn);
    }
    None
}

fn contains_word_sequence(words: &[&str], sequence: &[&str]) -> bool {
    if sequence.is_empty() || words.len() < sequence.len() {
        return false;
    }
    words.windows(sequence.len()).any(|window| window == sequence)
}

fn combine_mana_activation_condition(
    base: Option<ManaAbilityCondition>,
    timing: ActivationTiming,
) -> Option<ManaAbilityCondition> {
    if timing == ActivationTiming::AnyTime {
        return base;
    }
    let timing_condition = ManaAbilityCondition::Timing(timing);
    match base {
        Some(ManaAbilityCondition::All(mut conditions)) => {
            conditions.push(timing_condition);
            Some(ManaAbilityCondition::All(conditions))
        }
        Some(existing) => Some(ManaAbilityCondition::All(vec![existing, timing_condition])),
        None => Some(timing_condition),
    }
}

fn is_activate_only_restriction_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.starts_with(&["activate", "only"])
}

fn is_trigger_only_restriction_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.starts_with(&["this", "ability", "triggers", "only"])
}

fn triggered_once_each_turn_sentence(sentences: &[Vec<Token>]) -> bool {
    sentences.iter().any(|sentence| {
        let words = words(sentence);
        words.starts_with(&["this", "ability", "triggers", "only", "once", "each", "turn"])
    })
}

fn parse_level_up_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 3 || words[0] != "level" || words[1] != "up" {
        return Ok(None);
    }

    let mut symbols = Vec::new();
    for word in words.iter().skip(2) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            symbols.push(symbol);
        }
    }

    if symbols.is_empty() {
        return Err(CardTextError::ParseError(
            "level up missing mana cost".to_string(),
        ));
    }

    let pips = symbols.into_iter().map(|symbol| vec![symbol]).collect();
    let mana_cost = ManaCost::from_pips(pips);
    let level_up_text = format!("Level up {}", mana_cost.to_oracle());

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: TotalCost::mana(mana_cost),
                effects: vec![Effect::put_counters_on_source(CounterType::Level, 1)],
                choices: vec![],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(level_up_text),
        },
        effects_ast: None,
    }))
}

fn parse_cycling_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words_all = words(tokens);
    if words_all.is_empty() {
        return Ok(None);
    }

    let Some(cycling_idx) = words_all.iter().position(|word| word.ends_with("cycling")) else {
        return Ok(None);
    };
    // Static grant clauses like "Each Sliver card in each player's hand has slivercycling {3}."
    // must be handled by parse_filter_has_granted_ability_line, not parsed as a standalone
    // cycling keyword ability on this card.
    if words_all
        .iter()
        .take(cycling_idx)
        .any(|word| *word == "has" || *word == "have")
    {
        return Ok(None);
    }

    let cost_start = cycling_idx + 1;
    if cost_start >= tokens.len() {
        return Ok(None);
    }

    let cost_end = tokens[cost_start..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| cost_start + idx)
        .unwrap_or(tokens.len());
    if cost_end <= cost_start {
        return Ok(None);
    }

    let (base_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..cost_end])?;
    let mut full_cost_effects = cost_effects;
    full_cost_effects.push(Effect::discard(1));
    let mana_cost = crate::ability::merge_cost_effects(base_cost.clone(), full_cost_effects);

    let cycling_tokens = &tokens[..cost_start];
    let search_filter = parse_cycling_search_filter(cycling_tokens)?;
    let effect = if let Some(filter) = search_filter {
        Effect::search_library_to_hand(filter, true)
    } else {
        Effect::draw(1)
    };

    let keyword_text = words(cycling_tokens).join(" ");
    let cost_text = base_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| words(&tokens[cost_start..cost_end]).join(" "));
    let render_text = if let Some(group) = parse_cycling_keyword_group_text(tokens) {
        group
    } else if keyword_text.is_empty() {
        cost_text
    } else {
        format!("{keyword_text} {cost_text}")
    };

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects: vec![effect],
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(render_text),
        },
        effects_ast: None,
    }))
}

fn parse_cycling_keyword_group_text(tokens: &[Token]) -> Option<String> {
    let mut idx = 0usize;
    let mut parts = Vec::new();

    loop {
        let keyword = tokens.get(idx).and_then(Token::as_word)?;
        if !keyword.ends_with("cycling") {
            break;
        }
        idx += 1;

        let mut cost_parts = Vec::new();
        while let Some(word) = tokens.get(idx).and_then(Token::as_word) {
            if is_cycling_cost_word(word) {
                cost_parts.push(word.to_string());
                idx += 1;
            } else {
                break;
            }
        }
        if cost_parts.is_empty() {
            break;
        }
        parts.push(format!("{keyword} {}", cost_parts.join("")));

        if tokens.get(idx).is_some_and(|token| matches!(token, Token::Comma(_))) {
            idx += 1;
            continue;
        }
        break;
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(", "))
    }
}

fn is_cycling_cost_word(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            ch.is_ascii_digit()
                || matches!(ch, '{' | '}' | '/' | 'w' | 'u' | 'b' | 'r' | 'g' | 'c' | 'x')
        })
}

fn parse_madness_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("madness")) {
        return Ok(None);
    }

    let cost_start = 1;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let cost_end = tokens[cost_start..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| cost_start + idx)
        .unwrap_or(tokens.len());
    if cost_end <= cost_start {
        return Err(CardTextError::ParseError(
            "madness keyword missing mana cost".to_string(),
        ));
    }

    let (total_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..cost_end])?;
    if !cost_effects.is_empty() {
        return Err(CardTextError::ParseError(
            "madness keyword only supports mana cost".to_string(),
        ));
    }
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("madness keyword missing mana symbols".to_string())
    })?;

    Ok(Some(AlternativeCastingMethod::Madness { cost: mana_cost }))
}

fn parse_morph_keyword_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let Some(first_word) = tokens.first().and_then(Token::as_word) else {
        return Ok(None);
    };

    let is_megamorph = match first_word {
        "morph" => false,
        "megamorph" => true,
        _ => return Ok(None),
    };

    let mut symbols = Vec::new();
    let mut consumed = 1usize;
    for token in &tokens[1..] {
        let Some(word) = token.as_word() else {
            break;
        };
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        symbols.push(symbol);
        consumed += 1;
    }

    if symbols.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "{mechanic} keyword missing mana cost"
        )));
    }

    let trailing_words = words(&tokens[consumed..]);
    if !trailing_words.is_empty() {
        let mechanic = if is_megamorph { "megamorph" } else { "morph" };
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing {mechanic} clause (line: '{}')",
            trailing_words.join(" ")
        )));
    }

    let cost = ManaCost::from_symbols(symbols);
    let label = if is_megamorph { "Megamorph" } else { "Morph" };
    let text = format!("{label} {}", cost.to_oracle());
    let static_ability = if is_megamorph {
        StaticAbility::megamorph(cost)
    } else {
        StaticAbility::morph(cost)
    };

    Ok(Some(ParsedAbility {
        ability: Ability::static_ability(static_ability).with_text(&text),
        effects_ast: None,
    }))
}

fn parse_escape_line(
    tokens: &[Token],
) -> Result<Option<AlternativeCastingMethod>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("escape")) {
        return Ok(None);
    }

    let cost_start = 1;
    if cost_start >= tokens.len() {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let comma_idx = tokens[cost_start..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| cost_start + idx)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "escape keyword missing exile clause separator".to_string(),
            )
        })?;
    if comma_idx <= cost_start {
        return Err(CardTextError::ParseError(
            "escape keyword missing mana cost".to_string(),
        ));
    }

    let (total_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..comma_idx])?;
    if !cost_effects.is_empty() {
        return Err(CardTextError::ParseError(
            "escape keyword only supports mana cost".to_string(),
        ));
    }
    let mana_cost = total_cost.mana_cost().cloned().ok_or_else(|| {
        CardTextError::ParseError("escape keyword missing mana symbols".to_string())
    })?;

    let tail_tokens = trim_commas(&tokens[comma_idx + 1..]);
    if tail_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "escape keyword missing exile clause".to_string(),
        ));
    }

    let tail_words = words(&tail_tokens);
    if tail_words.first().copied() != Some("exile") {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    let (exile_count, used) = parse_number(&tail_tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "escape keyword missing exile count (clause: '{}')",
            tail_words.join(" ")
        ))
    })?;
    let mut idx = 1 + used;
    if tail_words.get(idx).copied() == Some("other") {
        idx += 1;
    }
    if !matches!(tail_words.get(idx).copied(), Some("card") | Some("cards")) {
        return Err(CardTextError::ParseError(format!(
            "escape keyword missing exiled card noun (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    idx += 1;
    if tail_words.get(idx..idx + 3) != Some(&["from", "your", "graveyard"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported escape clause tail (clause: '{}')",
            tail_words.join(" ")
        )));
    }
    if idx + 3 != tail_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing escape clause segment (clause: '{}')",
            tail_words.join(" ")
        )));
    }

    Ok(Some(AlternativeCastingMethod::Escape {
        cost: Some(mana_cost),
        exile_count,
    }))
}

fn parse_cycling_search_filter(tokens: &[Token]) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() {
        return Ok(None);
    }

    let keyword = words
        .last()
        .copied()
        .ok_or_else(|| CardTextError::ParseError("missing cycling keyword".to_string()))?;
    let mut filter = ObjectFilter::default();

    for word in &words[..words.len().saturating_sub(1)] {
        if let Some(supertype) = parse_supertype_word(word)
            && !filter.supertypes.contains(&supertype)
        {
            filter.supertypes.push(supertype);
        }
        if let Some(card_type) = parse_card_type(word)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        }
        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            && !filter.subtypes.contains(&subtype)
        {
            filter.subtypes.push(subtype);
            if is_land_subtype(subtype) && !filter.card_types.contains(&CardType::Land) {
                filter.card_types.push(CardType::Land);
            }
        }
        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
    }

    if keyword == "cycling" {
        return Ok(None);
    }

    if keyword == "landcycling" {
        if !filter.card_types.contains(&CardType::Land) {
            filter.card_types.push(CardType::Land);
        }
        return Ok(Some(filter));
    }

    if let Some(root) = keyword.strip_suffix("cycling") {
        if let Some(card_type) = parse_card_type(root)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        } else if let Some(subtype) =
            parse_subtype_word(root).or_else(|| root.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !filter.subtypes.contains(&subtype) {
                filter.subtypes.push(subtype);
            }
            if is_land_subtype(subtype) && !filter.card_types.contains(&CardType::Land) {
                filter.card_types.push(CardType::Land);
            }
        } else if let Some(color) = parse_color(root) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        } else {
            return Err(CardTextError::ParseError(format!(
                "unsupported cycling variant (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(Some(filter));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported cycling variant (clause: '{}')",
        words.join(" ")
    )))
}

fn is_land_subtype(subtype: Subtype) -> bool {
    matches!(
        subtype,
        Subtype::Plains | Subtype::Island | Subtype::Swamp | Subtype::Mountain | Subtype::Forest
    )
}

fn parse_equip_line(tokens: &[Token]) -> Result<Option<ParsedAbility>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("equip") {
        return Ok(None);
    }

    let mut symbols = Vec::new();
    let mut saw_zero = false;
    let mut saw_non_symbol = false;
    for word in words.iter().skip(1) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            if matches!(symbol, ManaSymbol::Generic(0)) {
                saw_zero = true;
            } else {
                symbols.push(symbol);
            }
        } else {
            saw_non_symbol = true;
        }
    }

    if saw_non_symbol {
        return Ok(None);
    }

    if symbols.is_empty() && !saw_zero {
        return Err(CardTextError::ParseError(
            "equip missing mana cost".to_string(),
        ));
    }

    let mana_cost = if symbols.is_empty() {
        ManaCost::new()
    } else {
        let pips = symbols.into_iter().map(|symbol| vec![symbol]).collect();
        ManaCost::from_pips(pips)
    };
    let total_cost = if mana_cost.pips().is_empty() {
        TotalCost::free()
    } else {
        TotalCost::mana(mana_cost)
    };
    let equip_text = if saw_zero && total_cost.costs().is_empty() {
        "Equip {0}".to_string()
    } else if let Some(mana) = total_cost.mana_cost() {
        format!("Equip {}", mana.to_oracle())
    } else {
        "Equip".to_string()
    };
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: total_cost,
                effects: vec![Effect::attach_to(target.clone())],
                choices: vec![target.clone()],
                timing: ActivationTiming::SorcerySpeed,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(equip_text),
        },
        effects_ast: None,
    }))
}

fn parse_activation_cost(tokens: &[Token]) -> Result<(TotalCost, Vec<Effect>), CardTextError> {
    let mut mana_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let cost_effects = Vec::new();
    let mut explicit_costs = Vec::new();
    let mut energy_count: u32 = 0;
    let mut sac_tag_id = 0u32;
    let mut tap_tag_id = 0u32;
    let mut exile_tag_id = 0u32;
    let mut return_tag_id = 0u32;

    for raw_segment in split_cost_segments(tokens) {
        if raw_segment.is_empty() {
            continue;
        }
        let mut segment = raw_segment;
        while segment
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            segment.remove(0);
        }
        if segment.is_empty() {
            continue;
        }

        let segment_words = words(&segment);
        if segment_words.is_empty() {
            continue;
        }

        if segment_words[0] == "tap" || segment_words[0] == "t" {
            if segment_words.len() == 1 {
                explicit_costs.push(crate::costs::Cost::effect(Effect::tap_source()));
                continue;
            }

            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            } else if segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }

            if !segment
                .get(idx)
                .is_some_and(|token| token.is_word("untapped"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported tap cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            idx += 1;

            let filter_tokens = &segment[idx..];
            if filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing tap-cost filter (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            let mut filter = parse_object_filter(filter_tokens, false)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }
            filter.untapped = true;

            let tag = format!("tap_cost_{tap_tag_id}");
            tap_tag_id += 1;
            explicit_costs.push(crate::costs::Cost::effect(Effect::choose_objects(
                filter,
                count as usize,
                PlayerFilter::You,
                tag.clone(),
            )));
            explicit_costs.push(crate::costs::Cost::effect(Effect::tap(ChooseSpec::tagged(
                tag,
            ))));
            continue;
        }

        if segment_words[0] == "pay" {
            if segment_words.contains(&"life") {
                let amount = parse_number(&segment[1..]).ok_or_else(|| {
                    CardTextError::ParseError("unable to parse pay life cost".to_string())
                })?;
                explicit_costs.push(crate::costs::Cost::life(amount.0));
                continue;
            }
            let mut parsed_any = false;
            for token in &segment[1..] {
                let Some(word) = token.as_word() else {
                    continue;
                };
                if let Ok(symbol) = parse_mana_symbol(word) {
                    mana_pips.push(vec![symbol]);
                    parsed_any = true;
                }
            }
            if !parsed_any {
                return Err(CardTextError::ParseError(
                    "unsupported pay cost (expected life or mana symbols)".to_string(),
                ));
            }
            continue;
        }

        if segment_words[0] == "discard" {
            let mut idx = 1usize;
            let mut count = 1u32;

            let after_discard_words = words(&segment[idx..]);
            if after_discard_words.starts_with(&["your", "hand"]) {
                if after_discard_words.len() != 2 {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing discard-hand cost clause (clause: '{}')",
                        segment_words.join(" ")
                    )));
                }
                explicit_costs.push(crate::costs::Cost::effect(Effect::discard_hand()));
                continue;
            }

            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            }

            let mut card_type: Option<CardType> = None;
            if let Some(word) = segment.get(idx).and_then(Token::as_word)
                && let Some(parsed_type) = parse_card_type(word)
            {
                card_type = Some(parsed_type);
                idx += 1;
            }

            if !segment.get(idx).is_some_and(|token| {
                token.is_word("card") || token.is_word("cards")
            }) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported discard cost selector (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            idx += 1;

            let trailing_words = words(&segment[idx..]);
            if !trailing_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing discard cost clause (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            if let Some(card_type) = card_type {
                explicit_costs.push(crate::costs::Cost::discard(count, Some(card_type)));
            } else {
                explicit_costs.push(crate::costs::Cost::effect(Effect::discard(count)));
            }
            continue;
        }

        if segment_words[0] == "mill" {
            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            } else if segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }

            if !segment.get(idx).is_some_and(|token| {
                token.is_word("card") || token.is_word("cards")
            }) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported mill cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            explicit_costs.push(crate::costs::Cost::effect(Effect::mill(count)));
            continue;
        }

        if segment_words[0] == "sacrifice" {
            if segment_words.get(1).copied() == Some("this") {
                explicit_costs.push(crate::costs::Cost::effect(Effect::sacrifice_source()));
                continue;
            }
            let mut idx = 1;
            let mut count = 1u32;
            let mut other = false;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            }
            if segment
                .get(idx)
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                idx += 1;
            }
            if count == 1
                && let Some((value, used)) = parse_number(&segment[idx..])
            {
                count = value;
                idx += used;
            }
            let filter_tokens = &segment[idx..];
            let mut filter = parse_object_filter(filter_tokens, other)?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            let tag = format!("sacrifice_cost_{sac_tag_id}");
            sac_tag_id += 1;
            explicit_costs.push(crate::costs::Cost::effect(Effect::choose_objects(
                filter,
                count as usize,
                PlayerFilter::You,
                tag.clone(),
            )));
            explicit_costs.push(crate::costs::Cost::effect(Effect::sacrifice(
                ObjectFilter::tagged(tag),
                count,
            )));
            continue;
        }

        if segment_words[0] == "exile" {
            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            }
            while segment
                .get(idx)
                .is_some_and(|token| token.is_word("a") || token.is_word("an"))
            {
                idx += 1;
            }
            let mut color_filter = None;
            if let Some(word) = segment.get(idx).and_then(Token::as_word)
                && let Some(color) = parse_color(word)
            {
                color_filter = Some(color);
                idx += 1;
            }

            let tail_words = words(&segment[idx..]);
            let has_card = tail_words.contains(&"card") || tail_words.contains(&"cards");
            let has_hand = tail_words.contains(&"hand");
            if has_card && has_hand {
                explicit_costs.push(crate::costs::Cost::effect(Effect::exile_from_hand_as_cost(
                    count,
                    color_filter,
                )));
                continue;
            }

            let mut filter_tokens = &segment[1..];
            let mut generic_count = 1usize;
            let mut top_only = false;
            if let Some((value, used)) = parse_number(filter_tokens) {
                generic_count = value as usize;
                filter_tokens = &filter_tokens[used..];
            }
            while filter_tokens.first().is_some_and(|token| {
                token.is_word("the") || token.is_word("a") || token.is_word("an")
            }) {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("top"))
            {
                // "Exile the top ... card of your graveyard" should select only the
                // top-most matching object in that ordered zone.
                top_only = true;
                filter_tokens = &filter_tokens[1..];
            }
            while filter_tokens.first().is_some_and(|token| {
                token.is_word("the") || token.is_word("a") || token.is_word("an")
            }) {
                filter_tokens = &filter_tokens[1..];
            }
            if let Some((value, used)) = parse_number(filter_tokens) {
                generic_count = value as usize;
                filter_tokens = &filter_tokens[used..];
            }
            while filter_tokens.first().is_some_and(|token| {
                token.is_word("the") || token.is_word("a") || token.is_word("an")
            }) {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
            {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("of"))
            {
                filter_tokens = &filter_tokens[1..];
            }
            if filter_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported exile cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let filter_words = words(filter_tokens);
            if filter_words.first().copied() == Some("target") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported targeted exile cost segment (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            if is_source_reference_words(&filter_words)
                || is_source_from_your_graveyard_words(&filter_words)
            {
                explicit_costs.push(crate::costs::Cost::effect(Effect::exile(
                    ChooseSpec::Source,
                )));
                continue;
            }

            let mut filter = parse_object_filter(filter_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported exile cost segment (clause: '{}')",
                    segment_words.join(" ")
                ))
            })?;
            if filter == ObjectFilter::default() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported exile cost filter (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            if filter.zone == Some(Zone::Battlefield) && filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            if filter.zone != Some(Zone::Battlefield)
                && filter.owner.is_none()
                && filter.controller.is_none()
            {
                filter.owner = Some(PlayerFilter::You);
            }

            let tag = format!("exile_cost_{exile_tag_id}");
            exile_tag_id += 1;
            let choice_zone = filter.zone.unwrap_or(Zone::Battlefield);
            let mut choose_effect = crate::effects::ChooseObjectsEffect::new(
                filter,
                generic_count,
                PlayerFilter::You,
                tag.clone(),
            )
            .in_zone(choice_zone);
            if top_only {
                choose_effect = choose_effect.top_only();
            }
            explicit_costs.push(crate::costs::Cost::effect(Effect::new(choose_effect)));
            explicit_costs.push(crate::costs::Cost::effect(Effect::exile(
                ChooseSpec::tagged(tag),
            )));
            continue;
        }

        if segment_words[0] == "put" {
            let (count, used) = parse_number(&segment[1..]).ok_or_else(|| {
                CardTextError::ParseError("unable to parse put counter cost amount".to_string())
            })?;
            let counter_type =
                parse_counter_type_from_tokens(&segment[1 + used..]).ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported counter type in activation cost (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;
            explicit_costs.push(crate::costs::Cost::add_counters(counter_type, count));
            continue;
        }

        if segment_words[0] == "remove" {
            let mut any_number = false;
            let mut variable_x = false;
            let (count, used) = if segment
                .get(1)
                .is_some_and(|token| token.is_word("any"))
                && segment
                    .get(2)
                    .is_some_and(|token| token.is_word("number"))
            {
                any_number = true;
                let consumed = if segment.get(3).is_some_and(|token| token.is_word("of")) {
                    3
                } else {
                    2
                };
                (u32::MAX / 4, consumed)
            } else if segment.get(1).is_some_and(|token| token.is_word("x")) {
                any_number = true;
                variable_x = true;
                (u32::MAX / 4, 1)
            } else {
                parse_number(&segment[1..]).ok_or_else(|| {
                    CardTextError::ParseError("unable to parse remove counter cost amount".to_string())
                })?
            };
            let mut idx = 1 + used;
            let from_idx = segment[idx..]
                .iter()
                .position(|token| token.is_word("from"))
                .map(|offset| idx + offset)
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "missing 'from' in remove-counter cost (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;
            let descriptor = &segment[idx..from_idx];
            let has_counter_keyword = descriptor
                .iter()
                .any(|token| token.is_word("counter") || token.is_word("counters"));
            if !has_counter_keyword {
                return Err(CardTextError::ParseError(format!(
                    "missing counter keyword in activation cost (clause: '{}')",
                    segment_words.join(" ")
                )));
            }
            let counter_type = parse_counter_type_from_tokens(descriptor);

            idx = from_idx + 1;
            let from_among = segment.get(idx).is_some_and(|token| token.is_word("among"));
            if from_among {
                idx += 1;
                if idx >= segment.len() {
                    return Err(CardTextError::ParseError(format!(
                        "missing filter for remove-counter cost (clause: '{}')",
                        segment_words.join(" ")
                    )));
                }

                if counter_type.is_some() {
                    return Err(CardTextError::ParseError(format!(
                        "typed distributed counter costs are unsupported (clause: '{}')",
                        segment_words.join(" ")
                    )));
                }

                let mut filter = parse_object_filter(&segment[idx..], false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                let max_count = if any_number { u32::MAX / 4 } else { count };
                explicit_costs.push(crate::costs::Cost::new(
                    crate::costs::RemoveAnyCountersAmongCost::new(max_count, filter),
                ));
                continue;
            }

            let counter_type = counter_type.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported counter type in activation cost (clause: '{}')",
                    segment_words.join(" ")
                ))
            })?;
            if idx >= segment.len() {
                return Err(CardTextError::ParseError(format!(
                    "missing filter for remove-counter cost (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let remaining = &segment[idx..];
            let remaining_words = words(remaining);
            let from_source = is_source_reference_words(&remaining_words);
            if from_source {
                if any_number {
                    let source_cost = if variable_x {
                        crate::costs::RemoveAnyCountersFromSourceCost::x(Some(counter_type))
                    } else {
                        crate::costs::RemoveAnyCountersFromSourceCost::any_number(Some(counter_type))
                    };
                    explicit_costs.push(crate::costs::Cost::new(source_cost));
                } else {
                    explicit_costs.push(crate::costs::Cost::remove_counters(counter_type, count));
                }
            } else {
                let mut filter = parse_object_filter(remaining, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                let max_count = if any_number { u32::MAX / 4 } else { count };
                explicit_costs.push(crate::costs::Cost::new(
                    crate::costs::RemoveAnyCountersAmongCost::new(max_count, filter),
                ));
            }
            continue;
        }

        if segment_words[0] == "return" {
            let mut idx = 1usize;
            let mut count = 1u32;
            if let Some((value, used)) = parse_number(&segment[idx..]) {
                count = value;
                idx += used;
            }

            let to_idx = segment
                .iter()
                .position(|token| token.is_word("to"))
                .ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported return cost segment (clause: '{}')",
                        segment_words.join(" ")
                    ))
                })?;
            let target_tokens = trim_commas(&segment[idx..to_idx]);
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "missing return-cost target (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let destination_words = words(&segment[to_idx + 1..]);
            if !destination_words.contains(&"hand") {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return-cost destination (clause: '{}')",
                    segment_words.join(" ")
                )));
            }

            let mut filter = parse_object_filter(&target_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported return-cost target filter (clause: '{}')",
                    segment_words.join(" ")
                ))
            })?;
            if filter.controller.is_none() {
                filter.controller = Some(PlayerFilter::You);
            }
            if filter.zone.is_none() {
                filter.zone = Some(Zone::Battlefield);
            }

            let tag = format!("return_cost_{return_tag_id}");
            return_tag_id += 1;
            explicit_costs.push(crate::costs::Cost::effect(Effect::choose_objects(
                filter,
                count as usize,
                PlayerFilter::You,
                tag.clone(),
            )));
            explicit_costs.push(crate::costs::Cost::effect(Effect::return_to_hand(
                ObjectFilter::tagged(tag),
            )));
            continue;
        }

        // Otherwise, treat as pure mana symbols.
        for word in &segment_words {
            if *word == "e" {
                energy_count = energy_count.saturating_add(1);
                continue;
            }
            if word.contains('/') {
                let alternatives = parse_mana_symbol_group(word)?;
                mana_pips.push(alternatives);
                continue;
            }
            if let Ok(symbol) = parse_mana_symbol(word) {
                mana_pips.push(vec![symbol]);
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported activation cost segment (clause: '{}')",
                segment_words.join(" ")
            )));
        }
    }

    let mut costs = Vec::new();
    if !mana_pips.is_empty() {
        costs.push(crate::costs::Cost::mana(ManaCost::from_pips(mana_pips)));
    }
    if energy_count > 0 {
        costs.push(crate::costs::Cost::energy(energy_count));
    }
    costs.extend(explicit_costs);

    let total_cost = if costs.is_empty() {
        TotalCost::free()
    } else {
        TotalCost::from_costs(costs)
    };

    Ok((total_cost, cost_effects))
}

fn parse_devotion_value_from_add_clause(tokens: &[Token]) -> Result<Option<Value>, CardTextError> {
    let words = words(tokens);
    let Some(devotion_idx) = words.iter().position(|word| *word == "devotion") else {
        return Ok(None);
    };

    let player = parse_devotion_player_from_words(&words, devotion_idx).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion player in clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let to_idx = words[devotion_idx + 1..]
        .iter()
        .position(|word| *word == "to")
        .map(|idx| devotion_idx + 1 + idx)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing color after devotion clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
    let color_word = words.get(to_idx + 1).copied().ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing devotion color (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let color_set = parse_color(color_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;
    let color = color_from_color_set(color_set).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "ambiguous devotion color '{}' (clause: '{}')",
            color_word,
            words.join(" ")
        ))
    })?;

    Ok(Some(Value::Devotion { player, color }))
}

fn parse_devotion_player_from_words(words: &[&str], devotion_idx: usize) -> Option<PlayerFilter> {
    if devotion_idx == 0 {
        return None;
    }
    let left = &words[..devotion_idx];
    if left.ends_with(&["your"]) {
        return Some(PlayerFilter::You);
    }
    if left.ends_with(&["opponent"]) || left.ends_with(&["opponents"]) {
        return Some(PlayerFilter::Opponent);
    }
    if left.ends_with(&["that", "players"]) || left.ends_with(&["that", "player"]) {
        return Some(PlayerFilter::Target(Box::new(PlayerFilter::Any)));
    }
    None
}

fn color_from_color_set(colors: ColorSet) -> Option<crate::color::Color> {
    let mut found = None;
    for color in [
        crate::color::Color::White,
        crate::color::Color::Blue,
        crate::color::Color::Black,
        crate::color::Color::Red,
        crate::color::Color::Green,
    ] {
        if colors.contains(color) {
            if found.is_some() {
                return None;
            }
            found = Some(color);
        }
    }
    found
}

fn parse_activation_condition(tokens: &[Token]) -> Option<ManaAbilityCondition> {
    let line_words = words(tokens);
    if line_words.len() < 5 {
        return None;
    }
    if line_words.starts_with(&["activate", "only", "as", "an", "instant"])
        || line_words.starts_with(&["activate", "only", "as", "instant"])
    {
        return Some(ManaAbilityCondition::Timing(ActivationTiming::AnyTime));
    }
    if line_words.starts_with(&["activate", "only", "if", "there", "is"])
        || line_words.starts_with(&["activate", "only", "if", "there", "are"])
    {
        let descriptor_start = 5usize;
        let in_idx = line_words
            .iter()
            .enumerate()
            .skip(descriptor_start)
            .find_map(|(idx, word)| (*word == "in").then_some(idx))?;
        let zone_tail = &line_words[in_idx..];
        let points_to_your_graveyard = zone_tail == ["in", "your", "graveyard"]
            || zone_tail == ["in", "graveyard"]
            || zone_tail == ["in", "the", "graveyard"];
        if !points_to_your_graveyard {
            return None;
        }

        let descriptor_words = &line_words[descriptor_start..in_idx];
        if descriptor_words.is_empty() {
            return None;
        }

        let mut card_types = Vec::new();
        let mut subtypes = Vec::new();
        for word in descriptor_words {
            if let Some(card_type) = parse_card_type(word)
                && !card_types.contains(&card_type)
            {
                card_types.push(card_type);
            }
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }

        if card_types.is_empty() && subtypes.is_empty() {
            return None;
        }

        return Some(ManaAbilityCondition::CardInYourGraveyard {
            card_types,
            subtypes,
        });
    }
    if line_words.starts_with(&[
        "activate",
        "only",
        "if",
        "creatures",
        "you",
        "control",
        "have",
        "total",
        "power",
    ]) {
        let threshold_word = line_words.get(9)?;
        let threshold = parse_cardinal_u32(threshold_word)?;
        let tail = &line_words[10..];
        if tail == ["or", "greater"] {
            return Some(ManaAbilityCondition::ControlCreaturesTotalPowerAtLeast(
                threshold,
            ));
        }
        return None;
    }
    if !line_words.starts_with(&["activate", "only", "if", "you", "control"]) {
        return None;
    }

    let after_control = &tokens[5..];
    let control_tail = words(after_control);
    if control_tail.starts_with(&["a", "creature", "with", "power"])
        || control_tail.starts_with(&["creature", "with", "power"])
    {
        let power_idx = control_tail.iter().position(|word| *word == "power")?;
        let threshold = parse_cardinal_u32(control_tail.get(power_idx + 1)?)?;
        let tail = &control_tail[power_idx + 2..];
        if tail == ["or", "greater"] {
            return Some(ManaAbilityCondition::ControlCreatureWithPowerAtLeast(
                threshold,
            ));
        }
        return None;
    }
    if let Some((count, used)) = parse_number(after_control) {
        let tail = words(&after_control[used..]);
        if tail == ["or", "more", "artifact"] || tail == ["or", "more", "artifacts"] {
            return Some(ManaAbilityCondition::ControlAtLeastArtifacts(count));
        }
        if tail == ["or", "more", "land"] || tail == ["or", "more", "lands"] {
            return Some(ManaAbilityCondition::ControlAtLeastLands(count));
        }
    }
    if control_tail == ["an", "artifact"]
        || control_tail == ["a", "artifact"]
        || control_tail == ["artifact"]
        || control_tail == ["artifacts"]
    {
        return Some(ManaAbilityCondition::ControlAtLeastArtifacts(1));
    }

    let mut subtypes = Vec::new();
    for word in line_words {
        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            && !subtypes.contains(&subtype)
        {
            subtypes.push(subtype);
        }
    }

    if subtypes.is_empty() {
        return None;
    }

    Some(ManaAbilityCondition::ControlLandWithSubtype(subtypes))
}

fn parse_cardinal_u32(word: &str) -> Option<u32> {
    let token = Token::Word(word.to_string(), TextSpan::synthetic());
    parse_number(&[token]).map(|(value, _)| value)
}

fn parse_enters_tapped_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        let has_enters_tapped =
            clause_words.contains(&"enters") && clause_words.contains(&"tapped");
        if has_enters_tapped {
            return Err(CardTextError::ParseError(format!(
                "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(None);
    }
    if clause_words.first().copied() == Some("this")
        && clause_words.contains(&"enters")
        && clause_words.contains(&"tapped")
    {
        let tapped_word_idx = clause_words
            .iter()
            .position(|word| *word == "tapped")
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let tapped_token_idx =
            token_index_for_word_index(tokens, tapped_word_idx).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unable to map tapped keyword in enters-tapped clause (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?;
        let trailing_words = words(&tokens[tapped_token_idx + 1..]);
        if !trailing_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing enters-tapped clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(StaticAbility::enters_tapped_ability()));
    }
    Ok(None)
}

fn parse_cost_reduction_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let line_words = words(tokens);
    if line_words.starts_with(&["this", "cost", "is", "reduced", "by"])
        && line_words.len() > 6
    {
        let amount_word = line_words[5];
        let amount_text = if amount_word.chars().all(|ch| ch.is_ascii_digit()) {
            format!("{{{amount_word}}}")
        } else {
            amount_word.to_string()
        };
        let tail = line_words[6..].join(" ");
        let text = format!("This cost is reduced by {amount_text} {tail}");
        return Ok(Some(StaticAbility::custom("cost_reduction_text", text)));
    }
    if !line_words.starts_with(&["this", "spell", "costs"]) {
        return Ok(None);
    }

    let costs_idx = tokens
        .iter()
        .position(|token| token.is_word("costs"))
        .ok_or_else(|| CardTextError::ParseError("missing costs keyword".to_string()))?;
    let amount_tokens = &tokens[costs_idx + 1..];
    let parsed_amount = parse_cost_modifier_amount(amount_tokens);
    let (amount_value, used) = parsed_amount.clone().unwrap_or((Value::Fixed(1), 0));
    let amount_fixed = if let Value::Fixed(value) = amount_value {
        value
    } else {
        1
    };

    let remaining_tokens = &tokens[costs_idx + 1 + used..];
    let remaining_words: Vec<&str> = words(remaining_tokens);

    if !remaining_words.contains(&"less") {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(remaining_tokens)? {
        let reduction = crate::static_abilities::CostReduction::new(
            crate::ability::SpellFilter::default(),
            dynamic,
        );
        return Ok(Some(StaticAbility::new(reduction)));
    }

    if parsed_amount.is_none() {
        return Ok(None);
    }

    let has_each = remaining_words.contains(&"each");
    let has_card_type = remaining_words
        .windows(2)
        .any(|pair| pair == ["card", "type"]);
    let has_graveyard = remaining_words.contains(&"graveyard");

    if has_each && has_card_type && has_graveyard {
        if amount_fixed != 1 {
            return Ok(None);
        }
        let reduction = crate::effect::Value::CardTypesInGraveyard(PlayerFilter::You);
        let cost_reduction = crate::static_abilities::CostReduction::new(
            crate::ability::SpellFilter::default(),
            reduction,
        );
        return Ok(Some(StaticAbility::new(cost_reduction)));
    }

    Ok(None)
}

fn parse_all_creatures_able_to_block_source_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = normalize_cant_words(tokens);
    if words.as_slice()
        == [
            "all",
            "creatures",
            "able",
            "to",
            "block",
            "this",
            "creature",
            "do",
            "so",
        ]
        || words.as_slice() == ["all", "creatures", "able", "to", "block", "this", "do", "so"]
    {
        return Ok(Some(StaticAbility::grant_ability(
            ObjectFilter::creature(),
            StaticAbility::must_block(),
        )));
    }
    Ok(None)
}

fn parse_cant_clauses(tokens: &[Token]) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut abilities = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            let mut expanded = segment.clone();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.clone());
                expanded = with_subject;
            }
            let Some(ability) = parse_cant_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    words(segment).join(" ")
                )));
            };
            abilities.push(ability);
        }

        return Ok(Some(abilities));
    }

    parse_cant_clause(tokens).map(|ability| ability.map(|ability| vec![ability]))
}

fn parse_cant_clause(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let starts_with_cant_be_blocked_by = normalized
        .starts_with(&["this", "creature", "cant", "be", "blocked", "by"])
        || normalized.starts_with(&["this", "cant", "be", "blocked", "by"]);
    if starts_with_cant_be_blocked_by {
        let mut idx =
            if normalized.starts_with(&["this", "creature", "cant", "be", "blocked", "by"]) {
                6
            } else {
                5
            };
        if normalized
            .get(idx)
            .is_some_and(|word| *word == "creature" || *word == "creatures")
        {
            idx += 1;
        }
        if normalized.get(idx) == Some(&"more") && normalized.get(idx + 1) == Some(&"than") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![Token::Word(amount_word.to_string(), TextSpan::synthetic())];
            let (max_blockers, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1 {
                return Err(CardTextError::ParseError(format!(
                    "invalid blocker threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            let noun = normalized.get(idx + 3).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if noun != "creature" && noun != "creatures" {
                return Err(CardTextError::ParseError(format!(
                    "unsupported blocker noun in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            if idx + 4 != normalized.len() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked max-blockers clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            return Ok(Some(StaticAbility::cant_be_blocked_by_more_than(
                max_blockers as usize,
            )));
        }
        if normalized.get(idx) == Some(&"with") && normalized.get(idx + 1) == Some(&"power") {
            let amount_word = normalized.get(idx + 2).copied().ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            let amount_tokens = vec![Token::Word(amount_word.to_string(), TextSpan::synthetic())];
            let (threshold, used) = parse_number(&amount_tokens).ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "invalid power threshold in cant-blocked clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;
            if used != 1
                || normalized.get(idx + 3) != Some(&"or")
                || normalized.get(idx + 4) != Some(&"less")
                || idx + 5 != normalized.len()
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant-be-blocked power clause tail (clause: '{}')",
                    normalized.join(" ")
                )));
            }
            return Ok(Some(StaticAbility::cant_be_blocked_by_power_or_less(
                threshold as i32,
            )));
        }
    }

    let starts_with_cant_attack_unless_defending_player =
        normalized.starts_with(&[
            "this",
            "creature",
            "cant",
            "attack",
            "unless",
            "defending",
            "player",
        ]) || normalized.starts_with(&["this", "cant", "attack", "unless", "defending", "player"]);
    if starts_with_cant_attack_unless_defending_player {
        let mut idx = if normalized.starts_with(&[
            "this",
            "creature",
            "cant",
            "attack",
            "unless",
            "defending",
            "player",
        ]) {
            7
        } else {
            6
        };

        if !normalized
            .get(idx)
            .is_some_and(|word| *word == "control" || *word == "controls")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported cant-attack unless clause tail (clause: '{}')",
                normalized.join(" ")
            )));
        }
        idx += 1;

        if normalized
            .get(idx)
            .is_some_and(|word| *word == "a" || *word == "an" || *word == "the")
        {
            idx += 1;
        }

        let subtype_word = normalized.get(idx).copied().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing land subtype in cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            ))
        })?;
        let subtype = parse_subtype_word(subtype_word)
            .or_else(|| subtype_word.strip_suffix('s').and_then(parse_subtype_word))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported land subtype in cant-attack unless clause (clause: '{}')",
                    normalized.join(" ")
                ))
            })?;

        if idx + 1 != normalized.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing cant-attack unless clause (clause: '{}')",
                normalized.join(" ")
            )));
        }

        return Ok(Some(
            StaticAbility::cant_attack_unless_defending_player_controls_land_subtype(subtype),
        ));
    }

    let ability = match normalized.as_slice() {
        ["players", "cant", "gain", "life"] => StaticAbility::players_cant_gain_life(),
        ["players", "cant", "search", "libraries"] => StaticAbility::players_cant_search(),
        ["damage", "cant", "be", "prevented"] => StaticAbility::damage_cant_be_prevented(),
        ["you", "cant", "lose", "the", "game"] => StaticAbility::you_cant_lose_game(),
        ["your", "opponents", "cant", "win", "the", "game"] => {
            StaticAbility::opponents_cant_win_game()
        }
        ["your", "life", "total", "cant", "change"] => StaticAbility::your_life_total_cant_change(),
        ["your", "opponents", "cant", "cast", "spells"] => {
            StaticAbility::opponents_cant_cast_spells()
        }
        [
            "your",
            "opponents",
            "cant",
            "draw",
            "more",
            "than",
            "one",
            "card",
            "each",
            "turn",
        ] => StaticAbility::opponents_cant_draw_extra_cards(),
        ["counters", "cant", "be", "put", "on", "this", "permanent"] => {
            StaticAbility::cant_have_counters_placed()
        }
        ["this", "spell", "cant", "be", "countered"] => StaticAbility::cant_be_countered_ability(),
        ["this", "creature", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "block"] => StaticAbility::cant_block(),
        ["this", "cant", "attack"] => StaticAbility::cant_attack(),
        ["this", "creature", "cant", "attack", "or", "block"] => StaticAbility::custom(
            "cant_attack_or_block",
            "this creature can't attack or block".to_string(),
        ),
        ["this", "cant", "attack", "or", "block"] => StaticAbility::custom(
            "cant_attack_or_block",
            "this can't attack or block".to_string(),
        ),
        ["this", "creature", "cant", "attack", "or", "block", "alone"] => {
            StaticAbility::custom(
                "cant_attack_or_block_alone",
                "this creature can't attack or block alone".to_string(),
            )
        }
        ["this", "cant", "attack", "or", "block", "alone"] => StaticAbility::custom(
            "cant_attack_or_block_alone",
            "this can't attack or block alone".to_string(),
        ),
        ["you", "cant", "cast", "creature", "spells"] => StaticAbility::custom(
            "cant_cast_creature_spells",
            "you can't cast creature spells".to_string(),
        ),
        ["each", "player", "cant", "cast", "more", "than", "one", "spell", "each", "turn"] => {
            StaticAbility::custom(
                "cant_cast_more_than_one_spell_each_turn",
                "each player can't cast more than one spell each turn".to_string(),
            )
        }
        ["permanents", "you", "control", "cant", "be", "sacrificed"] => {
            StaticAbility::permanents_you_control_cant_be_sacrificed()
        }
        ["this", "creature", "cant", "be", "blocked"] => StaticAbility::unblockable(),
        _ => return Ok(None),
    };

    Ok(Some(ability))
}

fn parse_cant_restrictions(
    tokens: &[Token],
) -> Result<Option<Vec<ParsedCantRestriction>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }
        let shared_subject = find_negation_span(&segments[0])
            .map(|(neg_start, _)| trim_commas(&segments[0][..neg_start]))
            .unwrap_or_default();

        let mut restrictions = Vec::new();
        for (idx, segment) in segments.iter().enumerate() {
            let mut expanded = segment.clone();
            if idx > 0
                && !shared_subject.is_empty()
                && matches!(find_negation_span(segment), Some((0, _)))
            {
                let mut with_subject = shared_subject.clone();
                with_subject.extend(segment.clone());
                expanded = with_subject;
            }
            let Some(restriction) = parse_cant_restriction_clause(&expanded)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant restriction segment (clause: '{}')",
                    words(segment).join(" ")
                )));
            };
            restrictions.push(restriction);
        }

        return Ok(Some(restrictions));
    }

    parse_cant_restriction_clause(tokens).map(|restriction| restriction.map(|r| vec![r]))
}

fn parse_cant_restriction_clause(
    tokens: &[Token],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let restriction = match normalized.as_slice() {
        ["players", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Any),
        ["players", "cant", "search", "libraries"] => {
            Restriction::search_libraries(PlayerFilter::Any)
        }
        ["players", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::Any),
        ["players", "cant", "cast", "spells"] => Restriction::cast_spells(PlayerFilter::Any),
        [
            "players",
            "cant",
            "draw",
            "more",
            "than",
            "one",
            "card",
            "each",
            "turn",
        ] => Restriction::draw_extra_cards(PlayerFilter::Any),
        ["damage", "cant", "be", "prevented"] => Restriction::prevent_damage(),
        ["you", "cant", "lose", "the", "game"] => Restriction::lose_game(PlayerFilter::You),
        ["your", "opponents", "cant", "win", "the", "game"] => {
            Restriction::win_game(PlayerFilter::Opponent)
        }
        ["your", "life", "total", "cant", "change"] => {
            Restriction::change_life_total(PlayerFilter::You)
        }
        ["your", "opponents", "cant", "cast", "spells"] => {
            Restriction::cast_spells(PlayerFilter::Opponent)
        }
        [
            "your",
            "opponents",
            "cant",
            "draw",
            "more",
            "than",
            "one",
            "card",
            "each",
            "turn",
        ] => Restriction::draw_extra_cards(PlayerFilter::Opponent),
        ["you", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::You),
        ["you", "cant", "search", "libraries"] => Restriction::search_libraries(PlayerFilter::You),
        ["you", "cant", "draw", "cards"] => Restriction::draw_cards(PlayerFilter::You),
        ["opponents", "cant", "gain", "life"] => Restriction::gain_life(PlayerFilter::Opponent),
        ["opponents", "cant", "cast", "spells"] => Restriction::cast_spells(PlayerFilter::Opponent),
        _ => return parse_negated_object_restriction_clause(tokens),
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target: None,
    }))
}

fn parse_negated_object_restriction_clause(
    tokens: &[Token],
) -> Result<Option<ParsedCantRestriction>, CardTextError> {
    use crate::effect::Restriction;

    let Some((neg_start, neg_end)) = find_negation_span(tokens) else {
        return Ok(None);
    };
    let subject_tokens = trim_commas(&tokens[..neg_start]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in negated restriction clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let (filter, target) = if starts_with_target_indicator(&subject_tokens) {
        let target = parse_target_phrase(&subject_tokens)?;
        let mut filter = target_ast_to_object_filter(target.clone()).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported target restriction subject (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
        if !filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG)
        {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
        }
        (filter, Some(target))
    } else {
        let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported subject in negated restriction clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        };
        (filter, None)
    };

    let remainder_tokens = trim_commas(&tokens[neg_end..]);
    if remainder_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing restriction tail in negated restriction clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let remainder_words = normalize_cant_words(&remainder_tokens);

    let restriction = match remainder_words.as_slice() {
        ["attack"] => Restriction::attack(filter),
        ["block"] => Restriction::block(filter),
        ["be", "blocked"] => Restriction::be_blocked(filter),
        ["be", "destroyed"] => Restriction::be_destroyed(filter),
        ["be", "sacrificed"] => Restriction::be_sacrificed(filter),
        ["be", "countered"] => Restriction::be_countered(filter),
        ["be", "targeted"] => Restriction::be_targeted(filter),
        _ if is_supported_untap_restriction_tail(&remainder_words) => Restriction::untap(filter),
        _ => {
            return Err(CardTextError::ParseError(format!(
                "unsupported negated restriction tail (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    };

    Ok(Some(ParsedCantRestriction {
        restriction,
        target,
    }))
}

#[derive(Debug, Clone)]
struct ParsedCantRestriction {
    restriction: crate::effect::Restriction,
    target: Option<TargetAst>,
}

fn starts_with_target_indicator(tokens: &[Token]) -> bool {
    let mut idx = 0usize;
    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((_, used)) = parse_number(&tokens[idx..]) {
            idx += used;
        }
    } else if let Some((_, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        idx += used;
    } else if let Some((_, used)) = parse_number(&tokens[idx..])
        && tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"))
    {
        idx += used;
    } else if tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        idx += 1;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        idx += 1;
    }

    tokens.get(idx).is_some_and(|token| token.is_word("target"))
}

fn find_negation_span(tokens: &[Token]) -> Option<(usize, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "cant" | "cannot" | "doesnt" | "dont") {
            return Some((idx, idx + 1));
        }
        if (word == "does" || word == "do" || word == "can")
            && tokens.get(idx + 1).is_some_and(|next| next.is_word("not"))
        {
            return Some((idx, idx + 2));
        }
    }
    None
}

fn parse_subject_object_filter(tokens: &[Token]) -> Result<Option<ObjectFilter>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let target = parse_target_phrase(tokens).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject target phrase (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    Ok(target_ast_to_object_filter(target))
}

fn target_ast_to_object_filter(target: TargetAst) -> Option<ObjectFilter> {
    match target {
        TargetAst::Source(_) => Some(ObjectFilter::source()),
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::Tagged(tag, _) => Some(ObjectFilter::tagged(tag)),
        TargetAst::WithCount(inner, _) => target_ast_to_object_filter(*inner),
        _ => None,
    }
}

fn is_supported_untap_restriction_tail(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }
    if !(words[0] == "untap" || words[0] == "untaps") {
        return false;
    }
    if words.len() == 1 {
        return true;
    }

    let allowed = [
        "untap",
        "untaps",
        "during",
        "its",
        "their",
        "your",
        "controllers",
        "controller",
        "untap",
        "step",
        "next",
        "the",
    ];
    if words.iter().any(|word| !allowed.contains(word)) {
        return false;
    }

    words.contains(&"during") && words.contains(&"step")
}

fn normalize_cant_words(tokens: &[Token]) -> Vec<&str> {
    words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect()
}

fn keyword_title(keyword: &str) -> String {
    let mut words = keyword.split_whitespace();
    let Some(first) = words.next() else {
        return String::new();
    };
    let mut out = String::new();
    let mut first_chars = first.chars();
    if let Some(ch) = first_chars.next() {
        out.push(ch.to_ascii_uppercase());
        out.push_str(first_chars.as_str());
    }
    for word in words {
        out.push(' ');
        out.push_str(word);
    }
    out
}

fn leading_mana_symbols_to_oracle(words: &[&str]) -> Option<(String, usize)> {
    if words.is_empty() {
        return None;
    }
    let mut pips = Vec::new();
    let mut consumed = 0usize;
    for word in words {
        let Ok(symbol) = parse_mana_symbol(word) else {
            break;
        };
        pips.push(vec![symbol]);
        consumed += 1;
    }
    if consumed == 0 {
        return None;
    }
    Some((ManaCost::from_pips(pips).to_oracle(), consumed))
}

fn marker_keyword_id(keyword: &str) -> Option<&'static str> {
    match keyword {
        "banding" => Some("banding"),
        "fabricate" => Some("fabricate"),
        "bestow" => Some("bestow"),
        "crew" => Some("crew"),
        "dash" => Some("dash"),
        "soulshift" => Some("soulshift"),
        "adapt" => Some("adapt"),
        "afterlife" => Some("afterlife"),
        "bolster" => Some("bolster"),
        "devour" => Some("devour"),
        "disturb" => Some("disturb"),
        "echo" => Some("echo"),
        "modular" => Some("modular"),
        "ninjutsu" => Some("ninjutsu"),
        "outlast" => Some("outlast"),
        "scavenge" => Some("scavenge"),
        "suspend" => Some("suspend"),
        "training" => Some("training"),
        "vanishing" => Some("vanishing"),
        "offering" => Some("offering"),
        "soulbond" => Some("soulbond"),
        "unearth" => Some("unearth"),
        "specialize" => Some("specialize"),
        "squad" => Some("squad"),
        "spectacle" => Some("spectacle"),
        "graft" => Some("graft"),
        "haunt" => Some("haunt"),
        "backup" => Some("backup"),
        "casualty" => Some("casualty"),
        "saddle" => Some("saddle"),
        "fading" => Some("fading"),
        "conspire" => Some("conspire"),
        "fuse" => Some("fuse"),
        "plot" => Some("plot"),
        "disguise" => Some("disguise"),
        "tribute" => Some("tribute"),
        "buyback" => Some("buyback"),
        "flashback" => Some("flashback"),
        "rebound" => Some("rebound"),
        "renown" => Some("renown"),
        _ => None,
    }
}

fn marker_keyword_display(words: &[&str]) -> Option<String> {
    let keyword = words.first().copied()?;
    let title = keyword_title(keyword);

    match keyword {
        "fabricate" | "crew" | "soulshift" | "adapt" | "afterlife" | "bolster" | "devour"
        | "modular" | "vanishing" | "backup" | "casualty" | "saddle" | "fading" | "graft"
        | "tribute" | "renown" => {
            let amount = words.get(1)?.parse::<u32>().ok()?;
            Some(format!("{title} {amount}"))
        }
        "bestow" | "dash" | "disturb" | "echo" | "ninjutsu" | "outlast" | "scavenge"
        | "unearth" | "specialize" | "spectacle" | "plot" | "disguise" | "flashback" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("{title} {cost}"))
        }
        "buyback" => {
            if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[1..]) {
                Some(format!("Buyback {cost}"))
            } else if words.len() > 1 {
                Some(format!("Buyback—{}", words[1..].join(" ")))
            } else {
                Some("Buyback".to_string())
            }
        }
        "suspend" => {
            let time = words.get(1)?.parse::<u32>().ok()?;
            let (cost, _) = leading_mana_symbols_to_oracle(&words[2..])?;
            Some(format!("Suspend {time} {cost}"))
        }
        "rebound" => Some("Rebound".to_string()),
        "squad" => {
            let (cost, _) = leading_mana_symbols_to_oracle(&words[1..])?;
            Some(format!("Squad {cost}"))
        }
        _ => None,
    }
}

fn parse_ability_phrase(tokens: &[Token]) -> Option<KeywordAction> {
    let mut words = words(tokens);
    if words.is_empty() {
        return None;
    }

    if words.first().copied() == Some("and") {
        words.remove(0);
    }

    if words.starts_with(&["cumulative", "upkeep"]) {
        let mut text = "Cumulative upkeep".to_string();
        let tail = &words[2..];
        if !tail.is_empty() {
            if tail.first().copied() == Some("add")
                && let Some((cost, consumed)) = leading_mana_symbols_to_oracle(&tail[1..])
                && consumed + 1 == tail.len()
            {
                text = format!("Cumulative upkeep—Add {cost}");
            } else if let Some((cost, consumed)) = leading_mana_symbols_to_oracle(tail)
                && consumed == tail.len()
            {
                text = format!("Cumulative upkeep {cost}");
            } else if tail.len() == 3
                && tail[1] == "or"
                && let (Some((left, 1)), Some((right, 1))) = (
                    leading_mana_symbols_to_oracle(&tail[..1]),
                    leading_mana_symbols_to_oracle(&tail[2..3]),
                )
            {
                text = format!("Cumulative upkeep {left} or {right}");
            } else {
                text.push(' ');
                text.push_str(&tail.join(" "));
            }
        }
        return Some(KeywordAction::MarkerText(text));
    }

    // Bushido appears as "Bushido N" and is often followed by reminder text.
    if words.first().copied() == Some("bushido") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Bushido(amount));
        }
        return Some(KeywordAction::Marker("bushido"));
    }

    // Bloodthirst appears as "Bloodthirst N" and is often followed by reminder text.
    if words.first().copied() == Some("bloodthirst") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Bloodthirst(amount));
        }
        return Some(KeywordAction::Marker("bloodthirst"));
    }

    // Rampage appears as "Rampage N" and is often followed by reminder text.
    if words.first().copied() == Some("rampage") {
        if words.len() >= 2
            && let Ok(amount) = words[1].parse::<u32>()
        {
            return Some(KeywordAction::Rampage(amount));
        }
        return Some(KeywordAction::Marker("rampage"));
    }

    if words.as_slice().starts_with(&["battle", "cry"]) {
        return Some(KeywordAction::Marker("battle cry"));
    }
    if words.as_slice().starts_with(&["split", "second"]) {
        return Some(KeywordAction::Marker("split second"));
    }
    if words.as_slice().starts_with(&["doctor", "companion"]) {
        return Some(KeywordAction::Marker("doctor companion"));
    }
    if words.as_slice().starts_with(&["splice", "onto", "arcane"]) {
        if let Some((cost, _)) = leading_mana_symbols_to_oracle(&words[3..]) {
            return Some(KeywordAction::MarkerText(format!("Splice onto Arcane {cost}")));
        }
        return Some(KeywordAction::Marker("splice onto arcane"));
    }

    if let Some(first) = words.first().copied()
        && matches!(
            first,
            "banding"
                | "fabricate"
                | "bestow"
                | "crew"
                | "dash"
                | "soulshift"
                | "adapt"
                | "afterlife"
                | "bolster"
                | "devour"
                | "disturb"
                | "echo"
                | "modular"
                | "ninjutsu"
                | "outlast"
                | "scavenge"
                | "suspend"
                | "training"
                | "vanishing"
                | "offering"
                | "soulbond"
                | "unearth"
                | "specialize"
                | "squad"
                | "spectacle"
                | "graft"
                | "haunt"
                | "backup"
                | "casualty"
                | "saddle"
                | "fading"
                | "conspire"
                | "fuse"
                | "plot"
                | "disguise"
                | "tribute"
                | "buyback"
                | "flashback"
                | "rebound"
                | "renown"
        )
    {
        if let Some(display) = marker_keyword_display(&words) {
            return Some(KeywordAction::MarkerText(display));
        }
        if words.len() > 1 {
            return None;
        }
        return Some(KeywordAction::Marker(marker_keyword_id(first).expect(
            "marker keyword id must exist for matched keyword",
        )));
    }

    let action = match words.as_slice() {
        ["flying"] => KeywordAction::Flying,
        ["menace"] => KeywordAction::Menace,
        ["hexproof"] => KeywordAction::Hexproof,
        ["haste"] => KeywordAction::Haste,
        ["improvise"] => KeywordAction::Improvise,
        ["convoke"] => KeywordAction::Convoke,
        ["affinity", "for", "artifacts"] => KeywordAction::AffinityForArtifacts,
        ["delve"] => KeywordAction::Delve,
        ["first", "strike"] => KeywordAction::FirstStrike,
        ["double", "strike"] => KeywordAction::DoubleStrike,
        ["deathtouch"] => KeywordAction::Deathtouch,
        ["lifelink"] => KeywordAction::Lifelink,
        ["vigilance"] => KeywordAction::Vigilance,
        ["trample"] => KeywordAction::Trample,
        ["reach"] => KeywordAction::Reach,
        ["defender"] => KeywordAction::Defender,
        ["flash"] => KeywordAction::Flash,
        ["phasing"] => KeywordAction::Phasing,
        ["indestructible"] => KeywordAction::Indestructible,
        ["shroud"] => KeywordAction::Shroud,
        ["assist"] => KeywordAction::Marker("assist"),
        ["cipher"] => KeywordAction::Marker("cipher"),
        ["devoid"] => KeywordAction::Devoid,
        ["dethrone"] => KeywordAction::Marker("dethrone"),
        ["enlist"] => KeywordAction::Marker("enlist"),
        ["evolve"] => KeywordAction::Marker("evolve"),
        ["extort"] => KeywordAction::Marker("extort"),
        ["ingest"] => KeywordAction::Marker("ingest"),
        ["mentor"] => KeywordAction::Marker("mentor"),
        ["myriad"] => KeywordAction::Marker("myriad"),
        ["partner"] => KeywordAction::Marker("partner"),
        ["populate"] => KeywordAction::Marker("populate"),
        ["provoke"] => KeywordAction::Marker("provoke"),
        ["ravenous"] => KeywordAction::Marker("ravenous"),
        ["riot"] => KeywordAction::Marker("riot"),
        ["skulk"] => KeywordAction::Marker("skulk"),
        ["sunburst"] => KeywordAction::Marker("sunburst"),
        ["undaunted"] => KeywordAction::Marker("undaunted"),
        ["unleash"] => KeywordAction::Marker("unleash"),
        ["ward", amount] => {
            let value = amount.parse::<u32>().ok()?;
            KeywordAction::Ward(value)
        }
        ["wither"] => KeywordAction::Wither,
        ["infect"] => KeywordAction::Infect,
        ["undying"] => KeywordAction::Undying,
        ["persist"] => KeywordAction::Persist,
        ["prowess"] => KeywordAction::Prowess,
        ["exalted"] => KeywordAction::Exalted,
        ["cascade"] => KeywordAction::Marker("cascade"),
        ["storm"] => KeywordAction::Storm,
        ["ascend"] => KeywordAction::Marker("ascend"),
        ["daybound"] => KeywordAction::Marker("daybound"),
        ["nightbound"] => KeywordAction::Marker("nightbound"),
        ["islandwalk"] => KeywordAction::Landwalk(Subtype::Island),
        ["swampwalk"] => KeywordAction::Landwalk(Subtype::Swamp),
        ["mountainwalk"] => KeywordAction::Landwalk(Subtype::Mountain),
        ["forestwalk"] => KeywordAction::Landwalk(Subtype::Forest),
        ["plainswalk"] => KeywordAction::Landwalk(Subtype::Plains),
        ["fear"] => KeywordAction::Fear,
        ["intimidate"] => KeywordAction::Intimidate,
        ["shadow"] => KeywordAction::Shadow,
        ["horsemanship"] => KeywordAction::Horsemanship,
        ["flanking"] => KeywordAction::Flanking,
        ["changeling"] => KeywordAction::Changeling,
        ["protection", "from", "all", "colors"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "all", "color"] => KeywordAction::ProtectionFromAllColors,
        ["protection", "from", "colorless"] => KeywordAction::ProtectionFromColorless,
        ["protection", "from", value] => match *value {
            "white" => KeywordAction::ProtectionFrom(ColorSet::WHITE),
            "blue" => KeywordAction::ProtectionFrom(ColorSet::BLUE),
            "black" => KeywordAction::ProtectionFrom(ColorSet::BLACK),
            "red" => KeywordAction::ProtectionFrom(ColorSet::RED),
            "green" => KeywordAction::ProtectionFrom(ColorSet::GREEN),
            _ => {
                if let Some(card_type) = parse_card_type(value) {
                    KeywordAction::ProtectionFromCardType(card_type)
                } else if let Some(subtype) = parse_subtype_word(value)
                    .or_else(|| value.strip_suffix('s').and_then(parse_subtype_word))
                {
                    KeywordAction::ProtectionFromSubtype(subtype)
                } else {
                    return None;
                }
            }
        },
        _ => {
            // "toxic N" needs exactly 2 words
            if words.len() == 2 && words[0] == "toxic" {
                let amount = words[1].parse::<u32>().ok().unwrap_or(1);
                return Some(KeywordAction::Toxic(amount));
            }
            // Single-word keywords: only match when the segment is exactly one word
            // to avoid swallowing compound phrases like "menace and deathtouch"
            if words.len() == 1 {
                match words[0] {
                    "flying" => return Some(KeywordAction::Flying),
                    "menace" => return Some(KeywordAction::Menace),
                    "hexproof" => return Some(KeywordAction::Hexproof),
                    "haste" => return Some(KeywordAction::Haste),
                    "improvise" => return Some(KeywordAction::Improvise),
                    "convoke" => return Some(KeywordAction::Convoke),
                    "delve" => return Some(KeywordAction::Delve),
                    "deathtouch" => return Some(KeywordAction::Deathtouch),
                    "lifelink" => return Some(KeywordAction::Lifelink),
                    "vigilance" => return Some(KeywordAction::Vigilance),
                    "trample" => return Some(KeywordAction::Trample),
                    "reach" => return Some(KeywordAction::Reach),
                    "defender" => return Some(KeywordAction::Defender),
                    "flash" => return Some(KeywordAction::Flash),
                    "phasing" => return Some(KeywordAction::Phasing),
                    "indestructible" => return Some(KeywordAction::Indestructible),
                    "shroud" => return Some(KeywordAction::Shroud),
                    "assist" => return Some(KeywordAction::Marker("assist")),
                    "cipher" => return Some(KeywordAction::Marker("cipher")),
                    "devoid" => return Some(KeywordAction::Devoid),
                    "dethrone" => return Some(KeywordAction::Marker("dethrone")),
                    "enlist" => return Some(KeywordAction::Marker("enlist")),
                    "evolve" => return Some(KeywordAction::Marker("evolve")),
                    "extort" => return Some(KeywordAction::Marker("extort")),
                    "ingest" => return Some(KeywordAction::Marker("ingest")),
                    "mentor" => return Some(KeywordAction::Marker("mentor")),
                    "myriad" => return Some(KeywordAction::Marker("myriad")),
                    "partner" => return Some(KeywordAction::Marker("partner")),
                    "populate" => return Some(KeywordAction::Marker("populate")),
                    "provoke" => return Some(KeywordAction::Marker("provoke")),
                    "ravenous" => return Some(KeywordAction::Marker("ravenous")),
                    "riot" => return Some(KeywordAction::Marker("riot")),
                    "skulk" => return Some(KeywordAction::Marker("skulk")),
                    "sunburst" => return Some(KeywordAction::Marker("sunburst")),
                    "undaunted" => return Some(KeywordAction::Marker("undaunted")),
                    "unleash" => return Some(KeywordAction::Marker("unleash")),
                    "wither" => return Some(KeywordAction::Wither),
                    "infect" => return Some(KeywordAction::Infect),
                    "undying" => return Some(KeywordAction::Undying),
                    "persist" => return Some(KeywordAction::Persist),
                    "prowess" => return Some(KeywordAction::Prowess),
                    "exalted" => return Some(KeywordAction::Exalted),
                    "cascade" => return Some(KeywordAction::Marker("cascade")),
                    "storm" => return Some(KeywordAction::Storm),
                    "ascend" => return Some(KeywordAction::Marker("ascend")),
                    "daybound" => return Some(KeywordAction::Marker("daybound")),
                    "nightbound" => return Some(KeywordAction::Marker("nightbound")),
                    "islandwalk" => return Some(KeywordAction::Landwalk(Subtype::Island)),
                    "swampwalk" => return Some(KeywordAction::Landwalk(Subtype::Swamp)),
                    "mountainwalk" => return Some(KeywordAction::Landwalk(Subtype::Mountain)),
                    "forestwalk" => return Some(KeywordAction::Landwalk(Subtype::Forest)),
                    "plainswalk" => return Some(KeywordAction::Landwalk(Subtype::Plains)),
                    "fear" => return Some(KeywordAction::Fear),
                    "intimidate" => return Some(KeywordAction::Intimidate),
                    "shadow" => return Some(KeywordAction::Shadow),
                    "horsemanship" => return Some(KeywordAction::Horsemanship),
                    "flanking" => return Some(KeywordAction::Flanking),
                    "changeling" => return Some(KeywordAction::Changeling),
                    _ => {}
                }
            }
            if words.len() >= 2 {
                if words.starts_with(&["first", "strike"]) {
                    if words.len() > 2 && words.contains(&"and") {
                        return None;
                    }
                    return Some(KeywordAction::FirstStrike);
                }
                if words.starts_with(&["double", "strike"]) {
                    if words.len() > 2 && words.contains(&"and") {
                        return None;
                    }
                    return Some(KeywordAction::DoubleStrike);
                }
                if words.starts_with(&["protection", "from"]) && words.len() >= 3 {
                    let value = words[2];
                    return match value {
                        "white" => Some(KeywordAction::ProtectionFrom(ColorSet::WHITE)),
                        "blue" => Some(KeywordAction::ProtectionFrom(ColorSet::BLUE)),
                        "black" => Some(KeywordAction::ProtectionFrom(ColorSet::BLACK)),
                        "red" => Some(KeywordAction::ProtectionFrom(ColorSet::RED)),
                        "green" => Some(KeywordAction::ProtectionFrom(ColorSet::GREEN)),
                        _ => parse_card_type(value)
                            .map(KeywordAction::ProtectionFromCardType)
                            .or_else(|| {
                                parse_subtype_word(value)
                                    .or_else(|| {
                                        value.strip_suffix('s').and_then(parse_subtype_word)
                                    })
                                    .map(KeywordAction::ProtectionFromSubtype)
                            }),
                    };
                }
            }
            if words.len() >= 3 {
                let suffix = &words[words.len() - 3..];
                if suffix == ["cant", "be", "blocked"] || suffix == ["cannot", "be", "blocked"] {
                    return Some(KeywordAction::Unblockable);
                }
            }
            return None;
        }
    };

    Some(action)
}

fn parse_triggered_line(tokens: &[Token]) -> Result<LineAst, CardTextError> {
    let start_idx = if tokens.first().is_some_and(|token| {
        token.is_word("whenever") || token.is_word("at") || token.is_word("when")
    }) {
        1
    } else {
        0
    };

    if let Some(mut split_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .or_else(|| tokens.iter().position(|token| token.is_word("then")))
    {
        // Handle trigger lists like "Whenever you cast an Aura, Equipment, or Vehicle spell, ..."
        // by advancing to the next comma when the first split clearly stays inside the trigger text.
        if matches!(tokens.get(split_idx), Some(Token::Comma(_)))
            && tokens
                .first()
                .is_some_and(|token| token.is_word("whenever") || token.is_word("when"))
        {
            let tail = &tokens[split_idx + 1..];
            if looks_like_trigger_type_list_tail(tail)
                || looks_like_trigger_color_list_tail(tail)
                || looks_like_trigger_object_list_tail(tail)
            {
                let next_comma_rel = tail
                    .iter()
                    .enumerate()
                    .find_map(|(idx, token)| {
                        if !matches!(token, Token::Comma(_)) {
                            return None;
                        }
                        let before_words = words(&tail[..idx]);
                        if before_words.contains(&"spell") || before_words.contains(&"spells") {
                            Some(idx)
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        if looks_like_trigger_color_list_tail(tail)
                            || looks_like_trigger_object_list_tail(tail)
                        {
                            tail.iter().enumerate().rev().find_map(|(idx, token)| {
                                if matches!(token, Token::Comma(_)) {
                                    Some(idx)
                                } else {
                                    None
                                }
                            })
                        } else {
                            None
                        }
                    });
                if let Some(next_comma_rel) = next_comma_rel {
                    let candidate_idx = split_idx + 1 + next_comma_rel;
                    if candidate_idx > start_idx && candidate_idx + 1 < tokens.len() {
                        split_idx = candidate_idx;
                    }
                }
            }
        }

        let trigger_tokens = &tokens[start_idx..split_idx];
        let trigger = parse_trigger_clause(trigger_tokens)?;
        let effects_tokens =
            rewrite_attached_controller_trigger_effect_tokens(trigger_tokens, &tokens[split_idx + 1..]);
        let effects = parse_effect_sentences(&effects_tokens)?;
        return Ok(LineAst::Triggered {
            trigger,
            effects,
            once_each_turn: triggered_once_each_turn_sentence(&split_on_period(&effects_tokens)),
        });
    }

    // Some oracle lines omit the comma after the trigger clause.
    for split_idx in ((start_idx + 1)..tokens.len()).rev() {
        let trigger_tokens = &tokens[start_idx..split_idx];
        let effects_tokens = &tokens[split_idx..];
        if effects_tokens.is_empty() {
            continue;
        }
        if let Ok(trigger) = parse_trigger_clause(trigger_tokens) {
            let rewritten_effects_tokens =
                rewrite_attached_controller_trigger_effect_tokens(trigger_tokens, effects_tokens);
            if let Ok(effects) = parse_effect_sentences(&rewritten_effects_tokens)
        {
            return Ok(LineAst::Triggered {
                trigger,
                effects,
                once_each_turn: triggered_once_each_turn_sentence(&split_on_period(
                    &rewritten_effects_tokens,
                )),
            });
        }
        }
    }

    Err(CardTextError::ParseError(format!(
        "missing comma in triggered line (clause: '{}')",
        words(tokens).join(" ")
    )))
}

fn rewrite_attached_controller_trigger_effect_tokens(
    trigger_tokens: &[Token],
    effects_tokens: &[Token],
) -> Vec<Token> {
    let trigger_words = words(trigger_tokens);
    let references_enchanted_controller = trigger_words.windows(3).any(|window| {
        window[0] == "enchanted"
            && matches!(window[1], "creature" | "creatures" | "permanent" | "permanents")
            && window[2] == "controller"
    });
    if !references_enchanted_controller {
        return effects_tokens.to_vec();
    }

    let mut rewritten = Vec::with_capacity(effects_tokens.len());
    let mut idx = 0usize;
    while idx < effects_tokens.len() {
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("creature")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(Token::Word("enchanted".to_string(), first_span));
            rewritten.push(Token::Word("creature".to_string(), second_span));
            idx += 2;
            continue;
        }
        if idx + 1 < effects_tokens.len()
            && effects_tokens[idx].is_word("that")
            && effects_tokens[idx + 1].is_word("permanent")
        {
            let first_span = effects_tokens[idx].span();
            let second_span = effects_tokens[idx + 1].span();
            rewritten.push(Token::Word("enchanted".to_string(), first_span));
            rewritten.push(Token::Word("permanent".to_string(), second_span));
            idx += 2;
            continue;
        }
        rewritten.push(effects_tokens[idx].clone());
        idx += 1;
    }

    rewritten
}

fn looks_like_trigger_object_list_tail(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }

    let words = words(tokens);
    if words.is_empty() {
        return false;
    }

    let starts_with_or = words.first().copied() == Some("or");
    let first_candidate = if starts_with_or {
        words.get(1).copied()
    } else {
        words.first().copied()
    };
    let Some(first_word) = first_candidate else {
        return false;
    };

    let type_like = parse_card_type(first_word).is_some()
        || first_word
            .strip_suffix('s')
            .is_some_and(|stem| parse_card_type(stem).is_some());
    if !type_like {
        return false;
    }

    tokens
        .iter()
        .any(|token| matches!(token, Token::Comma(_)))
}

fn looks_like_trigger_type_list_tail(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    let first_is_card_type = parse_card_type(words[0]).is_some()
        || parse_subtype_word(words[0]).is_some()
        || words[0].strip_suffix('s').is_some_and(|word| {
            parse_card_type(word).is_some() || parse_subtype_word(word).is_some()
        });
    first_is_card_type
        && (words.contains(&"spell") || words.contains(&"spells"))
        && words.contains(&"or")
        && tokens.iter().any(|token| matches!(token, Token::Comma(_)))
}

fn looks_like_trigger_color_list_tail(tokens: &[Token]) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    is_basic_color_word(words[0])
        && words.contains(&"or")
        && tokens.iter().any(|token| matches!(token, Token::Comma(_)))
}

fn strip_leading_trigger_intro(tokens: &[Token]) -> &[Token] {
    if tokens.first().is_some_and(|token| {
        token.is_word("when") || token.is_word("whenever") || token.is_word("at")
    }) {
        &tokens[1..]
    } else {
        tokens
    }
}

fn split_trigger_or_index(tokens: &[Token]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if !token.is_word("or") {
            return None;
        }
        // Keep quantifiers like "one or more <subject>" intact.
        let quantifier_or = idx > 0
            && tokens
                .get(idx - 1)
                .is_some_and(|prev| prev.is_word("one"))
            && tokens
                .get(idx + 1)
                .is_some_and(|next| next.is_word("more"));
        if quantifier_or { None } else { Some(idx) }
    })
}

fn parse_trigger_clause(tokens: &[Token]) -> Result<TriggerSpec, CardTextError> {
    let words = words(tokens);

    if words.len() == 9
        && words.first().copied() == Some("this")
        && words.get(1).copied() == Some("creature")
        && words.get(2).copied() == Some("and")
        && words.get(3).copied() == Some("at")
        && words.get(4).copied() == Some("least")
        && words.get(6).copied() == Some("other")
        && words
            .get(7)
            .is_some_and(|word| *word == "creature" || *word == "creatures")
        && words
            .last()
            .is_some_and(|word| *word == "attack" || *word == "attacks")
    {
        let other_count = parse_cardinal_u32(words[5]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "invalid battalion attacker count in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::ThisAttacksWithNOthers(other_count));
    }

    if let Some(or_idx) = tokens.iter().position(|token| token.is_word("or"))
        && words.last().copied() == Some("dies")
        && tokens.first().is_some_and(|token| token.is_word("this"))
    {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..tokens.len() - 1];
        if left_tokens.len() == 1
            && left_tokens[0].is_word("this")
            && let Ok(filter) = parse_object_filter(right_tokens, false)
        {
            return Ok(TriggerSpec::Either(
                Box::new(TriggerSpec::ThisDies),
                Box::new(TriggerSpec::Dies(filter)),
            ));
        }
    }

    if let Some(or_idx) = split_trigger_or_index(tokens) {
        let left_tokens = &tokens[..or_idx];
        let right_tokens = &tokens[or_idx + 1..];
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause(left_tokens),
                parse_trigger_clause(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and"))
        && tokens.get(and_idx + 1).is_some_and(|token| {
            token.is_word("whenever") || token.is_word("when") || token.is_word("at")
        })
    {
        let left_tokens = strip_leading_trigger_intro(&tokens[..and_idx]);
        let right_tokens = strip_leading_trigger_intro(&tokens[and_idx + 1..]);
        if !left_tokens.is_empty()
            && !right_tokens.is_empty()
            && let (Ok(left), Ok(right)) = (
                parse_trigger_clause(left_tokens),
                parse_trigger_clause(right_tokens),
            )
        {
            return Ok(TriggerSpec::Either(Box::new(left), Box::new(right)));
        }
    }
    let is_you_cast_this_spell = words
        .windows(3)
        .any(|window| window == ["cast", "this", "spell"] || window == ["casts", "this", "spell"]);
    if is_you_cast_this_spell && words.contains(&"you")
    {
        return Ok(TriggerSpec::YouCastThisSpell);
    }

    if let Some(spell_activity_trigger) = parse_spell_activity_trigger(tokens)? {
        return Ok(spell_activity_trigger);
    }

    if let Some(enters_idx) = tokens
        .iter()
        .position(|token| token.is_word("enters") || token.is_word("enter"))
    {
        if enters_idx == 0 {
            return Ok(TriggerSpec::ThisEntersBattlefield);
        }
        let subject_tokens = &tokens[..enters_idx];
        if let Some(or_idx) = subject_tokens.iter().position(|token| token.is_word("or")) {
            let left_tokens = &subject_tokens[..or_idx];
            let mut right_tokens = &subject_tokens[or_idx + 1..];
            let left_words: Vec<&str> = left_tokens
                .iter()
                .filter_map(Token::as_word)
                .filter(|word| !is_article(word))
                .collect();
            if is_source_reference_words(&left_words) && !right_tokens.is_empty() {
                let mut other = false;
                if right_tokens
                    .first()
                    .is_some_and(|token| token.is_word("another") || token.is_word("other"))
                {
                    other = true;
                    right_tokens = &right_tokens[1..];
                }
                if !right_tokens.is_empty()
                    && let Ok(mut filter) = parse_object_filter(right_tokens, other)
                {
                    if words.contains(&"under")
                        && words.contains(&"your")
                        && words.contains(&"control")
                    {
                        filter.controller = Some(PlayerFilter::You);
                    } else if words.contains(&"under")
                        && (words.contains(&"opponent") || words.contains(&"opponents"))
                        && words.contains(&"control")
                    {
                        filter.controller = Some(PlayerFilter::Opponent);
                    }
                    let right_trigger = if words.contains(&"untapped") {
                        TriggerSpec::EntersBattlefieldUntapped(filter)
                    } else if words.contains(&"tapped") {
                        TriggerSpec::EntersBattlefieldTapped(filter)
                    } else {
                        TriggerSpec::EntersBattlefield(filter)
                    };
                    return Ok(TriggerSpec::Either(
                        Box::new(TriggerSpec::ThisEntersBattlefield),
                        Box::new(right_trigger),
                    ));
                }
            }
        }
        if subject_tokens
            .first()
            .is_some_and(|token| token.is_word("this"))
        {
            return Ok(TriggerSpec::ThisEntersBattlefield);
        }
        let mut filtered_subject_tokens = subject_tokens;
        let mut other = false;
        if filtered_subject_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            filtered_subject_tokens = &filtered_subject_tokens[1..];
        }
        if let Ok(mut filter) = parse_object_filter(filtered_subject_tokens, other) {
            if words.contains(&"under") && words.contains(&"your") && words.contains(&"control") {
                filter.controller = Some(PlayerFilter::You);
            } else if words.contains(&"under")
                && (words.contains(&"opponent") || words.contains(&"opponents"))
                && words.contains(&"control")
            {
                filter.controller = Some(PlayerFilter::Opponent);
            }
            if words.contains(&"untapped") {
                return Ok(TriggerSpec::EntersBattlefieldUntapped(filter));
            }
            if words.contains(&"tapped") {
                return Ok(TriggerSpec::EntersBattlefieldTapped(filter));
            }
            return Ok(TriggerSpec::EntersBattlefield(filter));
        }
    }

    if words.as_slice() == ["players", "finish", "voting"]
        || words.as_slice() == ["players", "finished", "voting"]
    {
        return Ok(TriggerSpec::KeywordAction {
            action: crate::events::KeywordActionKind::Vote,
            player: PlayerFilter::Any,
        });
    }

    if let Some(last_word) = words.last().copied()
        && let Some(action) = crate::events::KeywordActionKind::from_trigger_word(last_word)
    {
        let subject = &words[..words.len().saturating_sub(1)];
        let player = parse_trigger_subject_player_filter(subject);
        if let Some(player) = player {
            return Ok(TriggerSpec::KeywordAction { action, player });
        }
    }

    let has_deal = words.iter().any(|word| *word == "deal" || *word == "deals");
    if has_deal
        && words.contains(&"combat")
        && words.contains(&"damage")
        && (words.contains(&"creature") || words.contains(&"creatures"))
        && !words.contains(&"player")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-damage-to-creature trigger clause (clause: '{}')",
            words.join(" ")
        )));
    }
    if has_deal
        && words.contains(&"combat")
        && words.contains(&"damage")
        && words.contains(&"player")
    {
        if let Some(deals_idx) = tokens
            .iter()
            .position(|token| token.is_word("deal") || token.is_word("deals"))
        {
            let subject_tokens = &tokens[..deals_idx];
            return Ok(match parse_trigger_subject_filter(subject_tokens)? {
                Some(filter) => TriggerSpec::DealsCombatDamageToPlayer(filter),
                None => TriggerSpec::ThisDealsCombatDamageToPlayer,
            });
        }
        return Ok(TriggerSpec::ThisDealsCombatDamageToPlayer);
    }

    if words.as_slice() == ["this", "becomes", "monstrous"] {
        return Ok(TriggerSpec::ThisBecomesMonstrous);
    }

    if (words.starts_with(&["this", "creature", "blocks"])
        || words.starts_with(&["this", "blocks"]))
        && let Some(blocks_idx) = tokens
            .iter()
            .position(|token| token.is_word("block") || token.is_word("blocks"))
    {
        let tail_tokens = trim_commas(&tokens[blocks_idx + 1..]);
        if !tail_tokens.is_empty() && !tail_tokens.first().is_some_and(|token| token.is_word("or"))
        {
            let blocked_filter = parse_object_filter(&tail_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported blocked-object filter in trigger clause (clause: '{}')",
                    words.join(" ")
                ))
            })?;
            return Ok(TriggerSpec::ThisBlocksObject(blocked_filter));
        }
    }

    if words.as_slice() == ["this", "creature", "blocks"] || words.as_slice() == ["this", "blocks"]
    {
        return Ok(TriggerSpec::ThisBlocks);
    }

    if words.as_slice() == ["this", "creature", "becomes", "blocked"]
        || words.as_slice() == ["this", "becomes", "blocked"]
    {
        return Ok(TriggerSpec::ThisBecomesBlocked);
    }

    if words.as_slice() == ["this", "creature", "attacks", "or", "blocks"]
        || words.as_slice() == ["this", "attacks", "or", "blocks"]
    {
        return Ok(TriggerSpec::Either(
            Box::new(TriggerSpec::ThisAttacks),
            Box::new(TriggerSpec::ThisBlocks),
        ));
    }

    if words.starts_with(&["this", "creature", "blocks", "or", "becomes", "blocked"])
        || words.starts_with(&["this", "blocks", "or", "becomes", "blocked"])
    {
        return Ok(TriggerSpec::ThisBlocksOrBecomesBlocked);
    }

    if words.as_slice() == ["this", "creature", "leaves", "the", "battlefield"]
        || words.as_slice() == ["this", "leaves", "the", "battlefield"]
    {
        return Ok(TriggerSpec::ThisLeavesBattlefield);
    }

    if words.as_slice() == ["this", "creature", "becomes", "tapped"]
        || words.as_slice() == ["this", "becomes", "tapped"]
    {
        return Ok(TriggerSpec::ThisBecomesTapped);
    }

    if words.as_slice() == ["this", "creature", "becomes", "untapped"]
        || words.as_slice() == ["this", "becomes", "untapped"]
    {
        return Ok(TriggerSpec::ThisBecomesUntapped);
    }

    if words.as_slice() == ["this", "creature", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "permanent", "is", "turned", "face", "up"]
        || words.as_slice() == ["this", "is", "turned", "face", "up"]
    {
        return Ok(TriggerSpec::ThisTurnedFaceUp);
    }

    if words.as_slice()
        == [
            "this", "creature", "becomes", "the", "target", "of", "a", "spell", "or", "ability",
        ]
        || words.as_slice()
            == [
                "this", "becomes", "the", "target", "of", "a", "spell", "or", "ability",
            ]
    {
        return Ok(TriggerSpec::ThisBecomesTargeted);
    }

    if words.starts_with(&["this", "creature", "is", "dealt", "damage"])
        || words.starts_with(&["this", "is", "dealt", "damage"])
    {
        return Ok(TriggerSpec::ThisIsDealtDamage);
    }

    if (words.starts_with(&["this", "creature", "deals", "damage", "to"])
        || words.starts_with(&["this", "deals", "damage", "to"]))
        && let Some(to_idx) = tokens.iter().position(|token| token.is_word("to"))
    {
        let target_tokens = trim_commas(&tokens[to_idx + 1..]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            )));
        }
        let target_filter = parse_object_filter(&target_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported damage recipient filter in trigger clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        return Ok(TriggerSpec::ThisDealsDamageTo(target_filter));
    }

    if words.starts_with(&["this", "creature", "deals", "damage"])
        || words.starts_with(&["this", "deals", "damage"])
    {
        return Ok(TriggerSpec::ThisDealsDamage);
    }

    if has_deal
        && words.contains(&"damage")
        && let Some(deals_idx) = tokens
            .iter()
            .position(|token| token.is_word("deal") || token.is_word("deals"))
    {
        let subject_tokens = &tokens[..deals_idx];
        return Ok(match parse_trigger_subject_filter(subject_tokens)? {
            Some(filter) => TriggerSpec::DealsDamage(filter),
            None => TriggerSpec::ThisDealsDamage,
        });
    }

    if words.as_slice() == ["you", "gain", "life"] {
        return Ok(TriggerSpec::YouGainLife);
    }

    if words.ends_with(&["lose", "life"]) || words.ends_with(&["loses", "life"]) {
        let subject = &words[..words.len().saturating_sub(2)];
        let player = parse_trigger_subject_player_filter(subject);
        if let Some(player) = player {
            return Ok(TriggerSpec::PlayerLosesLife(player));
        }
    }

    if words.ends_with(&["draw", "a", "card"]) || words.ends_with(&["draws", "a", "card"]) {
        let subject = &words[..words.len().saturating_sub(3)];
        if subject == ["you"] {
            return Ok(TriggerSpec::YouDrawCard);
        }
        if let Some(player) = parse_trigger_subject_player_filter(subject) {
            return Ok(TriggerSpec::PlayerDrawsCard(player));
        }
    }

    if let Some(sacrifice_idx) = tokens
        .iter()
        .position(|token| token.is_word("sacrifice") || token.is_word("sacrifices"))
    {
        let subject_words = &words[..sacrifice_idx];
        if let Some(player) = parse_trigger_subject_player_filter(subject_words) {
            let mut filter_tokens = &tokens[sacrifice_idx + 1..];
            if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("another") || token.is_word("other"))
            {
                filter_tokens = &filter_tokens[1..];
            }

            let filter = if filter_tokens.is_empty() {
                ObjectFilter::permanent()
            } else if filter_tokens
                .first()
                .is_some_and(|token| token.is_word("this") || token.is_word("it"))
            {
                let mut filter = ObjectFilter::source();
                let filter_words: Vec<&str> =
                    filter_tokens.iter().filter_map(Token::as_word).collect();
                if filter_words.contains(&"artifact") {
                    filter = filter.with_type(CardType::Artifact);
                } else if filter_words.contains(&"creature") {
                    filter = filter.with_type(CardType::Creature);
                } else if filter_words.contains(&"enchantment") {
                    filter = filter.with_type(CardType::Enchantment);
                } else if filter_words.contains(&"land") {
                    filter = filter.with_type(CardType::Land);
                } else if filter_words.contains(&"planeswalker") {
                    filter = filter.with_type(CardType::Planeswalker);
                }
                filter
            } else {
                parse_object_filter(filter_tokens, false).map_err(|_| {
                    CardTextError::ParseError(format!(
                        "unsupported sacrifice trigger filter (clause: '{}')",
                        words.join(" ")
                    ))
                })?
            };
            return Ok(TriggerSpec::PlayerSacrifices { player, filter });
        }
    }

    let last = words
        .last()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    match *last {
        "attack" | "attacks" => {
            let subject_tokens = if tokens.len() > 1 {
                &tokens[..tokens.len() - 1]
            } else {
                &[]
            };
            Ok(match parse_trigger_subject_filter(subject_tokens)? {
                Some(filter) => TriggerSpec::Attacks(filter),
                None => TriggerSpec::ThisAttacks,
            })
        }
        "dies" => {
            let mut subject_tokens = if tokens.len() > 1 {
                &tokens[..tokens.len() - 1]
            } else {
                &[]
            };

            if subject_tokens.is_empty()
                || subject_tokens
                    .first()
                    .is_some_and(|token| token.is_word("this"))
            {
                return Ok(TriggerSpec::ThisDies);
            }

            let mut other = false;
            if subject_tokens
                .first()
                .is_some_and(|token| token.is_word("another"))
            {
                other = true;
                subject_tokens = &subject_tokens[1..];
            }

            if subject_tokens.is_empty() {
                return Ok(TriggerSpec::ThisDies);
            }

            if let Ok(filter) = parse_object_filter(subject_tokens, other) {
                return Ok(TriggerSpec::Dies(filter));
            }

            Ok(TriggerSpec::ThisDies)
        }
        _ if words.contains(&"beginning") && words.contains(&"end") && words.contains(&"step") => {
            Ok(TriggerSpec::BeginningOfEndStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if words.contains(&"beginning") && words.contains(&"upkeep") => Ok(
            TriggerSpec::BeginningOfUpkeep(parse_possessive_clause_player_filter(&words)),
        ),
        _ if words.contains(&"beginning") && words.contains(&"draw") && words.contains(&"step") => {
            Ok(TriggerSpec::BeginningOfDrawStep(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"combat")
            && words.contains(&"turn") =>
        {
            Ok(TriggerSpec::BeginningOfCombat(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"precombat")
            && words.contains(&"main") =>
        {
            Ok(TriggerSpec::BeginningOfPrecombatMain(
                parse_possessive_clause_player_filter(&words),
            ))
        }
        _ => Ok(TriggerSpec::Custom(words.join(" "))),
    }
}

fn parse_possessive_clause_player_filter(words: &[&str]) -> PlayerFilter {
    let attached_controller_filter = |tag: &str| {
        PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(TagKey::from(tag)))
    };
    let has_attached_controller = |subject: &str| {
        words.windows(3).any(|window| {
            window[0] == subject
                && matches!(window[1], "creature" | "creatures" | "permanent" | "permanents")
                && window[2] == "controller"
        })
    };

    if has_attached_controller("enchanted") {
        return attached_controller_filter("enchanted");
    }
    if has_attached_controller("equipped") {
        return attached_controller_filter("equipped");
    }

    if contains_your_team_words(words) || words.contains(&"your") {
        PlayerFilter::You
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

fn parse_subject_clause_player_filter(words: &[&str]) -> PlayerFilter {
    if contains_your_team_words(words) || words.contains(&"you") {
        PlayerFilter::You
    } else if contains_opponent_word(words) {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    }
}

fn contains_opponent_word(words: &[&str]) -> bool {
    words.contains(&"opponent") || words.contains(&"opponents")
}

fn contains_your_team_words(words: &[&str]) -> bool {
    words.windows(2).any(|window| window == ["your", "team"])
        || words
            .windows(3)
            .any(|window| window == ["on", "your", "team"])
}

fn parse_trigger_subject_player_filter(subject: &[&str]) -> Option<PlayerFilter> {
    if subject == ["you"] {
        return Some(PlayerFilter::You);
    }
    if subject == ["a", "player"]
        || subject == ["any", "player"]
        || subject == ["player"]
        || subject == ["one", "or", "more", "players"]
    {
        return Some(PlayerFilter::Any);
    }
    if subject == ["an", "opponent"]
        || subject == ["opponent"]
        || subject == ["opponents"]
        || subject == ["one", "or", "more", "opponents"]
    {
        return Some(PlayerFilter::Opponent);
    }
    if subject.ends_with(&["on", "your", "team"])
        && subject
            .iter()
            .any(|word| matches!(*word, "player" | "players"))
    {
        return Some(PlayerFilter::You);
    }
    None
}

fn parse_trigger_subject_filter(
    subject_tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = subject_tokens;
    if subject_tokens.len() >= 3
        && subject_tokens
            .first()
            .is_some_and(|token| token.is_word("one"))
        && subject_tokens
            .get(1)
            .is_some_and(|token| token.is_word("or"))
        && subject_tokens
            .get(2)
            .is_some_and(|token| token.is_word("more"))
    {
        subject_tokens = &subject_tokens[3..];
    }
    let mut other = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("another") || token.is_word("other"))
    {
        other = true;
        subject_tokens = &subject_tokens[1..];
    }
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(subject_tokens);
    if is_source_reference_words(&subject_words) {
        return Ok(None);
    }
    if subject_words
        .iter()
        .any(|word| matches!(*word, "that" | "which" | "who" | "whom" | "and" | "or"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger subject filter (clause: '{}')",
            subject_words.join(" ")
        )));
    }

    parse_object_filter(subject_tokens, other)
        .map(Some)
        .map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported trigger subject filter (clause: '{}')",
                words(subject_tokens).join(" ")
            ))
        })
}

fn parse_spell_activity_trigger(tokens: &[Token]) -> Result<Option<TriggerSpec>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"spell") && !clause_words.contains(&"spells") {
        return Ok(None);
    }

    let cast_idx = tokens
        .iter()
        .position(|token| token.is_word("cast") || token.is_word("casts"));
    let copy_idx = tokens
        .iter()
        .position(|token| token.is_word("copy") || token.is_word("copies"));
    if cast_idx.is_none() && copy_idx.is_none() {
        return Ok(None);
    }

    let actor = parse_subject_clause_player_filter(&clause_words);
    let during_turn = if contains_word_sequence(&clause_words, &["during", "your", "turn"]) {
        Some(PlayerFilter::You)
    } else if contains_word_sequence(&clause_words, &["during", "an", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "opponents", "turn"])
        || contains_word_sequence(&clause_words, &["during", "each", "opponents", "turn"])
    {
        Some(PlayerFilter::Opponent)
    } else {
        None
    };
    let has_other_than_first_spell_pattern = contains_word_sequence(
        &clause_words,
        &["other", "than", "your", "first", "spell"],
    ) || contains_word_sequence(
        &clause_words,
        &["other", "than", "the", "first", "spell"],
    ) || (contains_word_sequence(&clause_words, &["other", "than", "the", "first"])
        && clause_words.contains(&"spell")
        && clause_words.contains(&"casts")
        && clause_words.contains(&"turn"));
    let min_spells_this_turn = if has_other_than_first_spell_pattern
        || contains_word_sequence(
        &clause_words,
        &["second", "spell", "cast", "this", "turn"],
    ) || contains_word_sequence(&clause_words, &["second", "spell", "this", "turn"])
        || contains_word_sequence(&clause_words, &["your", "second", "spell", "each", "turn"])
        || contains_word_sequence(&clause_words, &["their", "second", "spell", "each", "turn"])
        || contains_word_sequence(&clause_words, &["your", "second", "spell", "this", "turn"])
        || contains_word_sequence(&clause_words, &["their", "second", "spell", "this", "turn"])
        || contains_word_sequence(&clause_words, &["second", "spell", "each", "turn"])
    {
        Some(2)
    } else {
        None
    };
    let from_not_hand = contains_word_sequence(
        &clause_words,
        &["from", "anywhere", "other", "than", "your", "hand"],
    ) || contains_word_sequence(
        &clause_words,
        &["from", "anywhere", "other", "than", "hand"],
    );

    let parse_filter = |filter_tokens: &[Token]| -> Result<Option<ObjectFilter>, CardTextError> {
        let filter_tokens = if let Some(idx) = filter_tokens
            .iter()
            .position(|token| token.is_word("during") || token.is_word("other"))
        {
            &filter_tokens[..idx]
        } else {
            filter_tokens
        };
        let filter_tokens = if let Some(idx) = filter_tokens
            .iter()
            .position(|token| token.is_word("from"))
            .filter(|idx| {
                filter_tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("anywhere"))
            })
        {
            &filter_tokens[..idx]
        } else {
            filter_tokens
        };
        let filter_words: Vec<&str> = filter_tokens.iter().filter_map(Token::as_word).collect();
        let is_unqualified_spell = filter_words.as_slice() == ["a", "spell"]
            || filter_words.as_slice() == ["spells"]
            || filter_words.as_slice() == ["spell"];
        if filter_tokens.is_empty() || is_unqualified_spell {
            Ok(None)
        } else {
            let parse_spell_origin_zone_filter = || -> Option<ObjectFilter> {
                let zone = if filter_words.contains(&"graveyard") {
                    Some(Zone::Graveyard)
                } else if filter_words.contains(&"exile") {
                    Some(Zone::Exile)
                } else {
                    None
                }?;
                let mentions_spell = filter_words.contains(&"spell") || filter_words.contains(&"spells");
                if !mentions_spell {
                    return None;
                }
                let mut filter = ObjectFilter::spell().in_zone(zone);
                if filter_words.contains(&"your") {
                    filter.owner = Some(actor.clone());
                } else if filter_words.contains(&"opponent") || filter_words.contains(&"their") {
                    filter.owner = Some(PlayerFilter::Opponent);
                }
                Some(filter)
            };
            match parse_object_filter(filter_tokens, false) {
                Ok(filter) => Ok(Some(filter)),
                Err(err) => {
                    if let Some(origin_filter) = parse_spell_origin_zone_filter() {
                        Ok(Some(origin_filter))
                    } else {
                        Err(err)
                    }
                }
            }
        }
    };

    if let (Some(cast), Some(copy)) = (cast_idx, copy_idx) {
        let (first, second, first_is_cast) = if cast < copy {
            (cast, copy, true)
        } else {
            (copy, cast, false)
        };
        let between_words = words(&tokens[first + 1..second]);
        if between_words.as_slice() == ["or"] {
            let filter = parse_filter(tokens.get(second + 1..).unwrap_or_default())?;
            let cast_trigger = TriggerSpec::SpellCast {
                filter: filter.clone(),
                caster: actor.clone(),
                during_turn: during_turn.clone(),
                min_spells_this_turn,
                from_not_hand,
            };
            let copied_trigger = TriggerSpec::SpellCopied {
                filter,
                copier: actor,
            };
            return Ok(Some(if first_is_cast {
                TriggerSpec::Either(Box::new(cast_trigger), Box::new(copied_trigger))
            } else {
                TriggerSpec::Either(Box::new(copied_trigger), Box::new(cast_trigger))
            }));
        }
    }

    if let Some(cast) = cast_idx {
        let filter = parse_filter(tokens.get(cast + 1..).unwrap_or_default())?;
        return Ok(Some(TriggerSpec::SpellCast {
            filter,
            caster: actor,
            during_turn,
            min_spells_this_turn,
            from_not_hand,
        }));
    }

    if let Some(copy) = copy_idx {
        let filter = parse_filter(tokens.get(copy + 1..).unwrap_or_default())?;
        return Ok(Some(TriggerSpec::SpellCopied {
            filter,
            copier: actor,
        }));
    }

    Ok(None)
}

fn is_spawn_scion_token_mana_reminder(tokens: &[Token]) -> bool {
    let words = words(tokens);
    let starts_with_token_pronoun = words.starts_with(&["they", "have"])
        || words.starts_with(&["it", "has"])
        || words.starts_with(&["this", "token", "has"])
        || words.starts_with(&["those", "tokens", "have"]);
    starts_with_token_pronoun
        && words.contains(&"sacrifice")
        && words.contains(&"add")
        && words.contains(&"c")
}

fn is_round_up_each_time_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.starts_with(&["round", "up", "each", "time"])
}

fn token_name_mentions_eldrazi_spawn_or_scion(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.contains("eldrazi") && lower.contains("spawn"))
        || (lower.contains("eldrazi") && lower.contains("scion"))
}

fn effect_creates_eldrazi_spawn_or_scion(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateToken { name, .. } | EffectAst::CreateTokenWithMods { name, .. } => {
            token_name_mentions_eldrazi_spawn_or_scion(name)
        }
        _ => false,
    }
}

fn effect_creates_any_token(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::CreateToken { .. }
        | EffectAst::CreateTokenWithMods { .. }
        | EffectAst::CreateTokenCopy { .. }
        | EffectAst::CreateTokenCopyFromSource { .. } => true,
        EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponent { effects } => effects.iter().any(effect_creates_any_token),
        _ => false,
    }
}

fn is_generic_token_reminder_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    if words.starts_with(&["it", "has"]) || words.starts_with(&["they", "have"]) {
        return true;
    }
    words.starts_with(&["when", "this", "token"])
        || words.starts_with(&["whenever", "this", "token"])
        || words.starts_with(&["this", "token"])
        || words.starts_with(&["those", "tokens"])
}

fn strip_embedded_token_rules_text(tokens: &[Token]) -> Vec<Token> {
    let words_all = words(tokens);
    if !words_all.contains(&"create") || !words_all.contains(&"token") {
        return tokens.to_vec();
    }
    let Some(with_idx) = tokens.iter().position(|token| token.is_word("with")) else {
        return tokens.to_vec();
    };
    let next_word = tokens.get(with_idx + 1).and_then(Token::as_word);
    if matches!(next_word, Some("t")) {
        return tokens[..with_idx].to_vec();
    }
    tokens.to_vec()
}

fn append_token_reminder_to_last_create_effect(
    effects: &mut Vec<EffectAst>,
    tokens: &[Token],
) -> bool {
    let mut reminder_words = words(tokens);
    if reminder_words.starts_with(&["it", "has"]) || reminder_words.starts_with(&["they", "have"]) {
        reminder_words = reminder_words[2..].to_vec();
    }
    if reminder_words.is_empty() {
        return false;
    }
    let reminder = reminder_words.join(" ");
    append_token_reminder_to_effect(effects.last_mut(), &reminder)
}

fn append_token_reminder_to_effect(effect: Option<&mut EffectAst>, reminder: &str) -> bool {
    let Some(effect) = effect else {
        return false;
    };
    match effect {
        EffectAst::CreateToken { name, .. } | EffectAst::CreateTokenWithMods { name, .. } => {
            if !name.ends_with(' ') {
                name.push(' ');
            }
            name.push_str(reminder);
            true
        }
        EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachOpponent { effects } => {
            append_token_reminder_to_effect(effects.last_mut(), reminder)
        }
        _ => false,
    }
}

fn parse_target_player_choose_objects_clause(
    tokens: &[Token],
) -> Result<Option<(PlayerAst, ObjectFilter, ChoiceCount)>, CardTextError> {
    let clause_words = words(tokens);
    let (chooser, choose_start_idx) =
        if clause_words.first().copied() == Some("target") && clause_words.len() >= 4 {
            let chooser = match clause_words.get(1).copied() {
                Some("player") => PlayerAst::Target,
                Some("opponent") | Some("opponents") => PlayerAst::TargetOpponent,
                _ => return Ok(None),
            };
            if !matches!(
                clause_words.get(2).copied(),
                Some("choose") | Some("chooses")
            ) {
                return Ok(None);
            }
            (chooser, 3usize)
        } else if clause_words.len() >= 4
            && clause_words.first().copied() == Some("that")
            && matches!(clause_words.get(1).copied(), Some("player" | "players"))
            && matches!(clause_words.get(2).copied(), Some("choose" | "chooses"))
        {
            (PlayerAst::That, 3usize)
        } else {
            return Ok(None);
        };

    let mut choose_object_tokens = trim_commas(&tokens[choose_start_idx..]);
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut count = ChoiceCount::exactly(1);
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("up"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("to"))
        && let Some((value, used)) = parse_number(&choose_object_tokens[2..])
    {
        count = ChoiceCount {
            min: 0,
            max: Some(value as usize),
            dynamic_x: false,
        };
        choose_object_tokens = trim_commas(&choose_object_tokens[2 + used..]);
    } else if let Some((value, used)) = parse_number(&choose_object_tokens) {
        count = ChoiceCount::exactly(value as usize);
        choose_object_tokens = trim_commas(&choose_object_tokens[used..]);
    }
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object filter after count in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if choose_object_tokens
        .first()
        .is_some_and(|token| token.is_word("target"))
        && choose_object_tokens
            .get(1)
            .is_some_and(|token| token.is_word("player") || token.is_word("opponent"))
    {
        return Ok(None);
    }
    if find_verb(&choose_object_tokens).is_some() {
        return Ok(None);
    }

    let mut choose_filter = parse_object_filter(&choose_object_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if matches!(
        choose_filter.zone,
        Some(Zone::Graveyard | Zone::Hand | Zone::Library | Zone::Exile)
    ) {
        choose_filter.controller = None;
    }
    if choose_filter.controller.is_none() && choose_filter.owner.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some((chooser, choose_filter, count)))
}

fn parse_target_player_chooses_then_other_cant_block(
    first: &[Token],
    second: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(first)?
    else {
        return Ok(None);
    };
    if choose_filter.card_types.is_empty() {
        choose_filter.card_types.push(CardType::Creature);
    }

    let second_words = words(second);
    let Some((neg_start, neg_end)) = find_negation_span(second) else {
        return Ok(None);
    };
    let tail_words = normalize_cant_words(&second[neg_end..]);
    if !matches!(tail_words.as_slice(), ["block", "this", "turn"] | ["block"]) {
        return Ok(None);
    }

    let mut subject_tokens = trim_commas(&second[..neg_start]);
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing subject in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut exclude_tagged_choice = false;
    if subject_tokens
        .first()
        .is_some_and(|token| token.is_word("other") || token.is_word("another"))
    {
        exclude_tagged_choice = true;
        subject_tokens = trim_commas(&subject_tokens[1..]);
    }
    if subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in cant-block clause (clause: '{}')",
            second_words.join(" ")
        )));
    }

    let mut restriction_filter = parse_object_filter(&subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported cant-block subject filter (clause: '{}')",
            second_words.join(" ")
        ))
    })?;
    if restriction_filter.card_types.is_empty() {
        restriction_filter.card_types.push(CardType::Creature);
    }
    if restriction_filter.controller.is_none() {
        restriction_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }
    if exclude_tagged_choice
        && !restriction_filter
            .tagged_constraints
            .iter()
            .any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
            })
    {
        restriction_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
    }

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::block(restriction_filter),
            duration: Until::EndOfTurn,
        },
    ]))
}

fn parse_sentence_target_player_chooses_then_puts_on_top_of_library(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) else {
        return Ok(None);
    };
    let first_clause = trim_commas(&tokens[..and_idx]);
    let second_clause = trim_commas(&tokens[and_idx + 1..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = words(&second_clause);
    if !matches!(second_words.first().copied(), Some("put" | "puts")) {
        return Ok(None);
    }
    let Some(on_idx) = second_clause.iter().position(|token| token.is_word("on")) else {
        return Ok(None);
    };
    if !second_clause
        .get(on_idx + 1)
        .is_some_and(|token| token.is_word("top"))
        || !second_clause
            .get(on_idx + 2)
            .is_some_and(|token| token.is_word("of"))
    {
        return Ok(None);
    }
    let destination_words = words(&second_clause[on_idx + 3..]);
    if !destination_words.contains(&"library") {
        return Ok(None);
    }

    let moved_tokens = trim_commas(&second_clause[1..on_idx]);
    let moved_words = words(&moved_tokens);
    let target = if moved_tokens.is_empty()
        || moved_words.as_slice() == ["it"]
        || moved_words.as_slice() == ["them"]
        || moved_words.as_slice() == ["those"]
        || moved_words.as_slice() == ["those", "cards"]
    {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause))
    } else {
        parse_target_phrase(&moved_tokens)?
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
        },
    ]))
}

fn parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            tokens
                .iter()
                .position(|token| token.is_word("then"))
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let first_clause = trim_commas(&tokens[..head_end]);
    let second_clause = trim_commas(&tokens[tail_start..]);
    if second_clause.is_empty() {
        return Ok(None);
    }

    let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(&first_clause)?
    else {
        return Ok(None);
    };

    let second_words = words(&second_clause);
    if second_words.len() < 4
        || second_words[0] != "you"
        || !matches!(second_words[1], "put" | "puts")
    {
        return Ok(None);
    }

    let Some(onto_idx) = second_clause.iter().position(|token| token.is_word("onto")) else {
        return Ok(None);
    };
    if onto_idx < 2 {
        return Ok(None);
    }

    let moved_words = words(&second_clause[2..onto_idx]);
    let moved_is_tagged_choice = moved_words == ["it"]
        || moved_words == ["that", "card"]
        || moved_words == ["that", "permanent"];
    if !moved_is_tagged_choice {
        return Ok(None);
    }

    let destination_words: Vec<&str> = words(&second_clause[onto_idx + 1..])
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if destination_words.first() != Some(&"battlefield") {
        return Ok(None);
    }
    let destination_tail = &destination_words[1..];
    let battlefield_controller = if destination_tail == ["under", "your", "control"] {
        ReturnControllerAst::You
    } else if destination_tail.is_empty() {
        ReturnControllerAst::Preserve
    } else if destination_tail == ["under", "its", "owners", "control"]
        || destination_tail == ["under", "their", "owners", "control"]
        || destination_tail == ["under", "that", "players", "control"]
    {
        ReturnControllerAst::Owner
    } else {
        return Ok(None);
    };

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::MoveToZone {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&second_clause)),
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
        },
    ]))
}

fn parse_effect_sentences(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::new();
    let sentences = split_on_period(tokens);
    let mut sentence_idx = 0usize;
    let mut carried_context: Option<CarryContext> = None;

    while sentence_idx < sentences.len() {
        let sentence = &sentences[sentence_idx];
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }

        if sentence_idx + 1 < sentences.len()
            && let Some(mut combined) = parse_target_player_chooses_then_other_cant_block(
                sentence,
                &sentences[sentence_idx + 1],
            )?
        {
            parser_trace(
                "parse_effect_sentences:sequence-hit:target-chooses-other-cant-block",
                sentence,
            );
            effects.append(&mut combined);
            sentence_idx += 2;
            continue;
        }
        let mut sentence_tokens = strip_embedded_token_rules_text(sentence);
        if sentence_tokens.is_empty() {
            sentence_idx += 1;
            continue;
        }
        sentence_tokens = rewrite_when_you_do_clause_prefix(&sentence_tokens);
        let mut wraps_as_if_did_not = false;
        if let Some(without_otherwise) = strip_otherwise_sentence_prefix(&sentence_tokens) {
            sentence_tokens = rewrite_otherwise_referential_subject(without_otherwise);
            wraps_as_if_did_not = true;
        }
        parser_trace("parse_effect_sentences:sentence", &sentence_tokens);

        if is_spawn_scion_token_mana_reminder(&sentence_tokens) {
            if effects
                .last()
                .is_some_and(effect_creates_eldrazi_spawn_or_scion)
            {
                parser_trace("parse_effect_sentences:spawn-scion-reminder", &sentence_tokens);
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token mana reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        if is_generic_token_reminder_sentence(&sentence_tokens)
            && effects.last().is_some_and(effect_creates_any_token)
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }

        let mut sentence_effects = parse_effect_sentence(&sentence_tokens)?;
        if wraps_as_if_did_not {
            sentence_effects = vec![EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        collapse_token_copy_next_end_step_exile_followup(&mut sentence_effects, &sentence_tokens);
        if is_that_turn_end_step_sentence(&sentence_tokens)
            && let Some(extra_turn_player) = most_recent_extra_turn_player(&effects)
            && !sentence_effects.is_empty()
        {
            sentence_effects = vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: extra_turn_player,
                effects: sentence_effects,
            }];
        }
        if words(&sentence_tokens).first().copied() == Some("you") {
            carried_context = None;
        }
        if try_apply_token_copy_followup(&mut effects, &sentence_effects)? {
            parser_trace("parse_effect_sentences:token-copy-followup", &sentence_tokens);
            sentence_idx += 1;
            continue;
        }
        if sentence_effects.is_empty()
            && !is_round_up_each_time_sentence(&sentence_tokens)
            && !is_nonsemantic_restriction_sentence(&sentence_tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        // If a token-copy modifier sentinel didn't apply (no preceding CreateTokenCopy),
        // convert it to a proper effect on the tagged "it" object.
        for effect in &mut sentence_effects {
            if matches!(effect, EffectAst::TokenCopyGainHasteUntilEot) {
                let span = span_from_tokens(&sentence);
                *effect = EffectAst::GrantAbilitiesToTarget {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
                    abilities: vec![StaticAbility::haste()],
                    duration: Until::EndOfTurn,
                };
            } else if matches!(effect, EffectAst::TokenCopySacrificeAtNextEndStep) {
                *effect = EffectAst::DelayedUntilNextEndStep {
                    player: PlayerFilter::Any,
                    effects: vec![EffectAst::Sacrifice {
                        filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                        player: PlayerAst::Implicit,
                        count: 1,
                    }],
                };
            } else if matches!(effect, EffectAst::TokenCopyExileAtNextEndStep) {
                let span = span_from_tokens(&sentence);
                *effect = EffectAst::DelayedUntilNextEndStep {
                    player: PlayerFilter::Any,
                    effects: vec![EffectAst::Exile {
                        target: TargetAst::Object(
                            ObjectFilter::tagged(TagKey::from(IT_TAG)),
                            span,
                            None,
                        ),
                    }],
                };
            }
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                maybe_apply_carried_player(effect, context);
            }
            if let Some(context) = explicit_player_for_carry(effect) {
                carried_context = Some(context);
            }
        }
        if sentence_effects.len() == 1
            && let Some(previous_effect) = effects.last()
            && let Some(EffectAst::IfResult {
                predicate,
                effects: if_result_effects,
            }) = sentence_effects.first_mut()
        {
            if matches!(predicate, IfResultPredicate::Did)
                && matches!(previous_effect, EffectAst::UnlessPays { .. })
            {
                *predicate = IfResultPredicate::DidNot;
            }
            if let Some(previous_target) = primary_damage_target_from_effect(previous_effect) {
                replace_it_damage_target_in_effects(if_result_effects, &previous_target);
            }
        }
        let has_instead = sentence.iter().any(|token| token.is_word("instead"));
        if has_instead && sentence_effects.len() == 1 && effects.len() >= 1 {
            if matches!(
                sentence_effects.first(),
                Some(EffectAst::Conditional { .. })
            ) {
                let previous = effects.pop().expect("effects length checked above");
                let previous_target = match &previous {
                    EffectAst::DealDamage { target, .. } => Some(target.clone()),
                    _ => None,
                };
                if let Some(EffectAst::Conditional {
                    predicate,
                    mut if_true,
                    mut if_false,
                }) = sentence_effects.pop()
                {
                    if let Some(target) = previous_target {
                        replace_it_damage_target_in_effects(&mut if_true, &target);
                    }
                    if_false.insert(0, previous);
                    effects.push(EffectAst::Conditional {
                        predicate,
                        if_true,
                        if_false,
                    });
                    sentence_idx += 1;
                    continue;
                }
            }
        }

        effects.extend(sentence_effects);
        sentence_idx += 1;
    }

    parser_trace("parse_effect_sentences:done", tokens);
    Ok(effects)
}

fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. } | EffectAst::DealDamageEqualToPower { target, .. } => {
            Some(target.clone())
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => if_true
            .iter()
            .find_map(primary_damage_target_from_effect)
            .or_else(|| if_false.iter().find_map(primary_damage_target_from_effect)),
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::VoteOption { effects, .. }
        | EffectAst::UnlessAction {
            effects,
            alternative: _,
            ..
        } => effects.iter().find_map(primary_damage_target_from_effect),
        _ => None,
    }
}

fn replace_it_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_damage_target(effect, target);
    }
}

fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        } => {
            if target_references_it(damage_target) {
                *damage_target = target.clone();
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            replace_it_damage_target_in_effects(if_true, target);
            replace_it_damage_target_in_effects(if_false, target);
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::VoteOption { effects, .. } => {
            replace_it_damage_target_in_effects(effects, target);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            replace_it_damage_target_in_effects(effects, target);
            replace_it_damage_target_in_effects(alternative, target);
        }
        _ => {}
    }
}

fn target_references_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        TargetAst::WithCount(inner, _) => target_references_it(inner),
        _ => false,
    }
}

fn is_that_turn_end_step_sentence(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    clause_words.starts_with(&["at", "the", "beginning", "of", "that", "turn", "end", "step"])
        || clause_words
            .starts_with(&["at", "the", "beginning", "of", "that", "turns", "end", "step"])
}

fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        if let EffectAst::ExtraTurnAfterTurn { player } = effect {
            Some(*player)
        } else {
            None
        }
    })
}

fn rewrite_when_you_do_clause_prefix(tokens: &[Token]) -> Vec<Token> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["when", "you", "do"]) {
        return tokens.to_vec();
    }
    let mut rewritten = tokens.to_vec();
    for token in &mut rewritten {
        if let Token::Word(word, _) = token {
            if word.eq_ignore_ascii_case("when") {
                *word = "if".to_string();
            }
            break;
        }
    }
    rewritten
}

fn strip_otherwise_sentence_prefix(tokens: &[Token]) -> Option<Vec<Token>> {
    if !tokens.first().is_some_and(|token| token.is_word("otherwise")) {
        return None;
    }

    let mut idx = 1usize;
    while matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("then")) {
        idx += 1;
    }
    while matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }

    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    }
}

fn rewrite_otherwise_referential_subject(tokens: Vec<Token>) -> Vec<Token> {
    let clause_words = words(&tokens);
    let is_referential_get = clause_words.len() >= 3
        && clause_words[0] == "that"
        && matches!(clause_words[1], "creature" | "permanent")
        && matches!(clause_words[2], "gets" | "get" | "gains" | "gain");
    if !is_referential_get {
        return tokens;
    }

    let mut rewritten = tokens;
    if let Some(first) = rewritten.get_mut(0)
        && let Token::Word(word, _) = first
    {
        *word = "target".to_string();
    }
    rewritten
}

fn is_nonsemantic_restriction_sentence(tokens: &[Token]) -> bool {
    is_activate_only_restriction_sentence(tokens) || is_trigger_only_restriction_sentence(tokens)
}

fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    sentence_effects: &[EffectAst],
) -> Result<bool, CardTextError> {
    if sentence_effects.len() != 1 {
        return Ok(false);
    }

    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    let Some((haste, sacrifice, exile)) = (match sentence_effects.first() {
        Some(EffectAst::TokenCopyGainHasteUntilEot) => Some((true, false, false)),
        Some(EffectAst::TokenCopySacrificeAtNextEndStep) => Some((false, true, false)),
        Some(EffectAst::TokenCopyExileAtNextEndStep) => Some((false, false, true)),
        _ => None,
    }) else {
        return Ok(false);
    };

    match last {
        EffectAst::CreateTokenCopy {
            has_haste,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            if haste {
                *has_haste = true;
            }
            if sacrifice {
                *sacrifice_at_next_end_step = true;
            }
            if exile {
                *exile_at_next_end_step = true;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

type SentencePrimitiveParser = fn(&[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError>;

struct SentencePrimitive {
    name: &'static str,
    parser: SentencePrimitiveParser,
}

fn run_sentence_primitives(
    tokens: &[Token],
    primitives: &[SentencePrimitive],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    for primitive in primitives {
        match (primitive.parser)(tokens) {
            Ok(Some(effects)) => {
                let stage = format!("parse_effect_sentence:primitive-hit:{}", primitive.name);
                parser_trace(&stage, tokens);
                if effects.is_empty() {
                    return Err(CardTextError::ParseError(format!(
                        "primitive '{}' produced empty effects (clause: '{}')",
                        primitive.name,
                        words(tokens).join(" ")
                    )));
                }
                return Ok(Some(effects));
            }
            Ok(None) => {}
            Err(err) => {
                if parser_trace_enabled() {
                    eprintln!(
                        "[parser-flow] stage=parse_effect_sentence:primitive-error primitive={} clause='{}' error={err:?}",
                        primitive.name,
                        words(tokens).join(" ")
                    );
                }
                return Err(err);
            }
        }
    }
    Ok(None)
}

fn parse_you_and_target_player_each_draw_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if !clause_words.starts_with(&["you", "and", "target"]) {
        return Ok(None);
    }

    let target_player = match clause_words.get(3).copied() {
        Some("opponent" | "opponents") => PlayerAst::TargetOpponent,
        Some("player" | "players") => PlayerAst::Target,
        _ => return Ok(None),
    };

    let mut idx = 4usize;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let remainder_words = &clause_words[idx..];
    if remainder_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let synthetic_tokens = remainder_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (count, used) = parse_value(&synthetic_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing draw count in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if synthetic_tokens
        .get(used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in shared draw sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words = words(&synthetic_tokens[used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count,
            player: target_player,
        },
    ]))
}

fn parse_sentence_you_and_target_player_each_draw(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_target_player_each_draw_sentence(tokens)
}

fn parse_sentence_you_and_attacking_player_each_draw_and_lose(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 11 || !clause_words.starts_with(&["you", "and"]) {
        return Ok(None);
    }

    let mut idx = 2usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"attacking")
        || clause_words.get(idx + 1) != Some(&"player")
    {
        return Ok(None);
    }
    idx += 2;

    if clause_words.get(idx) == Some(&"each") {
        idx += 1;
    }
    if !matches!(clause_words.get(idx).copied(), Some("draw" | "draws")) {
        return Ok(None);
    }
    idx += 1;

    let draw_tokens = clause_words[idx..]
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (draw_count, draw_used) = parse_value(&draw_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared draw count (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if draw_tokens
        .get(draw_used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(format!(
            "missing card keyword in shared draw/lose sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let after_draw_words = words(&draw_tokens[draw_used + 1..]);
    if after_draw_words.first() != Some(&"and")
        || !matches!(after_draw_words.get(1).copied(), Some("lose" | "loses"))
    {
        return Ok(None);
    }

    let lose_tokens = after_draw_words[2..]
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let (lose_amount, lose_used) = parse_value(&lose_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing shared life-loss amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if lose_tokens
        .get(lose_used)
        .and_then(Token::as_word)
        .is_none_or(|word| word != "life")
    {
        return Err(CardTextError::ParseError(format!(
            "missing life keyword in shared draw/lose sentence (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let trailing_words = words(&lose_tokens[lose_used + 1..]);
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing shared draw/lose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![
        EffectAst::Draw {
            count: draw_count.clone(),
            player: PlayerAst::You,
        },
        EffectAst::Draw {
            count: draw_count,
            player: PlayerAst::Attacking,
        },
        EffectAst::LoseLife {
            amount: lose_amount.clone(),
            player: PlayerAst::You,
        },
        EffectAst::LoseLife {
            amount: lose_amount,
            player: PlayerAst::Attacking,
        },
    ]))
}

fn parse_sentence_token_copy_modifier(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let effect = parse_token_copy_modifier_sentence(tokens);
    if effect.is_some() && tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "token copy modifier sentence missing tokens".to_string(),
        ));
    }
    Ok(effect.map(|effect| vec![effect]))
}

fn parse_sentence_sacrifice_it_next_end_step(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("sacrifice")) {
        return Ok(None);
    }

    let Some(at_idx) = tokens.iter().position(|token| token.is_word("at")) else {
        return Ok(None);
    };
    if at_idx <= 1 {
        return Ok(None);
    }

    let timing_words = words(&tokens[at_idx..]);
    let matches_sacrifice_delay = timing_words.as_slice()
        == ["at", "the", "beginning", "of", "the", "next", "end", "step"]
        || timing_words.as_slice() == ["at", "the", "beginning", "of", "next", "end", "step"];
    if !matches_sacrifice_delay {
        return Ok(None);
    }

    let object_tokens = trim_commas(&tokens[1..at_idx]);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in delayed next-end-step clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let object_words = words(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["the", "creature"]
            | ["that", "creature"]
            | ["the", "permanent"]
            | ["that", "permanent"]
            | ["the", "token"]
            | ["that", "token"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player: PlayerFilter::Any,
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
        }],
    }]))
}

fn parse_sentence_sacrifice_at_end_of_combat(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("sacrifice")) {
        return Ok(None);
    }
    let Some(at_idx) = tokens.iter().position(|token| token.is_word("at")) else {
        return Ok(None);
    };
    if at_idx <= 1 {
        return Ok(None);
    }

    let timing_words = words(&tokens[at_idx..]);
    let matches_end_of_combat = timing_words.as_slice() == ["at", "end", "of", "combat"]
        || timing_words.as_slice() == ["at", "the", "end", "of", "combat"];
    if !matches_end_of_combat {
        return Ok(None);
    }

    let object_tokens = trim_commas(&tokens[1..at_idx]);
    if object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object in end-of-combat clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let object_words = words(&object_tokens);
    let filter = if matches!(
        object_words.as_slice(),
        ["it"]
            | ["them"]
            | ["that", "token"]
            | ["this", "token"]
            | ["that", "permanent"]
            | ["this", "permanent"]
    ) {
        ObjectFilter::tagged(TagKey::from(IT_TAG))
    } else {
        parse_object_filter(&object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::Sacrifice {
            filter,
            player: PlayerAst::Implicit,
            count: 1,
        }],
    }]))
}

fn parse_sentence_each_player_choose_and_sacrifice_rest(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_each_player_choose_and_sacrifice_rest(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_exile_instead_of_graveyard(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_exile_instead_of_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_monstrosity(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_monstrosity_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_for_each_counter_removed(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_counter_removed_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_put_counter_ladder_segments(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let segments = split_on_comma(tokens);
    if segments.len() != 3 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = trim_commas(segment).to_vec();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }
        if clause.is_empty() {
            return Ok(None);
        }

        let Some(on_idx) = clause.iter().position(|token| token.is_word("on")) else {
            return Ok(None);
        };
        let descriptor = trim_commas(&clause[..on_idx]);
        let target_tokens = trim_commas(&clause[on_idx + 1..]);
        if descriptor.is_empty() || target_tokens.is_empty() {
            return Ok(None);
        }

        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        let target = parse_target_phrase(&target_tokens)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target,
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

fn parse_sentence_put_counter_sequence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("put")) {
        return Ok(None);
    }

    if let Some(effects) = parse_put_counter_ladder_segments(tokens)? {
        return Ok(Some(effects));
    }

    if let Some(on_idx) = tokens.iter().position(|token| token.is_word("on")) {
        let descriptor_tokens = trim_commas(&tokens[1..on_idx]);
        let target_tokens = trim_commas(&tokens[on_idx + 1..]);
        if !descriptor_tokens.is_empty() && !target_tokens.is_empty() {
            let mut descriptors: Vec<Vec<Token>> = Vec::new();
            let comma_segments = split_on_comma(&descriptor_tokens);
            if comma_segments.len() >= 2 {
                for segment in comma_segments {
                    let mut clause = trim_commas(&segment);
                    if clause.first().is_some_and(|token| token.is_word("and")) {
                        clause.remove(0);
                    }
                    if clause.is_empty() {
                        descriptors.clear();
                        break;
                    }
                    descriptors.push(clause);
                }
            } else if let Some(and_idx) = descriptor_tokens.iter().position(|token| token.is_word("and"))
            {
                let first = trim_commas(&descriptor_tokens[..and_idx]);
                let second = trim_commas(&descriptor_tokens[and_idx + 1..]);
                if !first.is_empty() && !second.is_empty() {
                    descriptors.push(first);
                    descriptors.push(second);
                }
            }

            if descriptors.len() >= 2 {
                let target = parse_target_phrase(&target_tokens)?;
                let mut effects = Vec::new();
                for descriptor in descriptors {
                    let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
                    effects.push(EffectAst::PutCounters {
                        counter_type,
                        count: Value::Fixed(count as i32),
                        target: target.clone(),
                        target_count: None,
                        distributed: false,
                    });
                }
                return Ok(Some(effects));
            }
        }
    }

    // Handle "put ... and ... counter on ..." without comma separation.
    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) {
        let first_clause = trim_commas(&tokens[1..and_idx]);
        let second_clause = trim_commas(&tokens[and_idx + 1..]);
        if !first_clause.is_empty() && !second_clause.is_empty() {
            if let (Ok(first), Ok(second)) = (
                parse_put_counters(&first_clause),
                parse_put_counters(&second_clause),
            ) {
                return Ok(Some(vec![first, second]));
            }
        }
    }

    let segments = split_on_comma(tokens);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for (idx, segment) in segments.iter().enumerate() {
        let mut clause = segment.clone();
        if idx == 0 {
            if clause.is_empty() || !clause[0].is_word("put") {
                return Ok(None);
            }
            clause.remove(0);
        } else if clause.first().is_some_and(|token| token.is_word("and")) {
            clause.remove(0);
        }

        if clause.is_empty() {
            return Ok(None);
        }

        let clause_words = words(&clause);
        if !clause_words.contains(&"counter") && !clause_words.contains(&"counters") {
            return Ok(None);
        }

        let Ok(effect) = parse_put_counters(&clause) else {
            return Ok(None);
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        Ok(Some(effects))
    } else {
        Ok(None)
    }
}

fn parse_return_with_counters_on_it_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }

    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before destination (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let destination_tokens = trim_commas(&tokens[to_idx + 1..]);
    if destination_tokens.is_empty() {
        return Ok(None);
    }
    if !words(&destination_tokens).contains(&"battlefield") {
        return Ok(None);
    }

    let Some(with_idx) = destination_tokens
        .iter()
        .position(|token| token.is_word("with"))
    else {
        return Ok(None);
    };
    if with_idx + 1 >= destination_tokens.len() {
        return Ok(None);
    }

    let base_destination_words = words(&destination_tokens[..with_idx]);
    if !base_destination_words.contains(&"battlefield") {
        return Ok(None);
    }
    let tapped = base_destination_words.contains(&"tapped");

    let counter_clause_tokens = trim_commas(&destination_tokens[with_idx + 1..]);
    let Some(on_idx) = counter_clause_tokens
        .iter()
        .rposition(|token| token.is_word("on"))
    else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words = words(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    if descriptor_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut descriptors = Vec::new();
    for descriptor in split_on_and(&descriptor_tokens) {
        let descriptor = trim_commas(&descriptor);
        if descriptor.is_empty() {
            continue;
        }
        descriptors.push(descriptor);
    }
    if descriptors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing counter descriptor in return-with-counters clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut effects = vec![EffectAst::ReturnToBattlefield {
        target: parse_target_phrase(&target_tokens)?,
        tapped,
        controller: ReturnControllerAst::Preserve,
    }];
    let tagged_target = TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens));
    for descriptor in descriptors {
        let (count, counter_type) = parse_counter_descriptor(&descriptor)?;
        effects.push(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target: tagged_target.clone(),
            target_count: None,
            distributed: false,
        });
    }

    Ok(Some(effects))
}

fn parse_sentence_return_with_counters_on_it(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(tokens)
}

fn replace_target_subtype(target: &mut TargetAst, subtype: Subtype) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.subtypes = vec![subtype];
            true
        }
        TargetAst::WithCount(inner, _) => replace_target_subtype(inner, subtype),
        _ => false,
    }
}

fn clone_return_effect_with_subtype(base: &EffectAst, subtype: Subtype) -> Option<EffectAst> {
    match base {
        EffectAst::ReturnToHand { target } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(EffectAst::ReturnToHand {
                target: cloned_target,
            })
        }
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            controller,
        } => {
            let mut cloned_target = target.clone();
            replace_target_subtype(&mut cloned_target, subtype).then_some(
                EffectAst::ReturnToBattlefield {
                    target: cloned_target,
                    tapped: *tapped,
                    controller: *controller,
                },
            )
        }
        EffectAst::ReturnAllToHand { filter } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToHand {
                filter: cloned_filter,
            })
        }
        EffectAst::ReturnAllToBattlefield { filter, tapped } => {
            let mut cloned_filter = filter.clone();
            cloned_filter.subtypes = vec![subtype];
            Some(EffectAst::ReturnAllToBattlefield {
                filter: cloned_filter,
                tapped: *tapped,
            })
        }
        _ => None,
    }
}

fn parse_draw_then_connive_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = tokens.windows(2).position(|window| {
        matches!(window[0], Token::Comma(_)) && window[1].is_word("then")
    }) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    if !tail_tokens
        .iter()
        .any(|token| token.is_word("connive") || token.is_word("connives"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let Some(connive_effect) = parse_connive_clause(&tail_tokens)? else {
        return Ok(None);
    };
    head_effects.push(connive_effect);
    Ok(Some(head_effects))
}

fn parse_sentence_draw_then_connive(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_draw_then_connive_sentence(tokens)
}

fn parse_each_player_return_with_additional_counter_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let inner_start_word_idx =
        if clause_words.starts_with(&["for", "each", "player"])
            || clause_words.starts_with(&["for", "each", "players"])
        {
            3
        } else if clause_words.starts_with(&["each", "player"])
            || clause_words.starts_with(&["each", "players"])
        {
            2
        } else {
            return Ok(None);
        };

    let Some(inner_start_token_idx) = token_index_for_word_index(tokens, inner_start_word_idx)
    else {
        return Ok(None);
    };
    let inner_tokens = trim_commas(&tokens[inner_start_token_idx..]);
    if inner_tokens.is_empty() {
        return Ok(None);
    }
    if !inner_tokens
        .first()
        .is_some_and(|token| token.is_word("return") || token.is_word("returns"))
    {
        return Ok(None);
    }

    let Some(with_idx) = inner_tokens.iter().rposition(|token| token.is_word("with")) else {
        return Ok(None);
    };
    if with_idx + 1 >= inner_tokens.len() {
        return Ok(None);
    }

    let return_clause_tokens = trim_commas(&inner_tokens[..with_idx]);
    if return_clause_tokens.is_empty() {
        return Ok(None);
    }

    let counter_clause_tokens = trim_commas(&inner_tokens[with_idx + 1..]);
    let Some(on_idx) = counter_clause_tokens
        .iter()
        .rposition(|token| token.is_word("on"))
    else {
        return Ok(None);
    };
    if on_idx + 1 >= counter_clause_tokens.len() {
        return Ok(None);
    }

    let on_target_words = words(&counter_clause_tokens[on_idx + 1..]);
    if on_target_words != ["it"] && on_target_words != ["them"] {
        return Ok(None);
    }

    let descriptor_tokens = trim_commas(&counter_clause_tokens[..on_idx]);
    let descriptor_words = words(&descriptor_tokens);
    if descriptor_tokens.is_empty() || !descriptor_words.contains(&"additional") {
        return Ok(None);
    }

    let (count, counter_type) = parse_counter_descriptor(&descriptor_tokens)?;
    let mut per_player_effects = parse_effect_chain_inner(&return_clause_tokens)?;
    if per_player_effects.is_empty() {
        return Ok(None);
    }
    if !per_player_effects.iter().any(|effect| {
        matches!(
            effect,
            EffectAst::ReturnToBattlefield { .. } | EffectAst::ReturnAllToBattlefield { .. }
        )
    }) {
        return Ok(None);
    }

    per_player_effects.push(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(tokens)),
        target_count: None,
        distributed: false,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: per_player_effects,
    }]))
}

fn parse_sentence_each_player_return_with_additional_counter(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_return_with_additional_counter_sentence(tokens)
}

fn parse_return_then_do_same_for_subtypes_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(comma_then_idx) = tokens.windows(2).position(|window| {
        matches!(window[0], Token::Comma(_)) && window[1].is_word("then")
    }) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&["do", "the", "same", "for"]) {
        return Ok(None);
    }
    let subtype_words = &tail_words[4..];
    if subtype_words.is_empty() {
        return Ok(None);
    }

    let mut extra_subtypes = Vec::new();
    for word in subtype_words {
        if matches!(*word, "and" | "or") {
            continue;
        }
        let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
        else {
            return Ok(None);
        };
        extra_subtypes.push(subtype);
    }
    if extra_subtypes.is_empty() {
        return Ok(None);
    }

    let mut effects = parse_effect_chain(&head_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let base_effect = effects[0].clone();
    for subtype in extra_subtypes {
        let Some(cloned) = clone_return_effect_with_subtype(&base_effect, subtype) else {
            return Ok(None);
        };
        effects.push(cloned);
    }

    Ok(Some(effects))
}

fn parse_sentence_return_then_do_same_for_subtypes(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_then_do_same_for_subtypes_sentence(tokens)
}

fn parse_sacrifice_any_number_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (head_tokens, tail_tokens) = if let Some(then_idx) =
        tokens.iter().position(|token| token.is_word("then"))
    {
        if then_idx == 0 {
            return Ok(None);
        }
        (
            trim_commas(&tokens[..then_idx]),
            Some(trim_commas(&tokens[then_idx + 1..])),
        )
    } else {
        (tokens.to_vec(), None)
    };

    if !head_tokens
        .first()
        .is_some_and(|token| token.is_word("sacrifice"))
    {
        return Ok(None);
    }

    let mut idx = 1usize;
    if !(head_tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && head_tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number")))
    {
        return Ok(None);
    }
    idx += 2;
    if head_tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }
    if idx >= head_tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let filter_tokens = trim_commas(&head_tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object after 'sacrifice any number of' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let tag = TagKey::from(IT_TAG);

    let mut effects = vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::any_number(),
            player: PlayerAst::Implicit,
            tag: tag.clone(),
        },
        EffectAst::SacrificeAll {
            filter: ObjectFilter::tagged(tag),
            player: PlayerAst::Implicit,
        },
    ];
    if let Some(tail_tokens) = tail_tokens
        && !tail_tokens.is_empty()
    {
        let mut tail_effects = parse_effect_chain(&tail_tokens)?;
        effects.append(&mut tail_effects);
    }

    Ok(Some(effects))
}

fn parse_sentence_sacrifice_any_number(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_sacrifice_any_number_sentence(tokens)
}

fn parse_sentence_keyword_then_chain(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) else {
        return Ok(None);
    };
    if then_idx == 0 || then_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let head_tokens = trim_commas(&tokens[..then_idx]);
    let Some(head_effect) = parse_keyword_mechanic_clause(&head_tokens)? else {
        return Ok(None);
    };

    let tail_tokens = trim_commas(&tokens[then_idx + 1..]);
    if tail_tokens.is_empty() {
        return Ok(Some(vec![head_effect]));
    }

    let mut effects = vec![head_effect];
    if let Some(mut counter_effects) = parse_sentence_put_counter_sequence(&tail_tokens)? {
        effects.append(&mut counter_effects);
        return Ok(Some(effects));
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    effects.append(&mut tail_effects);
    Ok(Some(effects))
}

fn parse_sentence_chain_then_keyword(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            tokens
                .iter()
                .position(|token| token.is_word("then"))
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..head_end]);
    let tail_tokens = trim_commas(&tokens[tail_start..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let Some(keyword_effect) = parse_keyword_mechanic_clause(&tail_tokens)? else {
        return Ok(None);
    };
    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    head_effects.push(keyword_effect);
    Ok(Some(head_effects))
}

fn parse_sentence_exile_then_may_put_from_exile(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let split = tokens
        .windows(2)
        .position(|window| matches!(window[0], Token::Comma(_)) && window[1].is_word("then"))
        .map(|idx| (idx, idx + 2))
        .or_else(|| {
            tokens
                .iter()
                .position(|token| token.is_word("then"))
                .and_then(|idx| (idx > 0 && idx + 1 < tokens.len()).then_some((idx, idx + 1)))
        });
    let Some((head_end, tail_start)) = split else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..head_end]);
    let tail_tokens = trim_commas(&tokens[tail_start..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&["you", "may", "put", "any", "number", "of"])
        || !tail_words.contains(&"from")
        || !tail_words.contains(&"exile")
        || !tail_words.contains(&"battlefield")
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }
    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

fn parse_sentence_comma_then_chain_special(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = tokens.windows(2).position(|window| {
        matches!(window[0], Token::Comma(_)) && window[1].is_word("then")
    }) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let head_words = words(&head_tokens);
    let tail_words = words(&tail_tokens);
    let is_that_player_tail = tail_words.starts_with(&["that", "player"]);
    let is_return_source_tail = tail_words.starts_with(&["return", "this"])
        && (tail_words.contains(&"owner") || tail_words.contains(&"owners"))
        && tail_words.contains(&"hand");
    if !is_that_player_tail && !is_return_source_tail {
        return Ok(None);
    }
    if is_return_source_tail
        && !head_words
            .first()
            .is_some_and(|word| matches!(*word, "tap" | "untap"))
    {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if head_effects.is_empty() {
        return Ok(None);
    }

    let mut tail_effects = parse_effect_chain(&tail_tokens)?;
    if tail_effects.is_empty() {
        return Ok(None);
    }

    head_effects.append(&mut tail_effects);
    Ok(Some(head_effects))
}

fn parse_destroy_then_land_controller_graveyard_count_damage_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(comma_then_idx) = tokens.windows(2).position(|window| {
        matches!(window[0], Token::Comma(_)) && window[1].is_word("then")
    }) else {
        return Ok(None);
    };

    let head_tokens = trim_commas(&tokens[..comma_then_idx]);
    let tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    if head_tokens.is_empty() || tail_tokens.is_empty() {
        return Ok(None);
    }

    let tail_words = words(&tail_tokens);
    let suffix = [
        "damage",
        "to",
        "that",
        "lands",
        "controller",
        "equal",
        "to",
        "the",
        "number",
        "of",
        "land",
        "cards",
        "in",
        "that",
        "players",
        "graveyard",
    ];
    let Some(suffix_start) = tail_words
        .windows(suffix.len())
        .position(|window| window == suffix)
    else {
        return Ok(None);
    };
    if suffix_start == 0 || !matches!(tail_words[suffix_start - 1], "deal" | "deals") {
        return Ok(None);
    }
    if suffix_start + suffix.len() != tail_words.len() {
        return Ok(None);
    }

    let mut head_effects = parse_effect_chain(&head_tokens)?;
    if !head_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Destroy { .. }))
    {
        return Ok(None);
    }

    let mut count_filter = ObjectFilter::default();
    count_filter.zone = Some(Zone::Graveyard);
    let tagged_ref = crate::target::ObjectRef::tagged(IT_TAG);
    count_filter.owner = Some(PlayerFilter::ControllerOf(tagged_ref.clone()));
    count_filter.card_types.push(CardType::Land);
    head_effects.push(EffectAst::DealDamage {
        amount: Value::Count(count_filter),
        target: TargetAst::Player(PlayerFilter::ControllerOf(tagged_ref), span_from_tokens(&tail_tokens)),
    });
    Ok(Some(head_effects))
}

fn parse_sentence_destroy_all_attached_to_target(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let sentence_words = words(tokens);
    if sentence_words.first().copied() != Some("destroy") {
        return Ok(None);
    }
    if !tokens
        .get(1)
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        return Ok(None);
    }
    let Some(attached_idx) = tokens.iter().position(|token| token.is_word("attached")) else {
        return Ok(None);
    };
    if !tokens
        .get(attached_idx + 1)
        .is_some_and(|token| token.is_word("to"))
    {
        return Ok(None);
    }
    if attached_idx <= 2 || attached_idx + 2 >= tokens.len() {
        return Ok(None);
    }

    let mut filter_tokens = trim_commas(&tokens[2..attached_idx]).to_vec();
    while filter_tokens
        .last()
        .and_then(Token::as_word)
        .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
    {
        filter_tokens.pop();
    }
    let target_tokens = trim_commas(&tokens[attached_idx + 2..]);
    let target_words = words(&target_tokens);
    let has_timing_tail = target_words.iter().any(|word| {
        matches!(
            *word,
            "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
        )
    });
    let supported_target = target_words.starts_with(&["target"])
        || target_words == ["it"]
        || target_words.starts_with(&["that", "creature"])
        || target_words.starts_with(&["that", "permanent"])
        || target_words.starts_with(&["that", "land"])
        || target_words.starts_with(&["that", "artifact"])
        || target_words.starts_with(&["that", "enchantment"]);
    if filter_tokens.is_empty() || target_tokens.is_empty() || !supported_target || has_timing_tail {
        return Ok(None);
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(vec![EffectAst::DestroyAllAttachedTo { filter, target }]))
}

fn parse_sentence_destroy_then_land_controller_graveyard_count_damage(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_then_land_controller_graveyard_count_damage_sentence(tokens)
}

fn add_tagged_subtype_constraint_to_target(target: &mut TargetAst, tag: TagKey) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag,
                relation: TaggedOpbjectRelation::SharesSubtypeWithTagged,
            });
            true
        }
        TargetAst::WithCount(inner, _) => add_tagged_subtype_constraint_to_target(inner, tag),
        _ => false,
    }
}

fn find_creature_type_choice_phrase(tokens: &[Token]) -> Option<(usize, usize)> {
    for idx in 0..tokens.len() {
        if tokens[idx].is_word("of")
            && tokens.get(idx + 1).is_some_and(|token| token.is_word("the"))
            && tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("creature"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 4).is_some_and(|token| token.is_word("of"))
            && tokens.get(idx + 5).is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 6)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 7));
        }
        if tokens[idx].is_word("of")
            && tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("creature"))
            && tokens.get(idx + 2).is_some_and(|token| token.is_word("type"))
            && tokens.get(idx + 3).is_some_and(|token| token.is_word("of"))
            && tokens.get(idx + 4).is_some_and(|token| token.is_word("your"))
            && tokens
                .get(idx + 5)
                .is_some_and(|token| token.is_word("choice"))
        {
            return Some((idx, 6));
        }
    }
    None
}

fn parse_sentence_destroy_creature_type_of_choice(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["destroy", "all", "creatures"]) {
        return Ok(None);
    }
    if find_creature_type_choice_phrase(tokens).is_none() {
        return Ok(None);
    }

    let chosen_type_tag: TagKey = "chosen_creature_type_ref".into();
    let mut choose_filter = ObjectFilter::creature();
    choose_filter.controller = Some(PlayerFilter::Any);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_type_tag.clone(),
        },
        EffectAst::DestroyAll {
            filter: ObjectFilter::creature()
                .match_tagged(chosen_type_tag, TaggedOpbjectRelation::SharesSubtypeWithTagged),
        },
    ]))
}

fn parse_sentence_pump_creature_type_of_choice(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(get_idx) = tokens
        .iter()
        .position(|token| token.is_word("get") || token.is_word("gets"))
    else {
        return Ok(None);
    };
    if get_idx == 0 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&tokens[..get_idx]);
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(&subject_tokens) else {
        return Ok(None);
    };
    let trailing_subject = trim_commas(&subject_tokens[choice_idx + consumed..]);
    if !trailing_subject.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing creature-type choice subject clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let trimmed_subject_tokens = trim_commas(&subject_tokens[..choice_idx]).to_vec();
    if trimmed_subject_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let chosen_type_tag: TagKey = "chosen_creature_type_ref".into();
    let mut choose_filter = ObjectFilter::creature();
    choose_filter.controller = Some(PlayerFilter::Any);

    // Handle composed clauses like:
    // "Creatures of the creature type of your choice get +2/+2 and gain trample until end of turn."
    let mut gain_candidate_tokens = trimmed_subject_tokens.clone();
    gain_candidate_tokens.extend_from_slice(&tokens[get_idx..]);
    if let Some(mut gain_effects) = parse_gain_ability_sentence(&gain_candidate_tokens)? {
        let mut patched = false;
        for effect in &mut gain_effects {
            match effect {
                EffectAst::PumpAll { filter, .. } | EffectAst::GrantAbilitiesAll { filter, .. } => {
                    if !filter
                        .tagged_constraints
                        .iter()
                        .any(|constraint| {
                            constraint.tag == chosen_type_tag
                                && constraint.relation
                                    == TaggedOpbjectRelation::SharesSubtypeWithTagged
                        })
                    {
                        filter.tagged_constraints.push(TaggedObjectConstraint {
                            tag: chosen_type_tag.clone(),
                            relation: TaggedOpbjectRelation::SharesSubtypeWithTagged,
                        });
                    }
                    patched = true;
                }
                _ => {}
            }
        }
        if patched {
            let mut effects = vec![EffectAst::ChooseObjects {
                filter: choose_filter,
                count: ChoiceCount::exactly(1),
                player: PlayerAst::You,
                tag: chosen_type_tag,
            }];
            effects.extend(gain_effects);
            return Ok(Some(effects));
        }
    }

    let mut filter_tokens = trimmed_subject_tokens;
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("all"))
    {
        filter_tokens.remove(0);
    }
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing creature subject before creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false)?;
    if !filter.card_types.contains(&CardType::Creature) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice pump subject must be creature-based (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let modifier = tokens
        .get(get_idx + 1)
        .and_then(Token::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing power/toughness modifier in creature-type choice pump clause (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
    let (base_power, base_toughness) = parse_pt_modifier_values(modifier).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in creature-type choice pump clause (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    let (power, toughness, duration) =
        parse_get_modifier_values_with_tail(&tokens[get_idx + 1..], base_power, base_toughness)?;

    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: chosen_type_tag.clone(),
        relation: TaggedOpbjectRelation::SharesSubtypeWithTagged,
    });

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_type_tag,
        },
        EffectAst::PumpAll {
            filter,
            power,
            toughness,
            duration,
        },
    ]))
}

fn parse_sentence_return_targets_of_creature_type_of_choice(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let destination_words = words(&tokens[to_idx + 1..]);
    if !destination_words.contains(&"hand") && !destination_words.contains(&"hands") {
        return Ok(None);
    }

    let target_tokens = &tokens[1..to_idx];
    let Some((choice_idx, consumed)) = find_creature_type_choice_phrase(target_tokens) else {
        return Ok(None);
    };

    let trimmed_target = trim_commas(&target_tokens[..choice_idx]);
    if trimmed_target.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing return target before creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let trailing = trim_commas(&target_tokens[choice_idx + consumed..]);
    if !trailing.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing return target clause after creature-type choice phrase (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut target = parse_target_phrase(&trimmed_target)?;
    let chosen_type_tag: TagKey = "chosen_creature_type_ref".into();
    if !add_tagged_subtype_constraint_to_target(&mut target, chosen_type_tag.clone()) {
        return Err(CardTextError::ParseError(format!(
            "creature-type choice return target must be object-based (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut choose_filter = ObjectFilter::creature();
    choose_filter.controller = Some(PlayerFilter::Any);
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_type_tag,
        },
        EffectAst::ReturnToHand { target },
    ]))
}

fn return_segment_mentions_zone(tokens: &[Token]) -> bool {
    let segment_words = words(tokens);
    segment_words.contains(&"graveyard")
        || segment_words.contains(&"graveyards")
        || segment_words.contains(&"battlefield")
        || segment_words.contains(&"hand")
        || segment_words.contains(&"hands")
        || segment_words.contains(&"library")
        || segment_words.contains(&"libraries")
        || segment_words.contains(&"exile")
}

fn parse_sentence_return_multiple_targets(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !tokens.first().is_some_and(|token| token.is_word("return")) {
        return Ok(None);
    }
    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Ok(None);
    };
    if to_idx <= 1 {
        return Ok(None);
    }

    let destination_words = words(&tokens[to_idx + 1..]);
    let is_hand = destination_words.contains(&"hand") || destination_words.contains(&"hands");
    let is_battlefield = destination_words.contains(&"battlefield");
    let tapped = destination_words.contains(&"tapped");
    if !is_hand && !is_battlefield {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..to_idx]);
    let has_multi_separator = target_tokens.iter().any(|token| {
        token.is_word("and")
            || matches!(token, Token::Comma(_))
            || token.is_word("or")
            || token.is_word("and/or")
    });
    if !has_multi_separator {
        return Ok(None);
    }

    let mut segments: Vec<Vec<Token>> = Vec::new();
    for and_segment in split_on_and(&target_tokens) {
        for comma_segment in split_on_comma(&and_segment) {
            let trimmed = trim_commas(&comma_segment);
            if !trimmed.is_empty() {
                let trimmed_words = words(&trimmed);
                let starts_new_target = trimmed_words.first().is_some_and(|word| {
                    matches!(
                        *word,
                        "target"
                            | "up"
                            | "another"
                            | "other"
                            | "this"
                            | "that"
                            | "it"
                            | "them"
                            | "all"
                            | "each"
                    )
                });
                let mentions_target = trimmed_words.contains(&"target");
                let starts_like_zone_suffix = trimmed_words.first().is_some_and(|word| {
                    matches!(*word, "from" | "to" | "in" | "on" | "under")
                });
                if !segments.is_empty()
                    && !starts_new_target
                    && !mentions_target
                    && !starts_like_zone_suffix
                {
                    let last = segments.last_mut().expect("segments is non-empty");
                    last.push(Token::Comma(TextSpan::synthetic()));
                    last.extend(trimmed.to_vec());
                } else {
                    segments.push(trimmed.to_vec());
                }
            }
        }
    }
    if segments.len() < 2 {
        return Ok(None);
    }

    let shared_quantifier = segments
        .first()
        .and_then(|segment| segment.first())
        .and_then(Token::as_word)
        .filter(|word| matches!(*word, "all" | "each"))
        .map(str::to_string);

    let shared_suffix = segments
        .last()
        .and_then(|segment| {
            segment
                .iter()
                .position(|token| token.is_word("from"))
                .map(|idx| segment[idx..].to_vec())
        })
        .unwrap_or_default();

    let mut effects = Vec::new();
    for mut segment in segments {
        if !return_segment_mentions_zone(&segment) && !shared_suffix.is_empty() {
            segment.extend(shared_suffix.clone());
        }
        if let Some(quantifier) = shared_quantifier.as_deref() {
            let segment_words = words(&segment);
            let has_explicit_quantifier =
                matches!(segment_words.first().copied(), Some("all" | "each"));
            let starts_like_target_reference = matches!(
                segment_words.first().copied(),
                Some("target" | "up" | "this" | "that" | "it" | "them" | "another")
            );
            if !has_explicit_quantifier
                && !starts_like_target_reference
                && !segment_words.contains(&"target")
            {
                segment.insert(
                    0,
                    Token::Word(quantifier.to_string(), TextSpan::synthetic()),
                );
            }
        }
        let segment_words = words(&segment);
        if matches!(segment_words.first().copied(), Some("all" | "each")) {
            if segment.len() < 2 {
                return Err(CardTextError::ParseError(format!(
                    "missing return-all filter (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
            let filter = parse_object_filter(&segment[1..], false)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnAllToBattlefield { filter, tapped });
            } else {
                effects.push(EffectAst::ReturnAllToHand { filter });
            }
        } else {
            let target = parse_target_phrase(&segment)?;
            if is_battlefield {
                effects.push(EffectAst::ReturnToBattlefield {
                    target,
                    tapped,
                    controller: ReturnControllerAst::Preserve,
                });
            } else {
                effects.push(EffectAst::ReturnToHand { target });
            }
        }
    }

    Ok(Some(effects))
}

fn parse_distribute_counters_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("distribute") {
        return Ok(None);
    }

    let (count, used) = parse_number(&tokens[1..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing distributed counter amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let rest = &tokens[1 + used..];
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported distributed counter type (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    let among_idx = rest
        .iter()
        .position(|token| token.is_word("among"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target clause after 'among' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_tokens = trim_commas(&rest[among_idx + 1..]);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed counter targets (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let (target_count, used_count) = parse_counter_target_count_prefix(&target_tokens)?
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing distributed target count prefix (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let target_phrase = &target_tokens[used_count..];
    if target_phrase.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing distributed target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target = parse_target_phrase(target_phrase)?;

    Ok(Some(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target,
        target_count: Some(target_count),
        distributed: true,
    }))
}

fn parse_sentence_distribute_counters(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut head_tokens = tokens.to_vec();
    let mut tail_tokens: Vec<Token> = Vec::new();

    if let Some(comma_then_idx) = tokens.windows(2).position(|window| {
        matches!(window[0], Token::Comma(_)) && window[1].is_word("then")
    }) {
        head_tokens = tokens[..comma_then_idx].to_vec();
        tail_tokens = trim_commas(&tokens[comma_then_idx + 2..]);
    } else if let Some(then_idx) = tokens.iter().position(|token| token.is_word("then")) {
        head_tokens = tokens[..then_idx].to_vec();
        tail_tokens = trim_commas(&tokens[then_idx + 1..]);
    }

    let Some(primary) = parse_distribute_counters_sentence(&head_tokens)? else {
        return Ok(None);
    };

    let mut effects = vec![primary];
    if !tail_tokens.is_empty() {
        effects.extend(parse_effect_chain(&tail_tokens)?);
    }

    Ok(Some(effects))
}

fn parse_sentence_exile_that_token_at_end_of_combat(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if is_exile_that_token_at_end_of_combat(tokens) {
        return Ok(Some(vec![EffectAst::ExileThatTokenAtEndOfCombat]));
    }
    Ok(None)
}

fn parse_sentence_take_extra_turn(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_take_extra_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_earthbend(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(earthbend) = parse_earthbend_sentence(tokens)? else {
        return Ok(None);
    };

    // Support chained text like "earthbend 8, then untap that land."
    let Some((_, used)) = parse_number(&tokens[1..]) else {
        return Ok(Some(vec![earthbend]));
    };
    let mut tail = trim_commas(&tokens[1 + used..]).to_vec();
    while tail.first().is_some_and(|token| token.is_word("then")) {
        tail.remove(0);
    }
    if tail.is_empty() {
        return Ok(Some(vec![earthbend]));
    }

    let mut effects = vec![earthbend];
    effects.extend(parse_effect_chain(&tail)?);
    Ok(Some(effects))
}

fn parse_sentence_enchant(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_enchant_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_cant_effect(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_cant_effect_sentence(tokens)
}

fn parse_sentence_prevent_damage(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_prevent_damage_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_gain_ability_to_source(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_gain_ability_to_source_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_gain_ability(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_ability_sentence(tokens)
}

fn parse_sentence_you_and_each_opponent_voted_with_you(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_you_and_each_opponent_voted_with_you_sentence(tokens)
}

fn parse_sentence_gain_life_equal_to_power(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_power_sentence(tokens)
}

fn parse_sentence_gain_x_plus_life(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_x_plus_life_sentence(tokens)
}

fn parse_sentence_for_each_exiled_this_way(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_exiled_this_way_sentence(tokens)
}

fn parse_sentence_each_player_put_permanent_cards_exiled_with_source(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_each_player_put_permanent_cards_exiled_with_source_sentence(tokens)
}

fn parse_sentence_for_each_destroyed_this_way(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_for_each_destroyed_this_way_sentence(tokens)
}

fn parse_sentence_exile_then_return_same_object(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_then_return_same_object_sentence(tokens)
}

fn parse_sentence_search_library(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_search_library_sentence(tokens)
}

fn parse_sentence_shuffle_graveyard_into_library(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shuffle_graveyard_into_library_sentence(tokens)
}

fn parse_sentence_exile_hand_and_graveyard_bundle(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_hand_and_graveyard_bundle_sentence(tokens)
}

fn parse_sentence_play_from_graveyard(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_play_from_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_look_at_hand(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_hand_sentence(tokens)
}

fn parse_sentence_look_at_top_then_exile_one(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_top_then_exile_one_sentence(tokens)
}

fn parse_sentence_gain_life_equal_to_age(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gain_life_equal_to_age_sentence(tokens)
}

fn parse_sentence_for_each_opponent_doesnt(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_opponent_doesnt(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_for_each_player_doesnt(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_player_doesnt(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_vote_start(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_start_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_for_each_vote_clause(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_for_each_vote_clause(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_vote_extra(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_vote_extra_sentence(tokens).map(|effect| vec![effect]))
}

fn parse_sentence_after_turn(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_after_turn_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_same_name_target_fanout(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_target_fanout_sentence(tokens)
}

fn parse_sentence_shared_color_target_fanout(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_shared_color_target_fanout_sentence(tokens)
}

fn parse_sentence_same_name_gets_fanout(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_gets_fanout_sentence(tokens)
}

fn parse_sentence_delayed_until_next_end_step(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_delayed_until_next_end_step_sentence(tokens)
}

fn parse_sentence_destroy_or_exile_all_split(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_destroy_or_exile_all_split_sentence(tokens)
}

fn parse_sentence_exile_up_to_one_each_target_type(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_exile_up_to_one_each_target_type_sentence(tokens)
}

fn parse_sentence_damage_unless_controller_has_source_deal_damage(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) else {
        return Ok(None);
    };
    if unless_idx == 0 || unless_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let before_tokens = trim_commas(&tokens[..unless_idx]);
    if before_tokens.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(&before_tokens)?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let Some(main_damage) = effects.first() else {
        return Ok(None);
    };
    let EffectAst::DealDamage {
        amount: main_amount,
        target: main_target,
    } = main_damage
    else {
        return Ok(None);
    };
    if !matches!(main_target, TargetAst::Object(_, _, _) | TargetAst::WithCount(_, _)) {
        return Ok(None);
    }

    let after_unless = trim_commas(&tokens[unless_idx + 1..]);
    let after_words = words(&after_unless);
    let has_controller_clause = after_words.starts_with(&["that"])
        && after_words
            .iter()
            .any(|word| *word == "controller" || *word == "controllers");
    if !has_controller_clause {
        return Ok(None);
    }
    let Some(has_idx) = after_unless
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
    else {
        return Ok(None);
    };
    if has_idx + 1 >= after_unless.len() {
        return Ok(None);
    }

    let alt_tokens = &after_unless[has_idx + 1..];
    let Some(deal_idx) = alt_tokens
        .iter()
        .position(|token| token.is_word("deal") || token.is_word("deals"))
    else {
        return Ok(None);
    };
    let deal_tail = &alt_tokens[deal_idx..];
    let Some((alt_amount, used)) = parse_value(&deal_tail[1..]) else {
        return Ok(None);
    };
    if !deal_tail
        .get(1 + used)
        .is_some_and(|token| token.is_word("damage"))
    {
        return Ok(None);
    }

    let mut alt_target_tokens = &deal_tail[2 + used..];
    if alt_target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        alt_target_tokens = &alt_target_tokens[1..];
    }
    let alt_target_words = words(alt_target_tokens);
    if !matches!(alt_target_words.as_slice(), ["them"] | ["that", "player"]) {
        return Ok(None);
    }

    let alternative = EffectAst::DealDamage {
        amount: alt_amount,
        target: TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    };
    let unless = EffectAst::UnlessAction {
        effects: vec![EffectAst::DealDamage {
            amount: main_amount.clone(),
            target: main_target.clone(),
        }],
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    };
    Ok(Some(vec![unless]))
}

fn parse_sentence_unless_pays(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Find "unless" in the token stream
    let unless_idx = match tokens.iter().position(|t| t.is_word("unless")) {
        Some(idx) => idx,
        None => return Ok(None),
    };

    // Leading form: "Unless you pay ..., <effects>."
    // Rewrite by parsing the effect tail after the first comma and wrapping it
    // in the parsed unless-payment clause.
    if unless_idx == 0 {
        let comma_idx = match tokens.iter().position(|token| matches!(token, Token::Comma(_))) {
            Some(idx) => idx,
            None => return Ok(None),
        };
        if comma_idx + 1 >= tokens.len() {
            return Ok(None);
        }

        let effects = parse_effect_chain(&tokens[comma_idx + 1..])?;
        if effects.is_empty() {
            return Ok(None);
        }

        let unless_clause = &tokens[..comma_idx];
        if let Some(unless_effect) = try_build_unless(effects, unless_clause, 0)? {
            return Ok(Some(vec![unless_effect]));
        }
        return Ok(None);
    }

    // Need at least something before "unless" and something after.
    let before_words: Vec<&str> = tokens[..unless_idx]
        .iter()
        .filter_map(Token::as_word)
        .collect();

    // Skip "counter ... unless" - already handled by parse_counter via CounterUnlessPays
    if before_words.first() == Some(&"counter") {
        return Ok(None);
    }
    // Ignore "unless ... pays" that appears inside quoted token rules text.
    // Example: create token with "{1}, Sacrifice this token: Counter ... unless ...".
    if before_words.first() == Some(&"create")
        && before_words.contains(&"token")
        && before_words.contains(&"sacrifice")
        && before_words.contains(&"counter")
    {
        return Ok(None);
    }

    // Handle "each opponent/player ... unless" by wrapping in ForEachOpponent/ForEachPlayer.
    // Structure: ForEachOpponent { [UnlessPays/UnlessAction { per-player effects }] }
    let each_prefix = if before_words.starts_with(&["each", "opponent"])
        || before_words.starts_with(&["each", "opponents"])
    {
        Some("opponent")
    } else if before_words.starts_with(&["each", "player"]) {
        Some("player")
    } else {
        None
    };
    if let Some(prefix_kind) = each_prefix {
        // Tokens between "each opponent/player" and "unless" form the per-player effect
        let inner_token_start = tokens
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_word().map(|_| i))
            .nth(2) // skip "each" and "opponent"/"player"
            .unwrap_or(2);
        let inner_tokens = &tokens[inner_token_start..unless_idx];
        if let Ok(inner_effects) = parse_effect_chain(inner_tokens) {
            if !inner_effects.is_empty() {
                if let Some(unless_effect) = try_build_unless(inner_effects, tokens, unless_idx)? {
                    let wrapper = match prefix_kind {
                        "opponent" => EffectAst::ForEachOpponent {
                            effects: vec![unless_effect],
                        },
                        _ => EffectAst::ForEachPlayer {
                            effects: vec![unless_effect],
                        },
                    };
                    return Ok(Some(vec![wrapper]));
                }
            }
        }
        return Ok(None);
    }

    // Normal path: parse effects before "unless", then build unless wrapper
    let effect_tokens = &tokens[..unless_idx];
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Ok(None);
    }

    if let Some(unless_effect) = try_build_unless(effects, tokens, unless_idx)? {
        return Ok(Some(vec![unless_effect]));
    }

    Ok(None)
}

/// Try to build an UnlessPays or UnlessAction AST from the tokens after "unless".
/// Returns the unless wrapper containing the given `effects` as the main effects.
fn try_build_unless(
    effects: Vec<EffectAst>,
    tokens: &[Token],
    unless_idx: usize,
) -> Result<Option<EffectAst>, CardTextError> {
    let after_unless = &tokens[unless_idx + 1..];
    let after_words: Vec<&str> = after_unless.iter().filter_map(Token::as_word).collect();

    // Determine the player from the "unless" clause
    let (player, action_token_start) = if after_words.starts_with(&["you"]) {
        (PlayerAst::You, 1)
    } else if after_words.starts_with(&["any", "player"]) {
        (PlayerAst::Any, 2)
    } else if after_words.len() >= 6
        && after_words[0] == "that"
        && matches!(
            after_words[1],
            "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
        )
        && matches!(after_words[2], "controller" | "controllers")
        && after_words[3] == "or"
        && after_words[4] == "that"
        && after_words[5] == "player"
    {
        (PlayerAst::ItsController, 6)
    } else if after_words.len() >= 3
        && after_words[0] == "that"
        && matches!(
            after_words[1],
            "creature" | "creatures" | "permanent" | "permanents" | "source" | "sources"
        )
        && matches!(after_words[2], "controller" | "controllers")
    {
        (PlayerAst::ItsController, 3)
    } else if after_words.starts_with(&["they"]) {
        (PlayerAst::That, 1)
    } else if after_words.starts_with(&["defending", "player"]) {
        (PlayerAst::Defending, 2)
    } else if after_words.starts_with(&["that", "player"]) {
        (PlayerAst::That, 2)
    } else if after_words.starts_with(&["its", "controller"]) {
        (PlayerAst::ItsController, 2)
    } else if after_words.starts_with(&["their", "controller"]) {
        (PlayerAst::ItsController, 2)
    } else if after_words.starts_with(&["its", "owner"]) {
        (PlayerAst::ItsOwner, 2)
    } else if after_words.starts_with(&["their", "owner"]) {
        (PlayerAst::ItsOwner, 2)
    } else {
        return Ok(None);
    };

    // Find the token position corresponding to action_token_start words in
    let mut action_token_idx = 0;
    let mut wc = 0;
    for (i, token) in after_unless.iter().enumerate() {
        if token.as_word().is_some() {
            wc += 1;
            if wc == action_token_start {
                action_token_idx = i + 1;
                break;
            }
        }
    }

    let action_tokens = &after_unless[action_token_idx..];
    let action_words: Vec<&str> = action_tokens.iter().filter_map(Token::as_word).collect();

    // "unless [player] pays N life" should compile as an unless-action branch
    // where the deciding player loses life.
    if action_words.first() == Some(&"pay") || action_words.first() == Some(&"pays") {
        let life_tokens = &action_tokens[1..];
        if let Some((amount, used)) = parse_value(life_tokens)
            && life_tokens
                .get(used)
                .is_some_and(|token| token.is_word("life"))
            && life_tokens
                .get(used + 1)
                .map_or(true, |token| matches!(token, Token::Period(_)))
        {
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative: vec![EffectAst::LoseLife { amount, player }],
                player,
            }));
        }
    }

    // Try mana payment first: "pay(s) {mana} [optional trailing condition]"
    // Uses greedy mana parsing — collects mana symbols until first non-mana word,
    // then categorizes remaining tokens to decide whether to accept.
    if action_words.first() == Some(&"pay") || action_words.first() == Some(&"pays") {
        // Skip any non-word tokens between "pay" and mana
        let mana_start = action_tokens
            .iter()
            .skip(1)
            .position(|t| t.as_word().is_some())
            .map(|p| p + 1)
            .unwrap_or(1);
        let mana_tokens = &action_tokens[mana_start..];
        let mut mana = Vec::new();
        let mut remaining_idx = mana_tokens.len();
        for (i, token) in mana_tokens.iter().enumerate() {
            if let Some(word) = token.as_word() {
                match parse_mana_symbol(word) {
                    Ok(symbol) => mana.push(symbol),
                    Err(_) => {
                        remaining_idx = i;
                        break;
                    }
                }
            }
        }

        if !mana.is_empty() {
            // Check what follows the mana symbols
            let remaining_words: Vec<&str> = mana_tokens[remaining_idx..]
                .iter()
                .filter_map(Token::as_word)
                .collect();

            let accept = if remaining_words.is_empty() {
                // Pure mana payment (e.g., "pays {2}")
                true
            } else if remaining_words.first() == Some(&"life") {
                // "pay N life" — not a mana payment, it's a life cost
                false
            } else if remaining_words.first() == Some(&"before") {
                // Timing condition like "before that step" — accept, drop condition
                true
            } else {
                // Unknown trailing tokens (for each, where X is, etc.) — skip for now
                false
            };

            if accept {
                return Ok(Some(EffectAst::UnlessPays {
                    effects,
                    player,
                    mana,
                }));
            }

            if !remaining_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing unless-payment clause (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
        }
    }

    // Try action-based alternative: "unless you [verb] [object]"
    // Both the main effects and the alternative must parse successfully.
    if let Ok(alternative) = parse_effect_chain(after_unless) {
        if !alternative.is_empty() {
            return Ok(Some(EffectAst::UnlessAction {
                effects,
                alternative,
                player,
            }));
        }
    }

    Ok(None)
}

const PRE_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "you-and-target-player-each-draw",
        parser: parse_sentence_you_and_target_player_each_draw,
    },
    SentencePrimitive {
        name: "you-and-attacking-player-each-draw-and-lose",
        parser: parse_sentence_you_and_attacking_player_each_draw_and_lose,
    },
    SentencePrimitive {
        name: "sacrifice-it-next-end-step",
        parser: parse_sentence_sacrifice_it_next_end_step,
    },
    SentencePrimitive {
        name: "sacrifice-at-end-of-combat",
        parser: parse_sentence_sacrifice_at_end_of_combat,
    },
    SentencePrimitive {
        name: "token-copy-modifier",
        parser: parse_sentence_token_copy_modifier,
    },
    SentencePrimitive {
        name: "each-player-choose-keep-rest-sacrifice",
        parser: parse_sentence_each_player_choose_and_sacrifice_rest,
    },
    SentencePrimitive {
        name: "target-player-choose-then-put-on-top-library",
        parser: parse_sentence_target_player_chooses_then_puts_on_top_of_library,
    },
    SentencePrimitive {
        name: "target-player-choose-then-you-put-it-onto-battlefield",
        parser: parse_sentence_target_player_chooses_then_you_put_it_onto_battlefield,
    },
    SentencePrimitive {
        name: "exile-instead-of-graveyard",
        parser: parse_sentence_exile_instead_of_graveyard,
    },
];

const POST_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
    SentencePrimitive {
        name: "exile-target-creature-with-greatest-power",
        parser: parse_sentence_exile_target_creature_with_greatest_power,
    },
    SentencePrimitive {
        name: "counter-target-spell-thats-second-cast-this-turn",
        parser: parse_sentence_counter_target_spell_thats_second_cast_this_turn,
    },
    SentencePrimitive {
        name: "counter-target-spell-if-it-was-kicked",
        parser: parse_sentence_counter_target_spell_if_it_was_kicked,
    },
    SentencePrimitive {
        name: "destroy-creature-type-of-choice",
        parser: parse_sentence_destroy_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "pump-creature-type-of-choice",
        parser: parse_sentence_pump_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "return-multiple-targets",
        parser: parse_sentence_return_multiple_targets,
    },
    SentencePrimitive {
        name: "return-creature-type-of-choice",
        parser: parse_sentence_return_targets_of_creature_type_of_choice,
    },
    SentencePrimitive {
        name: "distribute-counters",
        parser: parse_sentence_distribute_counters,
    },
    SentencePrimitive {
        name: "keyword-then-chain",
        parser: parse_sentence_keyword_then_chain,
    },
    SentencePrimitive {
        name: "chain-then-keyword",
        parser: parse_sentence_chain_then_keyword,
    },
    SentencePrimitive {
        name: "exile-then-may-put-from-exile",
        parser: parse_sentence_exile_then_may_put_from_exile,
    },
    SentencePrimitive {
        name: "destroy-all-attached-to-target",
        parser: parse_sentence_destroy_all_attached_to_target,
    },
    SentencePrimitive {
        name: "comma-then-chain-special",
        parser: parse_sentence_comma_then_chain_special,
    },
    SentencePrimitive {
        name: "destroy-then-land-controller-graveyard-count-damage",
        parser: parse_sentence_destroy_then_land_controller_graveyard_count_damage,
    },
    SentencePrimitive {
        name: "draw-then-connive",
        parser: parse_sentence_draw_then_connive,
    },
    SentencePrimitive {
        name: "return-then-do-same-for-subtypes",
        parser: parse_sentence_return_then_do_same_for_subtypes,
    },
    SentencePrimitive {
        name: "put-counter-sequence",
        parser: parse_sentence_put_counter_sequence,
    },
    SentencePrimitive {
        name: "return-with-counters-on-it",
        parser: parse_sentence_return_with_counters_on_it,
    },
    SentencePrimitive {
        name: "each-player-return-with-additional-counter",
        parser: parse_sentence_each_player_return_with_additional_counter,
    },
    SentencePrimitive {
        name: "sacrifice-any-number",
        parser: parse_sentence_sacrifice_any_number,
    },
    SentencePrimitive {
        name: "monstrosity",
        parser: parse_sentence_monstrosity,
    },
    SentencePrimitive {
        name: "for-each-counter-removed",
        parser: parse_sentence_for_each_counter_removed,
    },
    SentencePrimitive {
        name: "exile-that-token-end-of-combat",
        parser: parse_sentence_exile_that_token_at_end_of_combat,
    },
    SentencePrimitive {
        name: "take-extra-turn",
        parser: parse_sentence_take_extra_turn,
    },
    SentencePrimitive {
        name: "earthbend",
        parser: parse_sentence_earthbend,
    },
    SentencePrimitive {
        name: "enchant",
        parser: parse_sentence_enchant,
    },
    SentencePrimitive {
        name: "cant-effect",
        parser: parse_sentence_cant_effect,
    },
    SentencePrimitive {
        name: "prevent-damage",
        parser: parse_sentence_prevent_damage,
    },
    SentencePrimitive {
        name: "gain-ability-to-source",
        parser: parse_sentence_gain_ability_to_source,
    },
    SentencePrimitive {
        name: "gain-ability",
        parser: parse_sentence_gain_ability,
    },
    SentencePrimitive {
        name: "vote-with-you",
        parser: parse_sentence_you_and_each_opponent_voted_with_you,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-power",
        parser: parse_sentence_gain_life_equal_to_power,
    },
    SentencePrimitive {
        name: "gain-x-plus-life",
        parser: parse_sentence_gain_x_plus_life,
    },
    SentencePrimitive {
        name: "for-each-exiled-this-way",
        parser: parse_sentence_for_each_exiled_this_way,
    },
    SentencePrimitive {
        name: "each-player-put-permanent-cards-exiled-with-source",
        parser: parse_sentence_each_player_put_permanent_cards_exiled_with_source,
    },
    SentencePrimitive {
        name: "for-each-destroyed-this-way",
        parser: parse_sentence_for_each_destroyed_this_way,
    },
    SentencePrimitive {
        name: "exile-then-return-same-object",
        parser: parse_sentence_exile_then_return_same_object,
    },
    SentencePrimitive {
        name: "search-library",
        parser: parse_sentence_search_library,
    },
    SentencePrimitive {
        name: "shuffle-graveyard-into-library",
        parser: parse_sentence_shuffle_graveyard_into_library,
    },
    SentencePrimitive {
        name: "exile-hand-and-graveyard-bundle",
        parser: parse_sentence_exile_hand_and_graveyard_bundle,
    },
    SentencePrimitive {
        name: "play-from-graveyard",
        parser: parse_sentence_play_from_graveyard,
    },
    SentencePrimitive {
        name: "look-at-top-then-exile-one",
        parser: parse_sentence_look_at_top_then_exile_one,
    },
    SentencePrimitive {
        name: "look-at-hand",
        parser: parse_sentence_look_at_hand,
    },
    SentencePrimitive {
        name: "gain-life-equal-to-age",
        parser: parse_sentence_gain_life_equal_to_age,
    },
    SentencePrimitive {
        name: "for-each-player-doesnt",
        parser: parse_sentence_for_each_player_doesnt,
    },
    SentencePrimitive {
        name: "for-each-opponent-doesnt",
        parser: parse_sentence_for_each_opponent_doesnt,
    },
    SentencePrimitive {
        name: "vote-start",
        parser: parse_sentence_vote_start,
    },
    SentencePrimitive {
        name: "for-each-vote-clause",
        parser: parse_sentence_for_each_vote_clause,
    },
    SentencePrimitive {
        name: "vote-extra",
        parser: parse_sentence_vote_extra,
    },
    SentencePrimitive {
        name: "after-turn",
        parser: parse_sentence_after_turn,
    },
    SentencePrimitive {
        name: "same-name-target-fanout",
        parser: parse_sentence_same_name_target_fanout,
    },
    SentencePrimitive {
        name: "shared-color-target-fanout",
        parser: parse_sentence_shared_color_target_fanout,
    },
    SentencePrimitive {
        name: "same-name-gets-fanout",
        parser: parse_sentence_same_name_gets_fanout,
    },
    SentencePrimitive {
        name: "delayed-next-end-step",
        parser: parse_sentence_delayed_until_next_end_step,
    },
    SentencePrimitive {
        name: "delayed-when-that-dies-this-turn",
        parser: parse_delayed_when_that_dies_this_turn_sentence,
    },
    SentencePrimitive {
        name: "destroy-or-exile-all-split",
        parser: parse_sentence_destroy_or_exile_all_split,
    },
    SentencePrimitive {
        name: "exile-up-to-one-each-target-type",
        parser: parse_sentence_exile_up_to_one_each_target_type,
    },
    SentencePrimitive {
        name: "damage-unless-controller-has-source-deal-damage",
        parser: parse_sentence_damage_unless_controller_has_source_deal_damage,
    },
    SentencePrimitive {
        name: "unless-pays",
        parser: parse_sentence_unless_pays,
    },
];

fn parse_effect_sentence(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    parser_trace("parse_effect_sentence:entry", tokens);
    let sentence_words = words(tokens);
    if is_activate_only_restriction_sentence(tokens) {
        return Ok(Vec::new());
    }
    if is_trigger_only_restriction_sentence(tokens) {
        return Ok(Vec::new());
    }
    let is_each_player_lose_discard_sacrifice_chain = sentence_words
        .starts_with(&["each", "player"])
        && sentence_words.contains(&"then")
        && (sentence_words.contains(&"lose") || sentence_words.contains(&"loses"))
        && (sentence_words.contains(&"discard") || sentence_words.contains(&"discards"))
        && (sentence_words.contains(&"sacrifice") || sentence_words.contains(&"sacrifices"));
    if is_each_player_lose_discard_sacrifice_chain {
        return Err(CardTextError::ParseError(format!(
            "unsupported each-player lose/discard/sacrifice chain clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let is_each_player_exile_sacrifice_return_exiled = sentence_words
        .starts_with(&["each", "player", "exiles", "all"])
        && sentence_words.contains(&"sacrifices")
        && sentence_words.contains(&"puts")
        && sentence_words.contains(&"exiled")
        && sentence_words.contains(&"this")
        && sentence_words.contains(&"way");
    if is_each_player_exile_sacrifice_return_exiled {
        return Err(CardTextError::ParseError(format!(
            "unsupported each-player exile/sacrifice/return-this-way clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_loses_all_abilities = (sentence_words.contains(&"lose")
        || sentence_words.contains(&"loses"))
        && sentence_words
            .windows(2)
            .any(|window| window == ["all", "abilities"]);
    if has_loses_all_abilities && sentence_words.contains(&"becomes") {
        return Err(CardTextError::ParseError(format!(
            "unsupported loses-all-abilities with becomes clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_where_x_clause = sentence_words
        .windows(3)
        .any(|window| window == ["where", "x", "is"]);
    if has_where_x_clause {
        return Err(CardTextError::ParseError(format!(
            "unsupported where-x clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_spent_to_cast_this_spell = sentence_words
        .windows(6)
        .any(|window| window == ["was", "spent", "to", "cast", "this", "spell"]);
    if has_spent_to_cast_this_spell {
        return Err(CardTextError::ParseError(format!(
            "unsupported spent-to-cast conditional clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_would_enter_instead_replacement = sentence_words
        .iter()
        .any(|word| *word == "would")
        && sentence_words
            .iter()
            .any(|word| *word == "enter" || *word == "enters")
        && sentence_words.iter().any(|word| *word == "instead");
    if has_would_enter_instead_replacement {
        return Err(CardTextError::ParseError(format!(
            "unsupported would-enter replacement clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_different_mana_value_constraint = sentence_words
        .windows(3)
        .any(|window| window == ["different", "mana", "value"]);
    if has_different_mana_value_constraint {
        return Err(CardTextError::ParseError(format!(
            "unsupported different-mana-value constraint clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_most_common_color_constraint = sentence_words
        .windows(5)
        .any(|window| window == ["most", "common", "color", "among", "all"])
        && sentence_words.contains(&"permanents");
    if has_most_common_color_constraint {
        return Err(CardTextError::ParseError(format!(
            "unsupported most-common-color constraint clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_power_vs_count_constraint = sentence_words.contains(&"power")
        && sentence_words.windows(8).any(|window| {
            window
                == [
                    "less", "than", "or", "equal", "to", "the", "number", "of",
                ]
        });
    if has_power_vs_count_constraint {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-vs-count conditional clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_put_into_graveyards_from_battlefield_this_turn = sentence_words.windows(8).any(
        |window| {
            window
                == [
                    "put",
                    "into",
                    "graveyards",
                    "from",
                    "the",
                    "battlefield",
                    "this",
                    "turn",
                ]
        },
    );
    if has_put_into_graveyards_from_battlefield_this_turn {
        return Err(CardTextError::ParseError(format!(
            "unsupported put-into-graveyards-from-battlefield count clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_phase_out_until_leaves = sentence_words
        .iter()
        .any(|word| matches!(*word, "phase" | "phases" | "phased"))
        && sentence_words.contains(&"until")
        && sentence_words
            .windows(3)
            .any(|window| window == ["leaves", "the", "battlefield"]);
    if has_phase_out_until_leaves {
        return Err(CardTextError::ParseError(format!(
            "unsupported phase-out-until-leaves clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_investigate_for_each = sentence_words
        .iter()
        .any(|word| *word == "investigate" || *word == "investigates")
        && sentence_words
            .windows(2)
            .any(|window| window == ["for", "each"]);
    if has_investigate_for_each {
        return Err(CardTextError::ParseError(format!(
            "unsupported investigate-for-each clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_same_name_as_another_in_hand = sentence_words
        .windows(6)
        .any(|window| window == ["same", "name", "as", "another", "card", "in"])
        && sentence_words.contains(&"hand");
    if has_same_name_as_another_in_hand {
        return Err(CardTextError::ParseError(format!(
            "unsupported same-name-as-another-in-hand discard clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_for_each_mana_from_spent_to_cast = sentence_words
        .windows(4)
        .any(|window| window == ["for", "each", "mana", "from"])
        && sentence_words.contains(&"spent")
        && sentence_words
            .windows(4)
            .any(|window| window == ["cast", "this", "spell", "create"]);
    if has_for_each_mana_from_spent_to_cast {
        return Err(CardTextError::ParseError(format!(
            "unsupported for-each-mana-from-spent clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_when_you_sacrifice_this_way = sentence_words
        .windows(3)
        .any(|window| window == ["when", "you", "sacrifice"])
        && sentence_words
            .windows(2)
            .any(|window| window == ["this", "way"]);
    if has_when_you_sacrifice_this_way {
        return Err(CardTextError::ParseError(format!(
            "unsupported when-you-sacrifice-this-way clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_sacrifice_any_number_then_draw_that_many = sentence_words
        .iter()
        .any(|word| *word == "sacrifice" || *word == "sacrifices")
        && sentence_words
            .windows(3)
            .any(|window| window == ["any", "number", "of"])
        && sentence_words
            .iter()
            .any(|word| *word == "draw" || *word == "draws")
        && sentence_words
            .windows(2)
            .any(|window| window == ["that", "many"]);
    if has_sacrifice_any_number_then_draw_that_many {
        return Err(CardTextError::ParseError(format!(
            "unsupported sacrifice-any-number-then-draw-that-many clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_copy_spell_legendary_exception = sentence_words.contains(&"copy")
        && sentence_words.contains(&"spell")
        && sentence_words.contains(&"legendary")
        && (sentence_words.contains(&"except") || sentence_words.contains(&"isnt"));
    if has_copy_spell_legendary_exception {
        return Err(CardTextError::ParseError(format!(
            "unsupported copy-spell legendary-exception clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    let has_return_each_creature_that_isnt_list = sentence_words
        .starts_with(&["return", "each", "creature", "that", "isnt"])
        && sentence_words
            .iter()
            .filter(|word| **word == "or")
            .count()
            >= 1;
    if has_return_each_creature_that_isnt_list {
        return Err(CardTextError::ParseError(format!(
            "unsupported return-each-creature-that-isnt-list clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    if sentence_words.starts_with(&["round", "up", "each", "time"]) {
        // "Round up each time." is reminder text for half P/T copy effects.
        // The semantic behavior is represented by the underlying token-copy primitive.
        parser_trace("parse_effect_sentence:round-up-reminder", tokens);
        return Ok(Vec::new());
    }
    if let Some(stripped) = strip_labeled_conditional_prefix(tokens) {
        parser_trace("parse_effect_sentence:conditional-labeled", stripped);
        return parse_conditional_sentence(stripped);
    }
    if tokens.first().is_some_and(|token| token.is_word("then"))
        && tokens.get(1).is_some_and(|token| token.is_word("if"))
    {
        parser_trace("parse_effect_sentence:conditional-then", &tokens[1..]);
        return parse_conditional_sentence(&tokens[1..]);
    }
    if let Some(effects) = run_sentence_primitives(tokens, PRE_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }
    if tokens.first().is_some_and(|token| token.is_word("if")) {
        parser_trace("parse_effect_sentence:conditional", tokens);
        return parse_conditional_sentence(tokens);
    }
    if let Some(effects) = run_sentence_primitives(tokens, POST_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }
    if is_negated_untap_clause(&sentence_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported negated untap clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }

    if is_ring_tempts_sentence(tokens) {
        return Err(CardTextError::ParseError(format!(
            "unsupported ring tempts clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    if is_enters_as_copy_clause(&sentence_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported enters-as-copy replacement clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }

    parse_effect_chain(tokens)
}

fn is_enters_as_copy_clause(words: &[&str]) -> bool {
    let has_enter_before_as_copy = words
        .windows(3)
        .position(|window| window == ["as", "a", "copy"] || window == ["as", "an", "copy"])
        .is_some_and(|idx| {
            words[..idx]
                .iter()
                .any(|word| *word == "enter" || *word == "enters")
        });
    let has_enter_before_as_copy_no_article = words
        .windows(2)
        .position(|window| window == ["as", "copy"])
        .is_some_and(|idx| {
            words[..idx]
                .iter()
                .any(|word| *word == "enter" || *word == "enters")
        });
    has_enter_before_as_copy || has_enter_before_as_copy_no_article
}

fn strip_labeled_conditional_prefix(tokens: &[Token]) -> Option<&[Token]> {
    let if_idx = tokens.iter().position(|token| token.is_word("if"))?;
    if !(1..=3).contains(&if_idx) {
        return None;
    }
    if !tokens[..if_idx]
        .iter()
        .all(|token| matches!(token, Token::Word(_, _)))
    {
        return None;
    }

    let prefix_words = words(&tokens[..if_idx]);
    if prefix_words.is_empty() {
        return None;
    }
    let is_known_label = matches!(
        prefix_words[0],
        "adamant"
            | "addendum"
            | "ascend"
            | "battalion"
            | "delirium"
            | "domain"
            | "ferocious"
            | "formidable"
            | "hellbent"
            | "metalcraft"
            | "morbid"
            | "raid"
            | "revolt"
            | "spectacle"
            | "spell"
            | "surge"
            | "threshold"
            | "undergrowth"
    );
    if !is_known_label {
        return None;
    }

    Some(&tokens[if_idx..])
}

fn is_negated_untap_clause(words: &[&str]) -> bool {
    if words.len() < 3 {
        return false;
    }
    let has_untap = words.contains(&"untap") || words.contains(&"untaps");
    let has_negation = words.contains(&"doesnt")
        || words.contains(&"dont")
        || words.windows(2).any(|pair| pair == ["does", "not"])
        || words.windows(2).any(|pair| pair == ["do", "not"])
        || words.contains(&"cant")
        || words.windows(2).any(|pair| pair == ["can", "not"]);
    has_untap && has_negation
}

fn parse_token_copy_modifier_sentence(tokens: &[Token]) -> Option<EffectAst> {
    let filtered: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let is_exact_haste_followup = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
            | ["it", "has", "haste"]
            | ["they", "have", "haste"]
    );
    if is_exact_haste_followup {
        return Some(EffectAst::TokenCopyGainHasteUntilEot);
    }

    if filtered.starts_with(&["sacrifice", "it"]) || filtered.starts_with(&["sacrifice", "them"]) {
        let has_next_end_step = filtered
            .windows(6)
            .any(|window| window == ["at", "beginning", "of", "next", "end", "step"]);
        if has_next_end_step {
            return Some(EffectAst::TokenCopySacrificeAtNextEndStep);
        }
    }
    if filtered.starts_with(&["exile", "it"]) || filtered.starts_with(&["exile", "them"]) {
        let has_next_end_step = filtered.windows(6).any(|window| {
            window == ["at", "beginning", "of", "next", "end", "step"]
        });
        if has_next_end_step {
            return Some(EffectAst::TokenCopyExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice =
        filtered.starts_with(&["at", "the", "beginning", "of", "the", "end", "step", "sacrifice"])
            || filtered.starts_with(&[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "sacrifice",
            ])
            || filtered.starts_with(&[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "sacrifice",
            ]);
    if starts_delayed_end_step_sacrifice {
        return Some(EffectAst::TokenCopySacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile =
        filtered.starts_with(&["at", "the", "beginning", "of", "the", "end", "step", "exile"])
            || filtered.starts_with(&[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "exile",
            ])
            || filtered.starts_with(&[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "exile",
            ]);
    if starts_delayed_end_step_exile {
        return Some(EffectAst::TokenCopyExileAtNextEndStep);
    }

    None
}

fn parse_delayed_until_next_end_step_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut idx = 0usize;
    if !tokens.get(idx).is_some_and(|token| token.is_word("at")) {
        return Ok(None);
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }
    if !tokens
        .get(idx)
        .is_some_and(|token| token.is_word("beginning"))
    {
        return Ok(None);
    }
    idx += 1;
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return Ok(None);
    }
    idx += 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
        idx += 1;
    }

    let player = if tokens.get(idx).is_some_and(|token| token.is_word("your")) {
        idx += 1;
        PlayerFilter::You
    } else {
        PlayerFilter::Any
    };

    if tokens.get(idx).is_some_and(|token| token.is_word("next")) {
        if !tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("end"))
            || !tokens
                .get(idx + 2)
                .is_some_and(|token| token.is_word("step"))
        {
            return Ok(None);
        }
        idx += 3;
    } else {
        if !tokens.get(idx).is_some_and(|token| token.is_word("end"))
            || !tokens
                .get(idx + 1)
                .is_some_and(|token| token.is_word("step"))
        {
            return Ok(None);
        }
        idx += 2;
    }

    if matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }
    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(
            "missing delayed end-step effect clause".to_string(),
        ));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed end-step effect clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedUntilNextEndStep {
        player,
        effects: delayed_effects,
    }]))
}

fn parse_delayed_when_that_dies_this_turn_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 6 {
        return Ok(None);
    }
    if !matches!(clause_words.first().copied(), Some("when" | "if")) {
        return Ok(None);
    }
    if clause_words.get(1) != Some(&"that") {
        return Ok(None);
    }
    let Some(dies_idx) = clause_words.iter().position(|word| *word == "dies") else {
        return Ok(None);
    };
    if clause_words.get(dies_idx + 1) != Some(&"this")
        || clause_words.get(dies_idx + 2) != Some(&"turn")
    {
        return Ok(None);
    }

    let mut split_idx = tokens.len();
    let mut word_count = 0usize;
    for (idx, token) in tokens.iter().enumerate() {
        if token.as_word().is_some() {
            word_count += 1;
            if word_count == dies_idx + 3 {
                split_idx = idx + 1;
                break;
            }
        }
    }

    let mut remainder = &tokens[split_idx..];
    if matches!(remainder.first(), Some(Token::Comma(_))) {
        remainder = &remainder[1..];
    }
    let remainder = trim_commas(remainder);
    if remainder.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let delayed_effects = parse_effect_chain(&remainder)?;
    if delayed_effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing delayed dies-this-turn effect clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::DelayedWhenLastObjectDiesThisTurn {
        effects: delayed_effects,
    }]))
}

fn parse_each_player_choose_and_sacrifice_rest(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let all_words = words(tokens);
    if all_words.len() < 6 {
        return Ok(None);
    }

    if !all_words.starts_with(&["each", "player", "chooses"])
        && !all_words.starts_with(&["each", "player", "choose"])
    {
        return Ok(None);
    }

    let then_idx = tokens.iter().position(|token| token.is_word("then"));
    let Some(then_idx) = then_idx else {
        return Ok(None);
    };

    let after_then = &tokens[then_idx + 1..];
    let after_words = words(after_then);
    if !(after_words.starts_with(&["sacrifice", "the", "rest"])
        || after_words.starts_with(&["sacrifices", "the", "rest"]))
    {
        return Ok(None);
    }

    let choose_tokens = &tokens[3..then_idx];
    if choose_tokens.is_empty() {
        return Ok(None);
    }

    let from_idx = find_from_among(choose_tokens);
    let Some(from_idx) = from_idx else {
        return Ok(None);
    };

    let (list_tokens, base_tokens) = if from_idx == 0 {
        let list_start = find_list_start(&choose_tokens[2..])
            .map(|idx| idx + 2)
            .ok_or_else(|| {
                CardTextError::ParseError("missing choice list after 'from among'".to_string())
            })?;
        (
            choose_tokens.get(list_start..).unwrap_or_default(),
            choose_tokens.get(2..list_start).unwrap_or_default(),
        )
    } else {
        (
            choose_tokens.get(..from_idx).unwrap_or_default(),
            choose_tokens.get(from_idx + 2..).unwrap_or_default(),
        )
    };

    let list_tokens = trim_commas(list_tokens);
    let base_tokens = trim_commas(base_tokens);
    if list_tokens.is_empty() || base_tokens.is_empty() {
        return Ok(None);
    }

    let mut base_filter = parse_object_filter(&base_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported base filter in choose-and-sacrifice clause (clause: '{}')",
            all_words.join(" ")
        ))
    })?;
    if base_filter.controller.is_none() {
        base_filter.controller = Some(PlayerFilter::IteratedPlayer);
    }

    let mut effects = Vec::new();
    let keep_tag: TagKey = "keep".into();

    for segment in split_choose_list(&list_tokens) {
        let segment = strip_leading_articles(&segment);
        if segment.is_empty() {
            continue;
        }
        let segment_filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported choice filter in choose-and-sacrifice clause (clause: '{}')",
                all_words.join(" ")
            ))
        })?;
        let mut combined = merge_filters(&base_filter, &segment_filter);
        combined = combined.not_tagged(keep_tag.clone());
        effects.push(EffectAst::ChooseObjects {
            filter: combined,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::Implicit,
            tag: keep_tag.clone(),
        });
    }

    if effects.is_empty() {
        return Ok(None);
    }

    let sacrifice_filter = base_filter.clone().not_tagged(keep_tag.clone());
    effects.push(EffectAst::SacrificeAll {
        filter: sacrifice_filter,
        player: PlayerAst::Implicit,
    });

    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

fn find_from_among(tokens: &[Token]) -> Option<usize> {
    tokens.iter().enumerate().find_map(|(idx, token)| {
        if token.is_word("from") && tokens.get(idx + 1).is_some_and(|t| t.is_word("among")) {
            Some(idx)
        } else {
            None
        }
    })
}

fn find_list_start(tokens: &[Token]) -> Option<usize> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) {
            if tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .and_then(parse_card_type)
                .is_some()
            {
                return Some(idx);
            }
        } else if parse_card_type(word).is_some() {
            return Some(idx);
        }
    }
    None
}

fn trim_commas(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end && matches!(tokens[start], Token::Comma(_)) {
        start += 1;
    }
    while end > start && matches!(tokens[end - 1], Token::Comma(_)) {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

fn trim_edge_punctuation(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    let mut end = tokens.len();
    while start < end
        && matches!(
            tokens[start],
            Token::Comma(_) | Token::Period(_) | Token::Semicolon(_)
        )
    {
        start += 1;
    }
    while end > start
        && matches!(
            tokens[end - 1],
            Token::Comma(_) | Token::Period(_) | Token::Semicolon(_)
        )
    {
        end -= 1;
    }
    tokens[start..end].to_vec()
}

fn strip_leading_articles(tokens: &[Token]) -> Vec<Token> {
    let mut start = 0usize;
    while start < tokens.len() {
        if let Some(word) = tokens[start].as_word()
            && is_article(word)
        {
            start += 1;
            continue;
        }
        break;
    }
    tokens[start..].to_vec()
}

fn split_choose_list(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    for segment in split_on_and(tokens) {
        for sub in split_on_comma(&segment) {
            let trimmed = trim_commas(&sub);
            if !trimmed.is_empty() {
                segments.push(trimmed);
            }
        }
    }
    segments
}

fn merge_filters(base: &ObjectFilter, specific: &ObjectFilter) -> ObjectFilter {
    let mut merged = base.clone();

    if !specific.card_types.is_empty() {
        merged.card_types = specific.card_types.clone();
    }
    if !specific.all_card_types.is_empty() {
        merged.all_card_types = specific.all_card_types.clone();
    }
    if !specific.subtypes.is_empty() {
        merged.subtypes.extend(specific.subtypes.clone());
    }
    if !specific.excluded_card_types.is_empty() {
        merged
            .excluded_card_types
            .extend(specific.excluded_card_types.clone());
    }
    if !specific.excluded_colors.is_empty() {
        merged.excluded_colors = merged.excluded_colors.union(specific.excluded_colors);
    }
    if let Some(colors) = specific.colors {
        merged.colors = Some(
            merged
                .colors
                .map_or(colors, |existing| existing.union(colors)),
        );
    }
    if merged.zone.is_none() {
        merged.zone = specific.zone;
    }
    if merged.controller.is_none() {
        merged.controller = specific.controller.clone();
    }
    if merged.owner.is_none() {
        merged.owner = specific.owner.clone();
    }
    merged.other |= specific.other;
    merged.token |= specific.token;
    merged.nontoken |= specific.nontoken;
    merged.tapped |= specific.tapped;
    merged.untapped |= specific.untapped;
    merged.attacking |= specific.attacking;
    merged.blocking |= specific.blocking;
    merged.is_commander |= specific.is_commander;
    merged.colorless |= specific.colorless;
    merged.multicolored |= specific.multicolored;
    merged.monocolored |= specific.monocolored;

    if let Some(mv) = &specific.mana_value {
        merged.mana_value = Some(mv.clone());
    }
    if let Some(power) = &specific.power {
        merged.power = Some(power.clone());
        merged.power_reference = specific.power_reference;
    }
    if let Some(toughness) = &specific.toughness {
        merged.toughness = Some(toughness.clone());
        merged.toughness_reference = specific.toughness_reference;
    }
    if specific.has_mana_cost {
        merged.has_mana_cost = true;
    }
    if specific.no_x_in_cost {
        merged.no_x_in_cost = true;
    }
    if merged.with_counter.is_none() {
        merged.with_counter = specific.with_counter;
    }
    if merged.without_counter.is_none() {
        merged.without_counter = specific.without_counter;
    }
    if merged.alternative_cast.is_none() {
        merged.alternative_cast = specific.alternative_cast;
    }
    for ability_id in &specific.static_abilities {
        if !merged.static_abilities.contains(ability_id) {
            merged.static_abilities.push(*ability_id);
        }
    }
    for ability_id in &specific.excluded_static_abilities {
        if !merged.excluded_static_abilities.contains(ability_id) {
            merged.excluded_static_abilities.push(*ability_id);
        }
    }
    for marker in &specific.custom_static_markers {
        if !merged
            .custom_static_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.custom_static_markers.push(marker.clone());
        }
    }
    for marker in &specific.excluded_custom_static_markers {
        if !merged
            .excluded_custom_static_markers
            .iter()
            .any(|value| value.eq_ignore_ascii_case(marker))
        {
            merged.excluded_custom_static_markers.push(marker.clone());
        }
    }

    merged
}

fn parse_monstrosity_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("monstrosity") {
        return Ok(None);
    }

    let amount_tokens = &tokens[1..];
    let (amount, _) = parse_value(amount_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing monstrosity amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Monstrosity { amount }))
}

fn parse_for_each_counter_removed_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = words(tokens);
    if words_all.len() < 6 {
        return Ok(None);
    }
    if !words_all.starts_with(&["for", "each", "counter", "removed", "this", "way"]) {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[6..]
    };

    let remainder_words = words(remainder);
    if remainder_words.is_empty() {
        return Ok(None);
    }

    let gets_idx = remainder_words
        .iter()
        .position(|word| *word == "gets" || *word == "get");
    let Some(gets_idx) = gets_idx else {
        return Ok(None);
    };

    let subject_tokens = &remainder[..gets_idx];
    let subject = parse_subject(subject_tokens);
    let target = match subject {
        SubjectAst::This => TargetAst::Source(None),
        _ => return Ok(None),
    };

    let after_gets = &remainder[gets_idx + 1..];
    let modifier_token = after_gets.first().and_then(Token::as_word).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing power/toughness modifier (clause: '{}')",
            remainder_words.join(" ")
        ))
    })?;
    let (power, toughness) = parse_pt_modifier(modifier_token)?;

    let duration = if remainder_words.contains(&"until")
        && remainder_words.contains(&"end")
        && remainder_words.contains(&"turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    Ok(Some(EffectAst::PumpByLastEffect {
        power,
        toughness,
        target,
        duration,
    }))
}

fn is_exile_that_token_at_end_of_combat(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["exile", "that", "token", "at", "end", "of", "combat"]
        || words.as_slice() == ["exile", "that", "token", "at", "the", "end", "of", "combat"]
}

fn parse_take_extra_turn_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["take", "an", "extra", "turn", "after", "this", "one"] {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn {
            player: PlayerAst::You,
        }));
    }
    Ok(None)
}

fn is_ring_tempts_sentence(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["the", "ring", "tempts", "you"]
}

fn find_same_name_reference_span(
    tokens: &[Token],
) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("with") {
            continue;
        }
        if idx + 6 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
            && tokens[idx + 5].is_word("that")
        {
            return Ok(Some((idx, idx + 7)));
        }
        if idx + 5 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
            && tokens[idx + 4].is_word("that")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && tokens[idx + 1].is_word("the")
            && tokens[idx + 2].is_word("same")
            && tokens[idx + 3].is_word("name")
            && tokens[idx + 4].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        if idx + 3 < tokens.len()
            && tokens[idx + 1].is_word("same")
            && tokens[idx + 2].is_word("name")
            && tokens[idx + 3].is_word("as")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'that <object>' in same-name clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

fn strip_same_controller_reference(tokens: &[Token]) -> (Vec<Token>, bool) {
    let mut cleaned = Vec::with_capacity(tokens.len());
    let mut idx = 0usize;
    let mut same_controller = false;
    while idx < tokens.len() {
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("that")
            && tokens[idx + 1].is_word("player")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 2 < tokens.len()
            && tokens[idx].is_word("its")
            && tokens[idx + 1].is_word("controller")
            && (tokens[idx + 2].is_word("control") || tokens[idx + 2].is_word("controls"))
        {
            same_controller = true;
            idx += 3;
            continue;
        }
        if idx + 3 < tokens.len()
            && tokens[idx].is_word("that")
            && (tokens[idx + 1].is_word("creature")
                || tokens[idx + 1].is_word("permanent")
                || tokens[idx + 1].is_word("card"))
            && tokens[idx + 2].is_word("controller")
            && (tokens[idx + 3].is_word("control") || tokens[idx + 3].is_word("controls"))
        {
            same_controller = true;
            idx += 4;
            continue;
        }

        cleaned.push(tokens[idx].clone());
        idx += 1;
    }

    (cleaned, same_controller)
}

fn parse_same_name_fanout_filter(tokens: &[Token]) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((same_start, same_end)) = find_same_name_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..same_start]);
    filter_tokens.extend_from_slice(&tokens[same_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in same-name fanout clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let (cleaned_tokens, same_controller) = strip_same_controller_reference(&filter_tokens);
    let cleaned_tokens = trim_commas(&cleaned_tokens);
    if cleaned_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing base object filter in same-name fanout clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&cleaned_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported same-name fanout filter (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    if same_controller {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::SameControllerAsTagged,
        });
    }
    Ok(Some(filter))
}

fn parse_same_name_target_fanout_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let words_all = words(tokens);
    let Some(first_word) = words_all.first().copied() else {
        return Ok(None);
    };

    let deal_tokens: Option<&[Token]> = if first_word == "deal" {
        Some(tokens)
    } else if let Some((Verb::Deal, verb_idx)) = find_verb(tokens) {
        let subject_words: Vec<&str> = words(&tokens[..verb_idx])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        if is_source_reference_words(&subject_words) {
            Some(&tokens[verb_idx..])
        } else {
            None
        }
    } else {
        None
    };

    if let Some(deal_tokens) = deal_tokens {
        let deal_words = words(deal_tokens);
        let (amount, used) = if deal_words.get(1) == Some(&"that")
            && deal_words.get(2) == Some(&"much")
        {
            (Value::EventValue(EventValueSpec::Amount), 2usize)
        } else if let Some((value, used)) = parse_value(&deal_tokens[1..]) {
            (value, used)
        } else {
            return Ok(None);
        };

        let after_amount = &deal_tokens[1 + used..];
        if !after_amount.first().is_some_and(|token| token.is_word("damage")) {
            return Ok(None);
        }

        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }

        let split_idx = (0..target_tokens.len().saturating_sub(2)).find(|idx| {
            target_tokens[*idx].is_word("and")
                && target_tokens[*idx + 1].is_word("each")
                && target_tokens[*idx + 2].is_word("other")
        });
        let Some(split_idx) = split_idx else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }

        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: first_target,
            },
            EffectAst::DealDamageEach { amount, filter },
        ]));
    }

    let verb = first_word;
    if verb != "destroy" && verb != "exile" && verb != "return" {
        return Ok(None);
    }

    let and_idx = (0..tokens.len().saturating_sub(2)).find(|idx| {
        tokens[*idx].is_word("and")
            && tokens[*idx + 1].is_word("all")
            && tokens[*idx + 2].is_word("other")
    });
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };
    if and_idx <= 1 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&tokens[1..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }

    let second_clause_tokens = if verb == "return" {
        let to_idx = tokens
            .iter()
            .rposition(|token| token.is_word("to"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing return destination in same-name fanout clause (clause: '{}')",
                    words_all.join(" ")
                ))
            })?;
        if to_idx <= and_idx + 3 {
            return Err(CardTextError::ParseError(format!(
                "missing same-name filter before return destination (clause: '{}')",
                words_all.join(" ")
            )));
        }
        let destination_words = words(&tokens[to_idx + 1..]);
        if !destination_words.contains(&"hand") && !destination_words.contains(&"hands") {
            return Ok(None);
        }
        tokens[and_idx + 3..to_idx].to_vec()
    } else {
        tokens[and_idx + 3..].to_vec()
    };

    if second_clause_tokens.is_empty() {
        return Ok(None);
    }

    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let mut first_target = parse_target_phrase(&first_target_tokens)?;
    if verb == "return"
        && let Some(first_filter) = target_object_filter_mut(&mut first_target)
    {
        if first_filter.zone.is_none() {
            first_filter.zone = filter.zone;
            if first_filter.zone.is_none() && words_all.contains(&"graveyard") {
                first_filter.zone = Some(Zone::Graveyard);
            }
        }
        if first_filter.owner.is_none() {
            first_filter.owner = filter.owner.clone();
            if first_filter.owner.is_none() && words_all.windows(2).any(|window| window == ["your", "graveyard"]) {
                first_filter.owner = Some(PlayerFilter::You);
            }
        }
    }
    let first_effect = match verb {
        "destroy" => EffectAst::Destroy {
            target: first_target,
        },
        "exile" => {
            if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves {
                    target: first_target,
                }
            } else {
                EffectAst::Exile {
                    target: first_target,
                }
            }
        }
        "return" => EffectAst::ReturnToHand {
            target: first_target,
        },
        _ => unreachable!("verb already filtered"),
    };
    let second_effect = match verb {
        "destroy" => EffectAst::DestroyAll { filter },
        "exile" => {
            if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves {
                    target: TargetAst::Object(filter, None, None),
                }
            } else {
                EffectAst::ExileAll { filter }
            }
        }
        "return" => EffectAst::ReturnAllToHand { filter },
        _ => unreachable!("verb already filtered"),
    };

    Ok(Some(vec![first_effect, second_effect]))
}

fn find_shares_color_reference_span(tokens: &[Token]) -> Result<Option<(usize, usize)>, CardTextError> {
    for idx in 0..tokens.len() {
        if !tokens[idx].is_word("that") {
            continue;
        }
        if idx + 5 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
            && tokens[idx + 5].is_word("it")
        {
            return Ok(Some((idx, idx + 6)));
        }
        if idx + 4 < tokens.len()
            && (tokens[idx + 1].is_word("shares") || tokens[idx + 1].is_word("share"))
            && tokens[idx + 2].is_word("a")
            && tokens[idx + 3].is_word("color")
            && tokens[idx + 4].is_word("with")
        {
            return Err(CardTextError::ParseError(format!(
                "missing 'it' in shares-color clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }
    Ok(None)
}

fn parse_shared_color_fanout_filter(tokens: &[Token]) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some((share_start, share_end)) = find_shares_color_reference_span(tokens)? else {
        return Ok(None);
    };

    let mut filter_tokens = Vec::with_capacity(tokens.len());
    filter_tokens.extend_from_slice(&tokens[..share_start]);
    filter_tokens.extend_from_slice(&tokens[share_end..]);
    let filter_tokens = trim_commas(&filter_tokens);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing object phrase in shared-color fanout clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let mut filter = parse_object_filter(&filter_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported shared-color fanout filter (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::SharesColorWithTagged,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(IT_TAG),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    Ok(Some(filter))
}

fn parse_shared_color_target_fanout_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    let Some(verb) = words_all.first().copied() else {
        return Ok(None);
    };

    let and_idx = (0..tokens.len().saturating_sub(2)).find(|idx| {
        tokens[*idx].is_word("and")
            && tokens[*idx + 1].is_word("each")
            && tokens[*idx + 2].is_word("other")
    });
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };

    if matches!(verb, "destroy" | "exile") {
        if and_idx <= 1 {
            return Ok(None);
        }
        let first_target_tokens = trim_commas(&tokens[1..and_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = tokens[and_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        let first_effect = if verb == "destroy" {
            EffectAst::Destroy {
                target: first_target,
            }
        } else {
            EffectAst::Exile {
                target: first_target,
            }
        };
        let second_effect = if verb == "destroy" {
            EffectAst::DestroyAll { filter }
        } else {
            EffectAst::ExileAll { filter }
        };
        return Ok(Some(vec![first_effect, second_effect]));
    }

    if verb == "deal" {
        let (amount, used) = if words_all.get(1) == Some(&"that")
            && words_all.get(2) == Some(&"much")
        {
            (Value::EventValue(EventValueSpec::Amount), 2usize)
        } else if let Some((value, used)) = parse_value(&tokens[1..]) {
            (value, used)
        } else {
            return Ok(None);
        };

        let after_amount = &tokens[1 + used..];
        if !after_amount.first().is_some_and(|token| token.is_word("damage")) {
            return Ok(None);
        }
        let mut target_tokens = &after_amount[1..];
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Ok(None);
        }
        let split_idx = (0..target_tokens.len().saturating_sub(2)).find(|idx| {
            target_tokens[*idx].is_word("and")
                && target_tokens[*idx + 1].is_word("each")
                && target_tokens[*idx + 2].is_word("other")
        });
        let Some(split_idx) = split_idx else {
            return Ok(None);
        };
        let first_target_tokens = trim_commas(&target_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = target_tokens[split_idx + 3..].to_vec();
        if second_clause_tokens.is_empty() {
            return Ok(None);
        }
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;
        return Ok(Some(vec![
            EffectAst::DealDamage {
                amount: amount.clone(),
                target: first_target,
            },
            EffectAst::DealDamageEach { amount, filter },
        ]));
    }

    if verb == "prevent" {
        let mut idx = 1usize;
        if tokens.get(idx).is_some_and(|token| token.is_word("the")) {
            idx += 1;
        }
        if !tokens.get(idx).is_some_and(|token| token.is_word("next")) {
            return Ok(None);
        }
        idx += 1;
        let amount_token = tokens.get(idx).cloned().ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing prevent damage amount (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
        let Some((amount, _)) = parse_value(&[amount_token]) else {
            return Ok(None);
        };
        idx += 1;
        if !tokens.get(idx).is_some_and(|token| token.is_word("damage")) {
            return Ok(None);
        }
        idx += 1;
        if tokens.get(idx..idx + 4).is_none_or(|window| {
            !window[0].is_word("that")
                || !window[1].is_word("would")
                || !window[2].is_word("be")
                || !window[3].is_word("dealt")
        }) {
            return Ok(None);
        }
        idx += 4;
        if !tokens.get(idx).is_some_and(|token| token.is_word("to")) {
            return Ok(None);
        }
        idx += 1;

        let this_turn_rel = words(&tokens[idx..])
            .windows(2)
            .position(|window| window == ["this", "turn"]);
        let Some(this_turn_rel) = this_turn_rel else {
            return Ok(None);
        };
        let this_turn_abs = idx + this_turn_rel;
        if this_turn_abs + 2 != tokens.len() {
            return Ok(None);
        }

        let scope_tokens = &tokens[idx..this_turn_abs];
        let split_idx = (0..scope_tokens.len().saturating_sub(2)).find(|split| {
            scope_tokens[*split].is_word("and")
                && scope_tokens[*split + 1].is_word("each")
                && scope_tokens[*split + 2].is_word("other")
        });
        let Some(split_idx) = split_idx else {
            return Ok(None);
        };

        let first_target_tokens = trim_commas(&scope_tokens[..split_idx]);
        if first_target_tokens.is_empty()
            || !first_target_tokens
                .iter()
                .any(|token| token.is_word("target"))
        {
            return Ok(None);
        }
        let second_clause_tokens = scope_tokens[split_idx + 3..].to_vec();
        let Some(filter) = parse_shared_color_fanout_filter(&second_clause_tokens)? else {
            return Ok(None);
        };
        let first_target = parse_target_phrase(&first_target_tokens)?;

        return Ok(Some(vec![
            EffectAst::PreventDamage {
                amount: amount.clone(),
                target: first_target,
                duration: Until::EndOfTurn,
            },
            EffectAst::PreventDamageEach {
                amount,
                filter,
                duration: Until::EndOfTurn,
            },
        ]));
    }

    Ok(None)
}

fn parse_same_name_gets_fanout_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((verb, verb_idx)) = find_verb(tokens) else {
        return Ok(None);
    };
    if verb != Verb::Get || verb_idx == 0 || verb_idx + 1 >= tokens.len() {
        return Ok(None);
    }

    let subject_tokens = &tokens[..verb_idx];
    let and_idx = (0..subject_tokens.len().saturating_sub(2)).find(|idx| {
        subject_tokens[*idx].is_word("and")
            && subject_tokens[*idx + 1].is_word("all")
            && subject_tokens[*idx + 2].is_word("other")
    });
    let Some(and_idx) = and_idx else {
        return Ok(None);
    };
    if and_idx == 0 {
        return Ok(None);
    }

    let first_target_tokens = trim_commas(&subject_tokens[..and_idx]);
    if first_target_tokens.is_empty()
        || !first_target_tokens
            .iter()
            .any(|token| token.is_word("target"))
    {
        return Ok(None);
    }
    let second_clause_tokens = trim_commas(&subject_tokens[and_idx + 3..]);
    if second_clause_tokens.is_empty() {
        return Ok(None);
    }
    let Some(filter) = parse_same_name_fanout_filter(&second_clause_tokens)? else {
        return Ok(None);
    };

    let modifier_tokens = &tokens[verb_idx + 1..];
    let modifier_word = modifier_tokens
        .first()
        .and_then(Token::as_word)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing modifier in same-name gets clause (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;
    let (power, toughness) = parse_pt_modifier(modifier_word).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid power/toughness modifier in same-name gets clause (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    let modifier_words = words(modifier_tokens);
    let duration = if modifier_words.contains(&"until")
        && modifier_words.contains(&"end")
        && modifier_words.contains(&"turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    let target = parse_target_phrase(&first_target_tokens)?;
    Ok(Some(vec![
        EffectAst::Pump {
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            target,
            duration: duration.clone(),
        },
        EffectAst::PumpAll {
            filter,
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            duration,
        },
    ]))
}

fn parse_destroy_or_exile_all_split_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let verb = if words[0] == "destroy" {
        Some(Verb::Destroy)
    } else if words[0] == "exile" {
        Some(Verb::Exile)
    } else {
        None
    };
    let Some(verb) = verb else {
        return Ok(None);
    };
    if words[1] != "all" || !words.contains(&"and") {
        return Ok(None);
    }

    let mut raw_segments = Vec::new();
    let mut current = Vec::new();
    for token in &tokens[2..] {
        if token.is_word("and") || matches!(token, Token::Comma(_)) {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut effects = Vec::new();
    for mut segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segment.first().is_some_and(|token| token.is_word("all")) {
            segment.remove(0);
        }
        if segment.is_empty() {
            continue;
        }
        let filter = parse_object_filter(&segment, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in split all clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        let effect = match verb {
            Verb::Destroy => EffectAst::DestroyAll { filter },
            Verb::Exile => EffectAst::ExileAll { filter },
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported split all clause verb".to_string(),
                ));
            }
        };
        effects.push(effect);
    }

    if effects.len() >= 2 {
        return Ok(Some(effects));
    }
    Ok(None)
}

fn parse_exile_then_return_same_object_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    fn target_references_it_tag(target: &TargetAst) -> bool {
        match target {
            TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
            TargetAst::Object(filter, _, _) => filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag.as_str() == IT_TAG
                    && matches!(
                        constraint.relation,
                        TaggedOpbjectRelation::IsTaggedObject
                    )
            }),
            _ => false,
        }
    }

    let mut clause_tokens = tokens;
    if clause_tokens
        .first()
        .is_some_and(|token| token.is_word("you"))
        && clause_tokens
            .get(1)
            .is_some_and(|token| token.is_word("exile"))
    {
        clause_tokens = &clause_tokens[1..];
    }

    let words_all = words(clause_tokens);
    if words_all.first().copied() != Some("exile")
        || !words_all.contains(&"then")
        || !words_all.contains(&"return")
    {
        return Ok(None);
    }

    let split_idx = (0..clause_tokens.len().saturating_sub(2)).find(|idx| {
        matches!(clause_tokens[*idx], Token::Comma(_))
            && clause_tokens[*idx + 1].is_word("then")
            && clause_tokens[*idx + 2].is_word("return")
    });
    let Some(split_idx) = split_idx else {
        return Ok(None);
    };

    let first_clause = &clause_tokens[..split_idx];
    let second_clause = &clause_tokens[split_idx + 2..];
    if first_clause.is_empty() || second_clause.is_empty() {
        return Ok(None);
    }

    let mut first_effects = parse_effect_chain_inner(first_clause)?;
    if !first_effects
        .iter()
        .any(|effect| matches!(effect, EffectAst::Exile { .. }))
    {
        return Ok(None);
    }

    let second_effect = parse_effect_clause(second_clause)?;
    let rewritten_second = match second_effect {
        EffectAst::ReturnToBattlefield {
            target,
            tapped,
            controller,
        } if target_references_it_tag(&target) => {
            EffectAst::ReturnToBattlefield {
                target: TargetAst::Tagged(TagKey::from("exiled_0"), None),
                tapped,
                controller,
            }
        }
        EffectAst::ReturnToHand { target } if target_references_it_tag(&target) => {
            EffectAst::ReturnToHand {
                target: TargetAst::Tagged(TagKey::from("exiled_0"), None),
            }
        }
        _ => return Ok(None),
    };

    first_effects.push(rewritten_second);
    Ok(Some(first_effects))
}

fn parse_exile_up_to_one_each_target_type_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.len() < 6 || words[0] != "exile" {
        return Ok(None);
    }
    if !words.starts_with(&["exile", "up", "to", "one", "target"]) {
        return Ok(None);
    }
    // This primitive is for repeated clauses like:
    // "Exile up to one target artifact, up to one target creature, ..."
    // Not for a single disjunctive target like:
    // "Exile up to one target artifact, creature, or enchantment ..."
    let target_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| token.is_word("target").then_some(idx))
        .collect();
    if target_positions.len() < 2 {
        return Ok(None);
    }
    for pos in target_positions.iter().skip(1) {
        if *pos < 3
            || !tokens[*pos - 3].is_word("up")
            || !tokens[*pos - 2].is_word("to")
            || !tokens[*pos - 1].is_word("one")
        {
            return Ok(None);
        }
    }

    let mut raw_segments: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    for token in &tokens[1..] {
        if matches!(token, Token::Comma(_)) || token.is_word("and") || token.is_word("or") {
            if !current.is_empty() {
                raw_segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token.clone());
    }
    if !current.is_empty() {
        raw_segments.push(current);
    }

    let mut filters = Vec::new();
    for segment in raw_segments {
        let mut slice: &[Token] = &segment;
        if slice.len() >= 3
            && slice[0].is_word("up")
            && slice[1].is_word("to")
            && slice[2].is_word("one")
        {
            slice = &slice[3..];
        }
        if slice.first().is_some_and(|token| token.is_word("target")) {
            slice = &slice[1..];
        }
        if slice.is_empty() {
            continue;
        }

        let mut filter = parse_object_filter(slice, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported filter in 'exile up to one each target type' clause (clause: '{}')",
                words.join(" ")
            ))
        })?;
        if filter.controller.is_none() {
            // Keep this unrestricted to avoid implicit "you control" defaulting in ChooseObjects compilation.
            filter.controller = Some(PlayerFilter::Any);
        }
        filters.push(filter);
    }

    if filters.len() < 2 {
        return Ok(None);
    }

    let tag = TagKey::from("exiled_0");
    let mut effects: Vec<EffectAst> = filters
        .into_iter()
        .map(|filter| EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::up_to(1),
            player: PlayerAst::You,
            tag: tag.clone(),
        })
        .collect();
    effects.push(EffectAst::Exile {
        target: TargetAst::Tagged(tag, None),
    });

    Ok(Some(effects))
}

fn parse_look_at_hand_sentence(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.as_slice() == ["look", "at", "target", "players", "hand"]
        || words.as_slice() == ["look", "at", "target", "player", "hand"]
    {
        let target = TargetAst::Player(PlayerFilter::target_player(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    if words.as_slice() == ["look", "at", "target", "opponent", "hand"]
        || words.as_slice() == ["look", "at", "target", "opponents", "hand"]
    {
        let target =
            TargetAst::Player(PlayerFilter::target_opponent(), Some(TextSpan::synthetic()));
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    Ok(None)
}

fn parse_look_at_top_then_exile_one_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let starts_with_look_top = clause_words.starts_with(&["look", "at", "the", "top"])
        || clause_words.starts_with(&["look", "at", "top"]);
    if !starts_with_look_top {
        return Ok(None);
    }

    let Some(top_idx) = tokens.iter().position(|token| token.is_word("top")) else {
        return Ok(None);
    };
    let Some((count, used_count)) = parse_number(&tokens[top_idx + 1..]) else {
        return Ok(None);
    };
    let mut idx = top_idx + 1 + used_count;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
    {
        idx += 1;
    }
    if !tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        return Ok(None);
    }
    idx += 1;

    let Some(library_idx) = tokens[idx..]
        .iter()
        .position(|token| token.is_word("library"))
        .map(|offset| idx + offset)
    else {
        return Ok(None);
    };
    let owner_tokens = trim_commas(&tokens[idx..library_idx]);
    if owner_tokens.is_empty() {
        return Ok(None);
    }
    let player = match parse_subject(&owner_tokens) {
        SubjectAst::Player(player) => player,
        _ => return Ok(None),
    };

    let mut tail_tokens = trim_commas(&tokens[library_idx + 1..]).to_vec();
    while tail_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        tail_tokens.remove(0);
    }
    let tail_words = words(&tail_tokens);
    let looks_like_exile_one_of_looked = tail_words.starts_with(&["exile", "one", "of", "them"])
        || tail_words.starts_with(&["exile", "one", "of", "those"])
        || tail_words.starts_with(&["exile", "one", "of", "those", "cards"]);
    if !looks_like_exile_one_of_looked {
        return Ok(None);
    }

    let looked_tag = TagKey::from("looked_0");
    let chosen_tag = TagKey::from("chosen_0");
    let mut looked_filter = ObjectFilter::tagged(looked_tag.clone());
    looked_filter.zone = Some(Zone::Library);

    Ok(Some(vec![
        EffectAst::LookAtTopCards {
            player,
            count,
            tag: looked_tag,
        },
        EffectAst::ChooseObjects {
            filter: looked_filter,
            count: ChoiceCount::exactly(1),
            player: PlayerAst::You,
            tag: chosen_tag.clone(),
        },
        EffectAst::Exile {
            target: TargetAst::Tagged(chosen_tag, None),
        },
    ]))
}

fn parse_gain_life_equal_to_age_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    if words.contains(&"age") && words.contains(&"life") && words.contains(&"gain") {
        return Ok(Some(vec![EffectAst::GainLife {
            amount: Value::Fixed(0),
            player: PlayerAst::You,
        }]));
    }
    Ok(None)
}

fn parse_you_and_each_opponent_voted_with_you_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let pattern = [
        "you", "and", "each", "opponent", "who", "voted", "for", "a", "choice", "you", "voted",
        "for", "may", "scry",
    ];

    if words.len() < pattern.len() {
        return Ok(None);
    }

    if !words.starts_with(&pattern) {
        return Ok(None);
    }

    let scry_index = pattern.len() - 1;
    let value_tokens = &tokens[(scry_index + 1)..];
    let Some((count, _)) = parse_value(value_tokens) else {
        return Err(CardTextError::ParseError(format!(
            "missing scry count in vote-with-you clause (clause: '{}')",
            words.join(" ")
        )));
    };

    let you_effect = EffectAst::May {
        effects: vec![EffectAst::Scry {
            count: count.clone(),
            player: PlayerAst::You,
        }],
    };

    let opponent_effect = EffectAst::ForEachTaggedPlayer {
        tag: TagKey::from("voted_with_you"),
        effects: vec![EffectAst::May {
            effects: vec![EffectAst::Scry {
                count,
                player: PlayerAst::Implicit,
            }],
        }],
    };

    Ok(Some(vec![you_effect, opponent_effect]))
}

fn parse_gain_life_equal_to_power_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let Some(gain_idx) = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains")
    else {
        return Ok(None);
    };

    if words.get(gain_idx + 1) != Some(&"life")
        || words.get(gain_idx + 2) != Some(&"equal")
        || words.get(gain_idx + 3) != Some(&"to")
    {
        return Ok(None);
    }

    let tail = &words[gain_idx + 4..];
    let has_its_power = tail.windows(2).any(|pair| pair == ["its", "power"]);
    if !has_its_power {
        return Ok(None);
    }

    let subject = if gain_idx > 0 {
        Some(parse_subject(&tokens[..gain_idx]))
    } else {
        None
    };
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let amount = Value::PowerOf(Box::new(ChooseSpec::Tagged(TagKey::from(IT_TAG))));
    Ok(Some(vec![EffectAst::GainLife { amount, player }]))
}

fn parse_prevent_damage_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    let prefix = ["prevent", "all", "combat", "damage"];
    if !words.starts_with(&prefix) {
        return Ok(None);
    }

    let this_turn_positions: Vec<usize> = words
        .windows(2)
        .enumerate()
        .filter_map(|(idx, pair)| (pair == ["this", "turn"]).then_some(idx))
        .collect();
    if this_turn_positions.len() != 1 {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }
    let this_turn_idx = this_turn_positions[0];
    if this_turn_idx < prefix.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all-combat-damage duration (clause: '{}')",
            words.join(" ")
        )));
    }

    let mut core_words = Vec::with_capacity(words.len() - prefix.len() - 2);
    core_words.extend_from_slice(&words[prefix.len()..this_turn_idx]);
    core_words.extend_from_slice(&words[this_turn_idx + 2..]);
    let mut core_tokens = Vec::with_capacity(tokens.len() - prefix.len() - 2);
    core_tokens.extend_from_slice(&tokens[prefix.len()..this_turn_idx]);
    core_tokens.extend_from_slice(&tokens[this_turn_idx + 2..]);
    let core_words = core_words;
    let core_tokens = core_tokens;

    if core_words == ["that", "would", "be", "dealt"] {
        return Ok(Some(EffectAst::PreventAllCombatDamage {
            duration: Until::EndOfTurn,
        }));
    }

    if core_words.starts_with(&["that", "would", "be", "dealt", "by"]) {
        let source_tokens = &core_tokens[5..];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    if core_words.starts_with(&["that", "would", "be", "dealt", "to"]) {
        return parse_prevent_damage_target_scope(&core_tokens[5..], &words);
    }

    if let Some(would_idx) = core_words.iter().position(|word| *word == "would")
        && core_words.get(would_idx + 1) == Some(&"deal")
    {
        let source_tokens = &core_tokens[..would_idx];
        let source = parse_prevent_damage_source_target(source_tokens, &words)?;
        return Ok(Some(EffectAst::PreventAllCombatDamageFromSource {
            duration: Until::EndOfTurn,
            source,
        }));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all-combat-damage clause tail (clause: '{}')",
        words.join(" ")
    )))
}

fn parse_prevent_damage_source_target(
    tokens: &[Token],
    clause_words: &[&str],
) -> Result<TargetAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all source target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let source_words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let is_explicit_reference = source_words.contains(&"target")
        || source_words
            .first()
            .is_some_and(|word| matches!(*word, "this" | "that" | "it"));
    if !is_explicit_reference {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            source_words.join(" ")
        )));
    }

    let source = parse_target_phrase(tokens)?;
    match source {
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _) => Ok(source),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported prevent-all source target '{}'",
            words(tokens).join(" ")
        ))),
    }
}

fn parse_prevent_damage_target_scope(
    tokens: &[Token],
    clause_words: &[&str],
) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if target_words.as_slice() == ["player"] || target_words.as_slice() == ["players"] {
        return Ok(Some(EffectAst::PreventAllCombatDamageToPlayers {
            duration: Until::EndOfTurn,
        }));
    }
    if target_words.as_slice() == ["you"] {
        return Ok(Some(EffectAst::PreventAllCombatDamageToYou {
            duration: Until::EndOfTurn,
        }));
    }

    Err(CardTextError::ParseError(format!(
        "unsupported prevent-all target scope '{}'",
        words(tokens).join(" ")
    )))
}

fn parse_gain_x_plus_life_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let Some(gain_idx) = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains")
    else {
        return Ok(None);
    };

    if words.len() <= gain_idx + 4 {
        return Ok(None);
    }

    if words[gain_idx + 1] != "x" || words[gain_idx + 2] != "plus" {
        return Ok(None);
    }

    let number_token = tokens.get(gain_idx + 3).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words.join(" ")
        ))
    })?;
    let number_word = number_token
        .as_word()
        .ok_or_else(|| CardTextError::ParseError("missing life gain amount".to_string()))?;
    let (bonus, _) = parse_number(&[Token::Word(number_word.to_string(), TextSpan::synthetic())])
        .ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let subject_tokens = &tokens[..gain_idx];
    let player = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::Implicit,
    };

    let effects = vec![
        EffectAst::GainLife {
            amount: Value::X,
            player,
        },
        EffectAst::GainLife {
            amount: Value::Fixed(bonus as i32),
            player,
        },
    ];

    Ok(Some(effects))
}

fn parse_gain_ability_sentence(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = words(tokens);
    let looks_like_can_attack_no_defender = words.windows(2).any(|window| window == ["can", "attack"])
        && words.windows(2).any(|window| window == ["as", "though"])
        && words.contains(&"defender");
    if looks_like_can_attack_no_defender {
        return Ok(None);
    }
    let gain_idx = words
        .iter()
        .position(|word| matches!(*word, "gain" | "gains" | "has" | "have"));
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };
    let Some(gain_token_idx) = token_index_for_word_index(tokens, gain_idx) else {
        return Ok(None);
    };

    let after_gain = &words[gain_idx + 1..];
    if matches!(words[gain_idx], "gain" | "gains") {
        let starts_with_life = after_gain.first().is_some_and(|word| *word == "life");
        let starts_with_control = after_gain.first().is_some_and(|word| *word == "control");
        if starts_with_life || starts_with_control {
            return Ok(None);
        }
    }

    let leading_duration_phrase = if words.starts_with(&["until", "end", "of", "turn"]) {
        Some((4usize, Until::EndOfTurn))
    } else if words.starts_with(&["until", "your", "next", "turn"])
        || words.starts_with(&["until", "your", "next", "upkeep"])
    {
        Some((4usize, Until::YourNextTurn))
    } else if words.starts_with(&["until", "your", "next", "untap", "step"])
        || words.starts_with(&["during", "your", "next", "untap", "step"])
    {
        Some((5usize, Until::YourNextTurn))
    } else {
        None
    };
    let subject_start_word_idx = leading_duration_phrase
        .as_ref()
        .map(|(len, _)| *len)
        .unwrap_or(0);
    let subject_start_token_idx = if subject_start_word_idx == 0 {
        0usize
    } else if let Some(idx) = token_index_for_word_index(tokens, subject_start_word_idx) {
        idx
    } else {
        return Ok(None);
    };

    let duration_phrase = if let Some(idx) = after_gain
        .windows(4)
        .position(|window| window == ["until", "end", "of", "turn"])
    {
        Some((idx, 4usize, Until::EndOfTurn))
    } else if let Some(idx) = after_gain.windows(4).position(|window| {
        window == ["until", "your", "next", "turn"] || window == ["until", "your", "next", "upkeep"]
    }) {
        Some((idx, 4usize, Until::YourNextTurn))
    } else if let Some(idx) = after_gain.windows(5).position(|window| {
        window == ["until", "your", "next", "untap", "step"]
            || window == ["during", "your", "next", "untap", "step"]
    }) {
        Some((idx, 5usize, Until::YourNextTurn))
    } else if let Some(idx) = after_gain
        .windows(6)
        .position(|window| window == ["for", "as", "long", "as", "you", "control"])
    {
        // Consume the remainder of the phrase as the duration clause.
        Some((idx, after_gain.len().saturating_sub(idx), Until::YouStopControllingThis))
    } else {
        None
    };
    let duration = duration_phrase
        .as_ref()
        .map(|(_, _, duration)| duration.clone())
        .or_else(|| leading_duration_phrase.as_ref().map(|(_, duration)| duration.clone()))
        .unwrap_or(Until::Forever);
    let has_explicit_duration =
        duration_phrase.is_some() || leading_duration_phrase.as_ref().is_some();

    let mut trailing_tail_tokens: Vec<Token> = Vec::new();
    if let Some((start_rel, len_words, _)) = duration_phrase {
        let tail_word_idx = gain_idx + 1 + start_rel + len_words;
        if let Some(tail_token_idx) = token_index_for_word_index(tokens, tail_word_idx) {
            let mut tail_tokens = trim_commas(&tokens[tail_token_idx..]).to_vec();
            while tail_tokens
                .first()
                .is_some_and(|token| token.is_word("and") || token.is_word("then"))
            {
                tail_tokens.remove(0);
            }
            if !tail_tokens.is_empty() {
                trailing_tail_tokens = tail_tokens;
            }
        }
    }

    let ability_end_word_idx = duration_phrase
        .as_ref()
        .map(|(start_rel, _, _)| gain_idx + 1 + *start_rel);
    let ability_end_token_idx = if let Some(end_word_idx) = ability_end_word_idx {
        token_index_for_word_index(tokens, end_word_idx).unwrap_or(tokens.len())
    } else {
        tokens.len()
    };
    let ability_start_token_idx = gain_token_idx + 1;
    if ability_start_token_idx > ability_end_token_idx || ability_start_token_idx >= tokens.len() {
        return Ok(None);
    }
    let ability_tokens = trim_commas(&tokens[ability_start_token_idx..ability_end_token_idx]);

    let mut abilities = if let Some(actions) = parse_ability_line(&ability_tokens)
        .or_else(|| parse_choice_of_abilities(&ability_tokens))
    {
        reject_unimplemented_keyword_actions(&actions, &words.join(" "))?;
        actions
            .into_iter()
            .filter_map(keyword_action_to_static_ability)
        .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    if abilities.is_empty()
        && let Some(granted) =
            parse_granted_activated_or_triggered_ability_for_gain(&ability_tokens, &words)?
    {
        abilities.push(granted);
    }
    if abilities.is_empty() {
        return Ok(None);
    }

    // Check for "gets +X/+Y and gains/has ..." pattern - if there's a pump modifier
    // before the granting verb, extract it as a separate Pump/PumpAll effect.
    let before_gain = &words[subject_start_word_idx..gain_idx];
    let get_idx = before_gain.iter().position(|w| *w == "get" || *w == "gets");
    let pump_effect = if let Some(gi) = get_idx {
        let mod_word = before_gain.get(gi + 1).copied().unwrap_or("");
        if let Ok((power, toughness)) = parse_pt_modifier_values(mod_word) {
            Some((power, toughness, subject_start_word_idx + gi))
        } else {
            None
        }
    } else {
        None
    };
    let has_have_verb = matches!(words[gain_idx], "has" | "have");
    if has_have_verb && pump_effect.is_none() && !has_explicit_duration {
        return Ok(None);
    }

    // Determine the real subject (before "get"/"gets" if pump is present)
    let real_subject_end_word_idx = pump_effect
        .as_ref()
        .map(|(_, _, gi)| *gi)
        .unwrap_or(gain_idx);
    let real_subject_end_token_idx = token_index_for_word_index(tokens, real_subject_end_word_idx)
        .unwrap_or(gain_token_idx);
    if subject_start_token_idx >= real_subject_end_token_idx {
        return Ok(None);
    }
    let real_subject_tokens =
        trim_commas(&tokens[subject_start_token_idx..real_subject_end_token_idx]);

    let mut effects = Vec::new();

    // Check for pronoun subjects ("it", "they") that reference a prior tagged object.
    let real_subject_words: Vec<&str> = real_subject_tokens
        .iter()
        .filter_map(Token::as_word)
        .collect();
    let is_pronoun_subject =
        real_subject_words.as_slice() == ["it"] || real_subject_words.as_slice() == ["they"];
    if is_pronoun_subject {
        let span = span_from_tokens(&real_subject_tokens);
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span);
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
            });
        }
        effects.push(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        });
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let is_demonstrative_subject = real_subject_words
        .first()
        .is_some_and(|word| *word == "that" || *word == "those");
    if is_demonstrative_subject {
        let target = parse_target_phrase(&real_subject_tokens)?;
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target: target.clone(),
                duration: duration.clone(),
            });
        }
        effects.push(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        });
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    if before_gain.contains(&"target") {
        if let Some((power, toughness, _)) = pump_effect {
            let target = parse_target_phrase(&real_subject_tokens)?;
            effects.push(EffectAst::Pump {
                power,
                toughness,
                target,
                duration: duration.clone(),
            });
        }
        let target = parse_target_phrase(&real_subject_tokens)?;
        effects.push(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        });
        effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;
        return Ok(Some(effects));
    }

    let filter = parse_object_filter(&real_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in gain-ability clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    if let Some((power, toughness, _)) = pump_effect {
        effects.push(EffectAst::PumpAll {
            filter: filter.clone(),
            power,
            toughness,
            duration: duration.clone(),
        });
    }
    effects.push(EffectAst::GrantAbilitiesAll {
        filter,
        abilities,
        duration,
    });
    effects = append_gain_ability_trailing_effects(effects, &trailing_tail_tokens)?;

    Ok(Some(effects))
}

fn parse_granted_activated_or_triggered_ability_for_gain(
    ability_tokens: &[Token],
    clause_words: &[&str],
) -> Result<Option<StaticAbility>, CardTextError> {
    if ability_tokens.is_empty() {
        return Ok(None);
    }

    let has_colon = ability_tokens
        .iter()
        .any(|token| matches!(token, Token::Colon(_)));
    let looks_like_trigger = ability_tokens.first().is_some_and(|token| {
        token.is_word("when")
            || token.is_word("whenever")
            || (token.is_word("at")
                && ability_tokens
                    .get(1)
                    .is_some_and(|next| next.is_word("the")))
    });
    if !has_colon && !looks_like_trigger {
        return Ok(None);
    }

    let mut ability = if has_colon {
        let Some(parsed) = parse_activated_line(ability_tokens)? else {
            return Err(CardTextError::ParseError(format!(
                "unsupported granted activated/triggered ability clause (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        parsed.ability
    } else {
        match parse_triggered_line(ability_tokens)? {
            LineAst::Triggered {
                trigger,
                effects,
                once_each_turn,
            } => {
                let (compiled_effects, choices) = compile_trigger_effects(Some(&trigger), &effects)?;
                Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger: compile_trigger_spec(trigger),
                        effects: compiled_effects,
                        choices,
                        intervening_if: None,
                        once_each_turn,
                    }),
                    functional_zones: vec![Zone::Battlefield],
                    text: None,
                }
            }
            _ => {
                return Err(CardTextError::ParseError(format!(
                    "unsupported granted activated/triggered ability clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    };

    if ability.text.is_none() {
        ability.text = Some(words(ability_tokens).join(" "));
    }

    Ok(Some(StaticAbility::grant_object_ability_for_filter(
        ObjectFilter::source(),
        ability,
        words(ability_tokens).join(" "),
    )))
}

fn append_gain_ability_trailing_effects(
    mut effects: Vec<EffectAst>,
    trailing_tokens: &[Token],
) -> Result<Vec<EffectAst>, CardTextError> {
    if trailing_tokens.is_empty() {
        return Ok(effects);
    }

    let trimmed = trim_commas(trailing_tokens);
    if trimmed
        .first()
        .is_some_and(|token| token.is_word("unless"))
    {
        if let Some(unless_effect) = try_build_unless(effects, &trimmed, 0)? {
            return Ok(vec![unless_effect]);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing unless gain-ability clause (clause: '{}')",
            words(&trimmed).join(" ")
        )));
    }

    if let Ok(parsed_tail) = parse_effect_chain(&trimmed)
        && !parsed_tail.is_empty()
    {
        effects.extend(parsed_tail);
    }
    Ok(effects)
}

fn parse_choice_of_abilities(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let words = words(tokens);
    if words.len() < 6 || !words.starts_with(&["your", "choice", "of"]) {
        return None;
    }

    let or_idx = words.iter().position(|word| *word == "or")?;
    if or_idx <= 3 || or_idx + 1 >= words.len() {
        return None;
    }

    let left = vec![Token::Word(words[3].to_string(), TextSpan::synthetic())];
    let right = vec![Token::Word(words[or_idx + 1].to_string(), TextSpan::synthetic())];
    let left_action = parse_ability_phrase(&left)?;
    let right_action = parse_ability_phrase(&right)?;
    Some(vec![left_action, right_action])
}

fn parse_gain_ability_to_source_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let gain_idx = clause_words
        .iter()
        .position(|word| *word == "gain" || *word == "gains");
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };

    let subject_tokens = &tokens[..gain_idx];
    let subject_words: Vec<&str> = words(subject_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    if !is_source_reference_words(&subject_words) {
        return Ok(None);
    }

    let ability_tokens = &tokens[gain_idx + 1..];
    if let Some(ability) = parse_activated_line(ability_tokens)? {
        return Ok(Some(EffectAst::GrantAbilityToSource {
            ability: ability.ability,
        }));
    }

    Ok(None)
}

fn parse_search_library_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    let Some(search_idx) = tokens.iter().position(|token| token.is_word("search")) else {
        return Ok(None);
    };
    if tokens[..search_idx]
        .iter()
        .any(|token| token.is_word("unless"))
    {
        return Ok(None);
    }
    if tokens[..search_idx]
        .iter()
        .any(|token| token.is_word("may"))
    {
        return Ok(None);
    }

    let mut subject_tokens = &tokens[..search_idx];
    if subject_tokens
        .last()
        .is_some_and(|token| token.is_word("may"))
    {
        subject_tokens = &subject_tokens[..subject_tokens.len().saturating_sub(1)];
    }
    let player = match parse_subject(subject_tokens) {
        SubjectAst::Player(player) => player,
        _ => PlayerAst::You,
    };

    let search_tokens = &tokens[search_idx..];
    let search_words = words(search_tokens);
    if !search_words.starts_with(&["search", "your", "library", "for"]) {
        return Ok(None);
    }
    let mentions_nth_from_top = search_words
        .windows(4)
        .any(|window| window[1] == "from" && window[2] == "the" && window[3] == "top")
        && !search_words.windows(4).any(|window| window == ["on", "top", "of", "library"]);
    if mentions_nth_from_top {
        return Err(CardTextError::ParseError(format!(
            "unsupported search-library top-position clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let for_idx = search_tokens
        .iter()
        .position(|token| token.is_word("for"))
        .unwrap_or(3);
    let put_idx = search_tokens.iter().position(|token| token.is_word("put"));
    let Some(put_idx) = put_idx else {
        return Err(CardTextError::ParseError(format!(
            "missing put clause in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    };

    let filter_end = {
        let mut end = put_idx;
        for idx in (for_idx + 1)..put_idx {
            if !matches!(search_tokens[idx], Token::Comma(_)) {
                continue;
            }
            let next_word = search_tokens[idx + 1..].iter().find_map(Token::as_word);
            if matches!(next_word, Some("put" | "reveal" | "then")) {
                end = idx;
                break;
            }
        }
        if end == put_idx
            && let Some(idx) = search_tokens
                .iter()
                .position(|token| token.is_word("reveal") || token.is_word("then"))
        {
            end = end.min(idx);
        }
        end
    };

    if filter_end <= for_idx + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing search filter in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let count_tokens = &search_tokens[for_idx + 1..filter_end];
    let mut count = ChoiceCount::up_to(1);
    let mut count_used = 0usize;

    if count_tokens.len() >= 2
        && count_tokens[0].is_word("any")
        && count_tokens[1].is_word("number")
    {
        count = ChoiceCount::any_number();
        count_used = 2;
    } else if count_tokens.len() >= 2
        && count_tokens[0].is_word("up")
        && count_tokens[1].is_word("to")
    {
        if count_tokens
            .get(2)
            .is_some_and(|token| token.is_word("x"))
        {
            count = ChoiceCount::dynamic_x();
            count_used = 3;
        } else if let Some((value, used)) = parse_number(&count_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
            count_used = 2 + used;
        }
    } else if count_tokens.first().is_some_and(|token| token.is_word("x")) {
        count = ChoiceCount::dynamic_x();
        count_used = 1;
    } else if let Some((value, used)) = parse_number(count_tokens) {
        count = ChoiceCount::up_to(value as usize);
        count_used = used;
    }

    if count_used < count_tokens.len() && count_tokens[count_used].is_word("of") {
        count_used += 1;
    }

    let filter_start = for_idx + 1 + count_used;
    if filter_start >= filter_end {
        return Err(CardTextError::ParseError(format!(
            "missing object selector in search-library sentence (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let filter_tokens = &search_tokens[filter_start..filter_end];
    let filter_words: Vec<&str> = words(filter_tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let mut filter = if let Some(named_idx) = filter_words.iter().position(|word| *word == "named")
    {
        let name = filter_words
            .iter()
            .skip(named_idx + 1)
            .copied()
            .collect::<Vec<_>>()
            .join(" ");
        if name.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in named search clause (clause: '{}')",
                words_all.join(" ")
            )));
        }
        let base_words = &filter_words[..named_idx];
        let mut base_filter = if base_words.is_empty()
            || (base_words.len() == 1 && (base_words[0] == "card" || base_words[0] == "cards"))
        {
            ObjectFilter::default()
        } else {
            let base_tokens: Vec<Token> = base_words
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect();
            parse_object_filter(&base_tokens, false).map_err(|_| {
                CardTextError::ParseError(format!(
                    "unsupported named search filter in search-library sentence (clause: '{}')",
                    words_all.join(" ")
                ))
            })?
        };
        base_filter.name = Some(name);
        base_filter
    } else if filter_words.len() == 1 && (filter_words[0] == "card" || filter_words[0] == "cards") {
        ObjectFilter::default()
    } else {
        parse_object_filter(filter_tokens, false).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported search filter in search-library sentence (clause: '{}')",
                words_all.join(" ")
            ))
        })?
    };
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
        )
    }) && !filter.card_types.contains(&CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    if words_all.contains(&"mana") && words_all.contains(&"cost") {
        filter.has_mana_cost = true;
        filter.no_x_in_cost = true;
        let mut max_value: Option<u32> = None;
        for word in words_all.iter() {
            if let Ok(value) = word.parse::<u32>() {
                max_value = Some(max_value.map_or(value, |max| max.max(value)));
            }
        }
        if let Some(max_value) = max_value {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(max_value as i32));
        }
    }

    let destination = if words_all.contains(&"graveyard") {
        Zone::Graveyard
    } else if words_all.contains(&"hand") {
        Zone::Hand
    } else if words_all.contains(&"top") && words_all.contains(&"library") {
        Zone::Library
    } else {
        Zone::Battlefield
    };

    let reveal = words_all.contains(&"reveal");
    let shuffle = words_all.contains(&"shuffle");
    let split_battlefield_and_hand = words_all.contains(&"battlefield")
        && words_all.contains(&"hand")
        && words_all.contains(&"other")
        && words_all.contains(&"one");
    let mut effects = if split_battlefield_and_hand {
        let battlefield_tapped = words_all.contains(&"tapped");
        vec![
            EffectAst::SearchLibrary {
                filter: filter.clone(),
                destination: Zone::Battlefield,
                player,
                reveal,
                shuffle: false,
                count: ChoiceCount::up_to(1),
                tapped: battlefield_tapped,
            },
            EffectAst::SearchLibrary {
                filter,
                destination: Zone::Hand,
                player,
                reveal,
                shuffle,
                count: ChoiceCount::up_to(1),
                tapped: false,
            },
        ]
    } else {
        let battlefield_tapped = destination == Zone::Battlefield && words_all.contains(&"tapped");
        vec![EffectAst::SearchLibrary {
            filter,
            destination,
            player,
            reveal,
            shuffle,
            count,
            tapped: battlefield_tapped,
        }]
    };

    if let Some(and_idx) = search_tokens
        .iter()
        .enumerate()
        .skip(put_idx)
        .find_map(|(idx, token)| token.is_word("and").then_some(idx))
    {
        let trailing_tokens = trim_commas(&search_tokens[and_idx + 1..]);
        if !trailing_tokens.is_empty() {
            let trailing_words = words(&trailing_tokens);
            let starts_with_life_clause = trailing_words.starts_with(&["you", "gain"])
                || trailing_words.starts_with(&["target", "player", "gains"])
                || trailing_words.starts_with(&["target", "player", "gain"]);
            if starts_with_life_clause {
                let trailing_effect = parse_effect_clause(&trailing_tokens)?;
                effects.push(trailing_effect);
            }
        }
    }

    Ok(Some(effects))
}

fn parse_shuffle_graveyard_into_library_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = words(&clause_tokens);
    if !clause_words
        .iter()
        .any(|word| *word == "shuffle" || *word == "shuffles")
        || !clause_words.contains(&"graveyard")
        || !clause_words.contains(&"library")
    {
        return Ok(None);
    }

    let Some(shuffle_idx) = clause_tokens
        .iter()
        .position(|token| token.is_word("shuffle") || token.is_word("shuffles"))
    else {
        return Ok(None);
    };

    // Keep this primitive focused on shuffle-led clauses so we don't swallow
    // earlier effects in chains like "... then shuffle your graveyard ...".
    if shuffle_idx > 3 {
        return Ok(None);
    }

    let subject_tokens = trim_commas(&clause_tokens[..shuffle_idx]);
    let each_player_subject = {
        let subject_words = words(&subject_tokens);
        subject_words.starts_with(&["each", "player"]) || subject_words.starts_with(&["each", "players"])
    };
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else if each_player_subject {
        SubjectAst::Player(PlayerAst::Implicit)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let body_tokens = trim_commas(&clause_tokens[shuffle_idx + 1..]);
    if body_tokens.is_empty() {
        return Ok(None);
    }

    let Some(into_idx) = body_tokens.iter().position(|token| token.is_word("into")) else {
        return Ok(None);
    };
    if into_idx == 0 {
        return Ok(None);
    }

    let destination_tokens = trim_commas(&body_tokens[into_idx + 1..]);
    let destination_words = words(&destination_tokens);
    if !destination_words.contains(&"library") {
        return Ok(None);
    }
    let trailing_tokens = destination_tokens
        .iter()
        .position(|token| token.is_word("library") || token.is_word("libraries"))
        .map(|idx| trim_commas(&destination_tokens[idx + 1..]).to_vec())
        .unwrap_or_default();
    let append_trailing = |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
        if trailing_tokens.is_empty() {
            return Ok(Some(effects));
        }
        let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
        if each_player_subject {
            for effect in &mut trailing_effects {
                maybe_apply_carried_player(effect, CarryContext::ForEachPlayer);
            }
        } else {
            for effect in &mut trailing_effects {
                maybe_apply_carried_player(effect, CarryContext::Player(player));
            }
        }
        effects.extend(trailing_effects);
        Ok(Some(effects))
    };

    let target_tokens = trim_commas(&body_tokens[..into_idx]);
    if target_tokens.is_empty() {
        return Ok(None);
    }
    let target_words = words(&target_tokens);
    if !target_words.contains(&"graveyard") {
        return Ok(None);
    }

    let has_target_selector = target_words.contains(&"target");
    if !has_target_selector {
        let mut effects = Vec::new();
        if each_player_subject && target_words.contains(&"hand") {
            let mut hand_filter = ObjectFilter::default();
            hand_filter.zone = Some(Zone::Hand);
            hand_filter.owner = Some(PlayerFilter::IteratedPlayer);
            effects.push(EffectAst::MoveToZone {
                target: TargetAst::Object(hand_filter, None, None),
                zone: Zone::Library,
                to_top: false,
                battlefield_controller: ReturnControllerAst::Preserve,
            });
        }
        effects.push(EffectAst::ShuffleGraveyardIntoLibrary { player });
        if each_player_subject {
            return append_trailing(vec![EffectAst::ForEachPlayer { effects }]);
        }
        return append_trailing(effects);
    }

    let mut target = parse_target_phrase(&target_tokens)?;
    apply_shuffle_subject_graveyard_owner_context(&mut target, subject);

    append_trailing(vec![
        EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: false,
            battlefield_controller: ReturnControllerAst::Preserve,
        },
        EffectAst::ShuffleLibrary { player },
    ])
}

fn parse_exile_hand_and_graveyard_bundle_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut clause_tokens = trim_commas(tokens);
    while clause_tokens
        .first()
        .is_some_and(|token| token.is_word("then") || token.is_word("and"))
    {
        clause_tokens.remove(0);
    }
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    let clause_words = words(&clause_tokens);
    if !clause_words.starts_with(&["exile", "all", "cards", "from"]) {
        return Ok(None);
    }
    if !clause_words.contains(&"hand") && !clause_words.contains(&"hands") {
        return Ok(None);
    }
    if !clause_words.contains(&"graveyard") && !clause_words.contains(&"graveyards") {
        return Ok(None);
    }

    let first_zone_idx = clause_words
        .iter()
        .position(|word| matches!(*word, "hand" | "hands" | "graveyard" | "graveyards"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing zone in exile hand+graveyard clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    if first_zone_idx <= 4 {
        return Ok(None);
    }

    let owner_words = &clause_words[4..first_zone_idx];
    let owner = match owner_words {
        ["target", "player"] | ["target", "players"] => PlayerFilter::target_player(),
        ["target", "opponent"] | ["target", "opponents"] => PlayerFilter::target_opponent(),
        ["your"] => PlayerFilter::You,
        _ => return Ok(None),
    };

    let Some(first_zone) = parse_zone_word(clause_words[first_zone_idx]) else {
        return Ok(None);
    };
    if !matches!(first_zone, Zone::Hand | Zone::Graveyard) {
        return Ok(None);
    }

    let Some(and_word) = clause_words.get(first_zone_idx + 1) else {
        return Ok(None);
    };
    if *and_word != "and" {
        return Ok(None);
    }

    let mut second_zone_idx = first_zone_idx + 2;
    while clause_words
        .get(second_zone_idx)
        .is_some_and(|word| matches!(*word, "all" | "cards" | "from"))
    {
        second_zone_idx += 1;
    }
    let Some(second_zone_word) = clause_words.get(second_zone_idx) else {
        return Ok(None);
    };
    if clause_words.len() != second_zone_idx + 1 {
        return Ok(None);
    }
    let Some(second_zone) = parse_zone_word(second_zone_word) else {
        return Ok(None);
    };
    if !matches!(second_zone, Zone::Hand | Zone::Graveyard) || second_zone == first_zone {
        return Ok(None);
    }

    let mut first_filter = ObjectFilter::default().in_zone(first_zone);
    first_filter.owner = Some(owner.clone());
    let mut second_filter = ObjectFilter::default().in_zone(second_zone);
    second_filter.owner = Some(owner);

    Ok(Some(vec![
        EffectAst::ExileAll {
            filter: first_filter,
        },
        EffectAst::ExileAll {
            filter: second_filter,
        },
    ]))
}

fn parse_for_each_exiled_this_way_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    if !words_all.starts_with(&["for", "each", "permanent", "exiled", "this", "way"]) {
        return Ok(None);
    }
    if !words_all.contains(&"shares")
        || !words_all.contains(&"card")
        || !words_all.contains(&"type")
        || !words_all.contains(&"library")
        || !words_all.contains(&"battlefield")
    {
        return Ok(None);
    }

    let filter_tokens = tokenize_line("a permanent that shares a card type with it", 0);
    let filter = parse_object_filter(&filter_tokens, false)?;

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: "exiled_0".into(),
        effects: vec![EffectAst::SearchLibrary {
            filter,
            destination: Zone::Battlefield,
            player: PlayerAst::Implicit,
            reveal: true,
            shuffle: true,
            count: ChoiceCount::up_to(1),
            tapped: false,
        }],
    }]))
}

fn parse_each_player_put_permanent_cards_exiled_with_source_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    let starts_with_each_player_turns_face_up = words_all
        .starts_with(&["each", "player", "turns", "face", "up", "all", "cards"]);
    if !starts_with_each_player_turns_face_up {
        return Ok(None);
    }
    let has_exiled_with_this = words_all
        .windows(3)
        .any(|window| window == ["exiled", "with", "this"]);
    if !has_exiled_with_this {
        return Ok(None);
    }
    let has_puts_all_permanent_cards = words_all
        .windows(5)
        .any(|window| window == ["then", "puts", "all", "permanent", "cards"]);
    let has_among_them_onto_battlefield = words_all
        .windows(4)
        .any(|window| window == ["among", "them", "onto", "battlefield"])
        || words_all
            .windows(5)
            .any(|window| window == ["among", "them", "onto", "the", "battlefield"]);
    if !has_puts_all_permanent_cards || !has_among_them_onto_battlefield {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![EffectAst::ReturnAllToBattlefield {
            filter,
            tapped: false,
        }],
    }]))
}

fn parse_for_each_destroyed_this_way_sentence(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words_all = words(tokens);
    if !words_all.starts_with(&["for", "each"]) {
        return Ok(None);
    }
    let refers_to_destroyed = words_all
        .windows(3)
        .any(|window| window == ["destroyed", "this", "way"]);
    let refers_to_died = words_all
        .windows(3)
        .any(|window| window == ["died", "this", "way"]);
    if !refers_to_destroyed && !refers_to_died {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing comma after 'for each ... this way' clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
    let effect_tokens = trim_commas(&tokens[comma_idx + 1..]);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    let effects = parse_effect_chain(&effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

fn parse_earthbend_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.first().copied() != Some("earthbend") {
        return Ok(None);
    }

    let count_tokens = &tokens[1..];
    let (count, _) = parse_number(count_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing earthbend count (clause: '{}')",
            words.join(" ")
        ))
    })?;

    Ok(Some(EffectAst::Earthbend { counters: count }))
}

fn parse_enchant_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.is_empty() || words[0] != "enchant" {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = parse_object_filter(remaining, false)?;
    Ok(Some(EffectAst::Enchant { filter }))
}

fn parse_cant_effect_sentence(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((duration, clause_tokens)) = parse_restriction_duration(tokens)? else {
        return Ok(None);
    };
    if clause_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "restriction clause missing body".to_string(),
        ));
    }
    if find_negation_span(&clause_tokens).is_none() {
        return Ok(None);
    }

    let Some(restrictions) = parse_cant_restrictions(&clause_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported restriction clause body (clause: '{}')",
            words(&clause_tokens).join(" ")
        )));
    };

    let mut target: Option<TargetAst> = None;
    let mut effects = Vec::new();
    for parsed in restrictions {
        if let Some(parsed_target) = parsed.target {
            if let Some(existing) = &target {
                if *existing != parsed_target {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported mixed restriction targets (clause: '{}')",
                        words(&clause_tokens).join(" ")
                    )));
                }
            } else {
                target = Some(parsed_target);
            }
        }
        effects.push(EffectAst::Cant {
            restriction: parsed.restriction,
            duration: duration.clone(),
        });
    }
    if let Some(target) = target {
        effects.insert(0, EffectAst::TargetOnly { target });
    }

    Ok(Some(effects))
}

fn parse_restriction_duration(
    tokens: &[Token],
) -> Result<Option<(crate::effect::Until, Vec<Token>)>, CardTextError> {
    use crate::effect::Until;

    let all_words = words(tokens);
    if all_words.len() < 4 {
        return Ok(None);
    }

    if all_words.starts_with(&["until", "end", "of", "turn"]) {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let remainder = if let Some(idx) = comma_idx {
            &tokens[idx + 1..]
        } else {
            &tokens[4..]
        };
        return Ok(Some((Until::EndOfTurn, trim_commas(remainder))));
    }

    if all_words.starts_with(&["until", "your", "next", "turn"]) {
        let comma_idx = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)));
        let remainder = if let Some(idx) = comma_idx {
            &tokens[idx + 1..]
        } else {
            &tokens[4..]
        };
        return Ok(Some((Until::YourNextTurn, trim_commas(remainder))));
    }

    if all_words.starts_with(&["for", "as", "long", "as"]) {
        let as_long_duration = all_words.contains(&"you")
            && all_words.contains(&"control")
            && (all_words.contains(&"this")
                || all_words.contains(&"thiss")
                || all_words.contains(&"source")
                || all_words.contains(&"creature")
                || all_words.contains(&"permanent"));
        if !as_long_duration {
            return Ok(None);
        }
        let Some(comma_idx) = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        else {
            return Err(CardTextError::ParseError(
                "missing comma after duration prefix".to_string(),
            ));
        };
        let remainder = trim_commas(&tokens[comma_idx + 1..]);
        return Ok(Some((Until::YouStopControllingThis, remainder)));
    }

    if all_words.ends_with(&["until", "end", "of", "turn"]) {
        let end_idx = tokens
            .iter()
            .rposition(|token| token.is_word("until"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..end_idx]);
        return Ok(Some((Until::EndOfTurn, remainder)));
    }

    if all_words.ends_with(&["until", "your", "next", "turn"])
        || (all_words.ends_with(&["next", "turn"]) && all_words.contains(&"until"))
    {
        let end_idx = tokens
            .iter()
            .rposition(|token| token.is_word("until"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..end_idx]);
        return Ok(Some((Until::YourNextTurn, remainder)));
    }

    if all_words.ends_with(&["during", "your", "next", "untap", "step"]) {
        let during_idx = tokens
            .iter()
            .rposition(|token| token.is_word("during"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..during_idx]);
        if !remainder.is_empty() {
            return Ok(Some((Until::YourNextTurn, remainder)));
        }
    }

    if all_words.ends_with(&["during", "its", "controller", "next", "untap", "step"])
        || all_words.ends_with(&["during", "its", "controllers", "next", "untap", "step"])
        || all_words.ends_with(&["during", "their", "controller", "next", "untap", "step"])
        || all_words.ends_with(&["during", "their", "controllers", "next", "untap", "step"])
    {
        let during_idx = tokens
            .iter()
            .rposition(|token| token.is_word("during"))
            .unwrap_or(tokens.len());
        let remainder = trim_commas(&tokens[..during_idx]);
        if !remainder.is_empty() {
            return Ok(Some((Until::ControllersNextUntapStep, remainder)));
        }
    }

    let suffix_idx = tokens.windows(4).position(|window| {
        window[0].is_word("for")
            && window[1].is_word("as")
            && window[2].is_word("long")
            && window[3].is_word("as")
    });
    if let Some(idx) = suffix_idx {
        let suffix_words = words(&tokens[idx..]);
        let as_long_duration = suffix_words.contains(&"you")
            && suffix_words.contains(&"control")
            && (suffix_words.contains(&"this")
                || suffix_words.contains(&"thiss")
                || suffix_words.contains(&"source")
                || suffix_words.contains(&"creature")
                || suffix_words.contains(&"permanent"));
        if as_long_duration {
            let remainder = trim_commas(&tokens[..idx]);
            return Ok(Some((Until::YouStopControllingThis, remainder)));
        }
    }

    let has_this_turn = all_words
        .windows(2)
        .any(|window| window == ["this", "turn"]);
    if has_this_turn {
        let mut cleaned = Vec::new();
        let mut idx = 0usize;
        while idx < tokens.len() {
            if tokens[idx].is_word("this")
                && tokens
                    .get(idx + 1)
                    .is_some_and(|token| token.is_word("turn"))
            {
                idx += 2;
                continue;
            }
            cleaned.push(tokens[idx].clone());
            idx += 1;
        }
        let remainder = trim_commas(&cleaned).to_vec();
        if !remainder.is_empty() {
            return Ok(Some((Until::EndOfTurn, remainder)));
        }
    }

    Ok(None)
}

fn parse_play_from_graveyard_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 8 || !line_words.starts_with(&["until", "end", "of", "turn"]) {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[4..]
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = [
        "you",
        "may",
        "play",
        "lands",
        "and",
        "cast",
        "spells",
        "from",
        "your",
        "graveyard",
    ];

    if remaining_words == expected {
        return Ok(Some(EffectAst::PlayFromGraveyardUntilEot {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}

fn parse_exile_instead_of_graveyard_sentence(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.first().copied() != Some("if") {
        return Ok(None);
    }

    let has_graveyard_clause = line_words
        .windows(4)
        .any(|w| w == ["into", "your", "graveyard", "from"])
        || line_words
            .windows(3)
            .any(|w| w == ["your", "graveyard", "from"])
        || (line_words.contains(&"your") && line_words.contains(&"graveyard"));
    let has_would_put = line_words
        .windows(4)
        .any(|w| w == ["card", "would", "be", "put"]);
    let has_this_turn = line_words.contains(&"this") && line_words.contains(&"turn");
    if !has_graveyard_clause || !has_would_put || !has_this_turn {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        return Ok(None);
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    let expected = ["exile", "that", "card", "instead"];
    if remaining_words == expected {
        return Ok(Some(EffectAst::ExileInsteadOfGraveyardThisTurn {
            player: PlayerAst::You,
        }));
    }

    Ok(None)
}

fn parse_scryfall_mana_cost(raw: &str) -> Result<ManaCost, CardTextError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "—" {
        return Ok(ManaCost::new());
    }

    let mut pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut current = String::new();
    let mut in_brace = false;

    for ch in trimmed.chars() {
        if ch == '{' {
            in_brace = true;
            current.clear();
            continue;
        }
        if ch == '}' {
            if !in_brace {
                continue;
            }
            in_brace = false;
            if current.is_empty() {
                continue;
            }
            let alternatives = parse_mana_symbol_group(&current)?;
            if !alternatives.is_empty() {
                pips.push(alternatives);
            }
            continue;
        }
        if in_brace {
            current.push(ch);
        }
    }

    Ok(ManaCost::from_pips(pips))
}

fn parse_mana_symbol_group(raw: &str) -> Result<Vec<ManaSymbol>, CardTextError> {
    let mut alternatives = Vec::new();
    for part in raw.split('/') {
        let symbol = parse_mana_symbol(part)?;
        alternatives.push(symbol);
    }
    Ok(alternatives)
}

fn parse_mana_symbol(part: &str) -> Result<ManaSymbol, CardTextError> {
    let upper = part.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return Err(CardTextError::ParseError("empty mana symbol".to_string()));
    }

    if upper.chars().all(|c| c.is_ascii_digit()) {
        let value = upper.parse::<u8>().map_err(|_| {
            CardTextError::ParseError(format!("invalid generic mana symbol '{part}'"))
        })?;
        return Ok(ManaSymbol::Generic(value));
    }

    match upper.as_str() {
        "W" => Ok(ManaSymbol::White),
        "U" => Ok(ManaSymbol::Blue),
        "B" => Ok(ManaSymbol::Black),
        "R" => Ok(ManaSymbol::Red),
        "G" => Ok(ManaSymbol::Green),
        "C" => Ok(ManaSymbol::Colorless),
        "S" => Ok(ManaSymbol::Snow),
        "X" => Ok(ManaSymbol::X),
        "P" => Ok(ManaSymbol::Life(2)),
        _ => Err(CardTextError::ParseError(format!(
            "unsupported mana symbol '{part}'"
        ))),
    }
}

fn parse_type_line(
    raw: &str,
) -> Result<(Vec<Supertype>, Vec<CardType>, Vec<Subtype>), CardTextError> {
    let mut supertypes = Vec::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::new();

    let parts: Vec<&str> = raw.split('—').collect();
    let left = parts[0].trim();
    let right = parts.get(1).map(|s| s.trim());

    for word in left.split_whitespace() {
        if let Some(supertype) = parse_supertype_word(word) {
            supertypes.push(supertype);
            continue;
        }
        if let Some(card_type) = parse_card_type(&word.to_ascii_lowercase()) {
            card_types.push(card_type);
        }
    }

    if let Some(right) = right {
        for word in right.split_whitespace() {
            if let Some(subtype) = parse_subtype_word(word) {
                subtypes.push(subtype);
            }
        }
    }

    Ok((supertypes, card_types, subtypes))
}

fn parse_supertype_word(word: &str) -> Option<Supertype> {
    match word.to_ascii_lowercase().as_str() {
        "basic" => Some(Supertype::Basic),
        "legendary" => Some(Supertype::Legendary),
        "snow" => Some(Supertype::Snow),
        "world" => Some(Supertype::World),
        _ => None,
    }
}

fn parse_subtype_word(word: &str) -> Option<Subtype> {
    match word.to_ascii_lowercase().as_str() {
        "plains" => Some(Subtype::Plains),
        "island" => Some(Subtype::Island),
        "swamp" => Some(Subtype::Swamp),
        "mountain" => Some(Subtype::Mountain),
        "forest" => Some(Subtype::Forest),
        "urzas" => Some(Subtype::Urzas),
        "cave" | "caves" => Some(Subtype::Cave),
        "gate" | "gates" => Some(Subtype::Gate),
        "locus" | "loci" => Some(Subtype::Locus),
        "advisor" => Some(Subtype::Advisor),
        "ally" | "allies" => Some(Subtype::Ally),
        "alien" | "aliens" => Some(Subtype::Alien),
        "angel" => Some(Subtype::Angel),
        "ape" => Some(Subtype::Ape),
        "army" | "armies" => Some(Subtype::Army),
        "archer" => Some(Subtype::Archer),
        "artificer" => Some(Subtype::Artificer),
        "assassin" => Some(Subtype::Assassin),
        "astartes" => Some(Subtype::Astartes),
        "avatar" => Some(Subtype::Avatar),
        "barbarian" => Some(Subtype::Barbarian),
        "bard" => Some(Subtype::Bard),
        "bat" | "bats" => Some(Subtype::Bat),
        "bear" => Some(Subtype::Bear),
        "beast" => Some(Subtype::Beast),
        "berserker" => Some(Subtype::Berserker),
        "bird" => Some(Subtype::Bird),
        "boar" => Some(Subtype::Boar),
        "cat" => Some(Subtype::Cat),
        "centaur" => Some(Subtype::Centaur),
        "citizen" | "citizens" => Some(Subtype::Citizen),
        "coward" | "cowards" => Some(Subtype::Coward),
        "changeling" => Some(Subtype::Changeling),
        "cleric" => Some(Subtype::Cleric),
        "construct" => Some(Subtype::Construct),
        "crab" => Some(Subtype::Crab),
        "crocodile" => Some(Subtype::Crocodile),
        "dauthi" => Some(Subtype::Dauthi),
        "detective" => Some(Subtype::Detective),
        "demon" => Some(Subtype::Demon),
        "devil" => Some(Subtype::Devil),
        "dinosaur" => Some(Subtype::Dinosaur),
        "djinn" => Some(Subtype::Djinn),
        "efreet" | "efreets" => Some(Subtype::Efreet),
        "dog" => Some(Subtype::Dog),
        "drone" | "drones" => Some(Subtype::Drone),
        "dragon" => Some(Subtype::Dragon),
        "drake" => Some(Subtype::Drake),
        "druid" => Some(Subtype::Druid),
        "dwarf" => Some(Subtype::Dwarf),
        "elder" => Some(Subtype::Elder),
        "eldrazi" => Some(Subtype::Eldrazi),
        "spawn" | "spawns" => Some(Subtype::Spawn),
        "scion" | "scions" => Some(Subtype::Scion),
        "elemental" => Some(Subtype::Elemental),
        "elephant" => Some(Subtype::Elephant),
        "elf" | "elves" => Some(Subtype::Elf),
        "faerie" => Some(Subtype::Faerie),
        "fish" => Some(Subtype::Fish),
        "fox" => Some(Subtype::Fox),
        "frog" => Some(Subtype::Frog),
        "fungus" => Some(Subtype::Fungus),
        "gargoyle" => Some(Subtype::Gargoyle),
        "giant" => Some(Subtype::Giant),
        "gnome" => Some(Subtype::Gnome),
        "glimmer" | "glimmers" => Some(Subtype::Glimmer),
        "goat" => Some(Subtype::Goat),
        "goblin" => Some(Subtype::Goblin),
        "god" => Some(Subtype::God),
        "golem" => Some(Subtype::Golem),
        "gorgon" => Some(Subtype::Gorgon),
        "germ" | "germs" => Some(Subtype::Germ),
        "gremlin" | "gremlins" => Some(Subtype::Gremlin),
        "griffin" => Some(Subtype::Griffin),
        "hag" => Some(Subtype::Hag),
        "halfling" => Some(Subtype::Halfling),
        "harpy" => Some(Subtype::Harpy),
        "hippo" => Some(Subtype::Hippo),
        "horror" => Some(Subtype::Horror),
        "homunculus" | "homunculi" => Some(Subtype::Homunculus),
        "horse" => Some(Subtype::Horse),
        "hound" => Some(Subtype::Hound),
        "human" => Some(Subtype::Human),
        "hydra" => Some(Subtype::Hydra),
        "illusion" => Some(Subtype::Illusion),
        "imp" => Some(Subtype::Imp),
        "insect" => Some(Subtype::Insect),
        "inkling" | "inklings" => Some(Subtype::Inkling),
        "jellyfish" => Some(Subtype::Jellyfish),
        "kavu" => Some(Subtype::Kavu),
        "kirin" => Some(Subtype::Kirin),
        "kithkin" => Some(Subtype::Kithkin),
        "knight" => Some(Subtype::Knight),
        "kobold" => Some(Subtype::Kobold),
        "kor" => Some(Subtype::Kor),
        "kraken" => Some(Subtype::Kraken),
        "leviathan" => Some(Subtype::Leviathan),
        "lizard" => Some(Subtype::Lizard),
        "manticore" => Some(Subtype::Manticore),
        "mercenary" => Some(Subtype::Mercenary),
        "merfolk" => Some(Subtype::Merfolk),
        "minion" => Some(Subtype::Minion),
        "minotaur" => Some(Subtype::Minotaur),
        "mole" => Some(Subtype::Mole),
        "monk" => Some(Subtype::Monk),
        "monkey" | "monkeys" => Some(Subtype::Monkey),
        "moonfolk" => Some(Subtype::Moonfolk),
        "mount" | "mounts" => Some(Subtype::Mount),
        "mouse" => Some(Subtype::Mouse),
        "mutant" => Some(Subtype::Mutant),
        "myr" => Some(Subtype::Myr),
        "naga" => Some(Subtype::Naga),
        "nightmare" => Some(Subtype::Nightmare),
        "ninja" => Some(Subtype::Ninja),
        "noble" => Some(Subtype::Noble),
        "octopus" | "octopuses" => Some(Subtype::Octopus),
        "ogre" => Some(Subtype::Ogre),
        "ooze" => Some(Subtype::Ooze),
        "orc" => Some(Subtype::Orc),
        "otter" => Some(Subtype::Otter),
        "ox" => Some(Subtype::Ox),
        "oyster" => Some(Subtype::Oyster),
        "peasant" => Some(Subtype::Peasant),
        "pest" => Some(Subtype::Pest),
        "pegasus" => Some(Subtype::Pegasus),
        "phyrexian" => Some(Subtype::Phyrexian),
        "phoenix" => Some(Subtype::Phoenix),
        "pilot" => Some(Subtype::Pilot),
        "pirate" => Some(Subtype::Pirate),
        "plant" => Some(Subtype::Plant),
        "praetor" => Some(Subtype::Praetor),
        "raccoon" => Some(Subtype::Raccoon),
        "rabbit" => Some(Subtype::Rabbit),
        "rat" => Some(Subtype::Rat),
        "reflection" => Some(Subtype::Reflection),
        "rebel" => Some(Subtype::Rebel),
        "rhino" => Some(Subtype::Rhino),
        "rogue" => Some(Subtype::Rogue),
        "robot" => Some(Subtype::Robot),
        "salamander" => Some(Subtype::Salamander),
        "saproling" | "saprolings" => Some(Subtype::Saproling),
        "samurai" => Some(Subtype::Samurai),
        "satyr" => Some(Subtype::Satyr),
        "scarecrow" => Some(Subtype::Scarecrow),
        "scout" => Some(Subtype::Scout),
        "servo" | "servos" => Some(Subtype::Servo),
        "serpent" => Some(Subtype::Serpent),
        "shade" => Some(Subtype::Shade),
        "shaman" => Some(Subtype::Shaman),
        "shapeshifter" => Some(Subtype::Shapeshifter),
        "shark" => Some(Subtype::Shark),
        "sheep" => Some(Subtype::Sheep),
        "skeleton" => Some(Subtype::Skeleton),
        "slith" => Some(Subtype::Slith),
        "sliver" => Some(Subtype::Sliver),
        "slug" => Some(Subtype::Slug),
        "snake" => Some(Subtype::Snake),
        "soldier" => Some(Subtype::Soldier),
        "sorcerer" => Some(Subtype::Sorcerer),
        "spacecraft" => Some(Subtype::Spacecraft),
        "sphinx" => Some(Subtype::Sphinx),
        "specter" => Some(Subtype::Specter),
        "spider" => Some(Subtype::Spider),
        "spike" => Some(Subtype::Spike),
        "spirit" => Some(Subtype::Spirit),
        "sponge" => Some(Subtype::Sponge),
        "squid" => Some(Subtype::Squid),
        "squirrel" => Some(Subtype::Squirrel),
        "starfish" => Some(Subtype::Starfish),
        "surrakar" => Some(Subtype::Surrakar),
        "thopter" => Some(Subtype::Thopter),
        "thrull" => Some(Subtype::Thrull),
        "tiefling" => Some(Subtype::Tiefling),
        "toy" => Some(Subtype::Toy),
        "treefolk" => Some(Subtype::Treefolk),
        "triskelavite" | "triskelavites" => Some(Subtype::Triskelavite),
        "trilobite" => Some(Subtype::Trilobite),
        "troll" => Some(Subtype::Troll),
        "turtle" => Some(Subtype::Turtle),
        "unicorn" => Some(Subtype::Unicorn),
        "vampire" => Some(Subtype::Vampire),
        "vedalken" => Some(Subtype::Vedalken),
        "viashino" => Some(Subtype::Viashino),
        "wall" => Some(Subtype::Wall),
        "warlock" => Some(Subtype::Warlock),
        "warrior" => Some(Subtype::Warrior),
        "weird" => Some(Subtype::Weird),
        "werewolf" => Some(Subtype::Werewolf),
        "whale" => Some(Subtype::Whale),
        "wizard" => Some(Subtype::Wizard),
        "wolf" => Some(Subtype::Wolf),
        "wolverine" => Some(Subtype::Wolverine),
        "wombat" => Some(Subtype::Wombat),
        "worm" => Some(Subtype::Worm),
        "wraith" => Some(Subtype::Wraith),
        "wurm" => Some(Subtype::Wurm),
        "yeti" => Some(Subtype::Yeti),
        "zombie" => Some(Subtype::Zombie),
        "zubera" => Some(Subtype::Zubera),
        "clue" => Some(Subtype::Clue),
        "contraption" => Some(Subtype::Contraption),
        "equipment" => Some(Subtype::Equipment),
        "food" => Some(Subtype::Food),
        "fortification" => Some(Subtype::Fortification),
        "gold" => Some(Subtype::Gold),
        "treasure" => Some(Subtype::Treasure),
        "vehicle" => Some(Subtype::Vehicle),
        "aura" => Some(Subtype::Aura),
        "background" => Some(Subtype::Background),
        "cartouche" => Some(Subtype::Cartouche),
        "class" => Some(Subtype::Class),
        "curse" => Some(Subtype::Curse),
        "role" => Some(Subtype::Role),
        "rune" => Some(Subtype::Rune),
        "saga" => Some(Subtype::Saga),
        "shard" => Some(Subtype::Shard),
        "shrine" => Some(Subtype::Shrine),
        "adventure" => Some(Subtype::Adventure),
        "arcane" => Some(Subtype::Arcane),
        "lesson" => Some(Subtype::Lesson),
        "trap" => Some(Subtype::Trap),
        "ajani" => Some(Subtype::Ajani),
        "ashiok" => Some(Subtype::Ashiok),
        "chandra" => Some(Subtype::Chandra),
        "elspeth" => Some(Subtype::Elspeth),
        "garruk" => Some(Subtype::Garruk),
        "gideon" => Some(Subtype::Gideon),
        "jace" => Some(Subtype::Jace),
        "karn" => Some(Subtype::Karn),
        "liliana" => Some(Subtype::Liliana),
        "nissa" => Some(Subtype::Nissa),
        "sorin" => Some(Subtype::Sorin),
        "teferi" => Some(Subtype::Teferi),
        "ugin" => Some(Subtype::Ugin),
        "vraska" => Some(Subtype::Vraska),
        _ => None,
    }
}

fn parse_power_toughness(raw: &str) -> Option<PowerToughness> {
    let trimmed = raw.trim();
    let parts: Vec<&str> = trimmed.split('/').collect();
    if parts.len() != 2 {
        return None;
    }

    let power = parse_pt_value(parts[0].trim())?;
    let toughness = parse_pt_value(parts[1].trim())?;
    Some(PowerToughness::new(power, toughness))
}

fn parse_pt_value(raw: &str) -> Option<PtValue> {
    if raw == ".5" || raw == "0.5" {
        return Some(PtValue::Fixed(0));
    }
    if raw == "*" {
        return Some(PtValue::Star);
    }
    if let Some(stripped) = raw.strip_prefix("*+") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Some(stripped) = raw.strip_suffix("+*") {
        let value = stripped.trim().parse::<i32>().ok()?;
        return Some(PtValue::StarPlus(value));
    }
    if let Ok(value) = raw.parse::<i32>() {
        return Some(PtValue::Fixed(value));
    }
    None
}

fn parse_for_each_opponent_doesnt(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    if !words.starts_with(&["for", "each", "opponent"])
        && !words.starts_with(&["for", "each", "opponents"])
    {
        return Ok(None);
    }

    let Some((negation_idx, negation_len)) = negated_action_word_index(&words) else {
        return Ok(None);
    };

    let effect_tokens = if let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    {
        trim_commas(&tokens[comma_idx + 1..])
    } else if let Some(this_way_idx) = words.windows(2).position(|pair| pair == ["this", "way"]) {
        let effect_token_start = token_index_for_word_index(tokens, this_way_idx + 2)
            .unwrap_or(tokens.len());
        trim_commas(&tokens[effect_token_start..])
    } else {
        let effect_token_start = token_index_for_word_index(tokens, negation_idx + negation_len)
            .unwrap_or(tokens.len());
        trim_commas(&tokens[effect_token_start..])
    };
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each opponent who doesn't clause (clause: '{}')",
            words.join(" ")
        )));
    }

    let effects = parse_effect_chain(&effect_tokens)?;
    Ok(Some(EffectAst::ForEachOpponentDoesNot { effects }))
}

fn parse_for_each_player_doesnt(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 5 {
        return Ok(None);
    }

    let start = if clause_words.starts_with(&["for", "each", "player"])
        || clause_words.starts_with(&["for", "each", "players"])
    {
        3
    } else if clause_words.starts_with(&["each", "player"])
        || clause_words.starts_with(&["each", "players"])
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    let inner_words = words(&inner_tokens);
    let starts_with_who = inner_words.first().copied() == Some("who");
    let Some((negation_idx, negation_len)) = negated_action_word_index(&inner_words) else {
        return Ok(None);
    };
    if !starts_with_who {
        return Ok(None);
    }

    let effect_token_start = if let Some(comma_idx) = inner_tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
    {
        comma_idx + 1
    } else if let Some(this_way_idx) = inner_words
        .windows(2)
        .position(|pair| pair == ["this", "way"])
    {
        token_index_for_word_index(&inner_tokens, this_way_idx + 2).unwrap_or(inner_tokens.len())
    } else {
        token_index_for_word_index(&inner_tokens, negation_idx + negation_len)
            .unwrap_or(inner_tokens.len())
    };

    let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect in for each player who doesn't clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let effects = parse_effect_chain(&effect_tokens)?;
    Ok(Some(EffectAst::ForEachPlayerDoesNot { effects }))
}

fn negated_action_word_index(words: &[&str]) -> Option<(usize, usize)> {
    if let Some(idx) = words
        .iter()
        .position(|word| *word == "doesnt" || *word == "didnt")
    {
        return Some((idx, 1));
    }
    for (idx, pair) in words.windows(2).enumerate() {
        if pair == ["do", "not"] || pair == ["did", "not"] {
            return Some((idx, 2));
        }
    }
    None
}

fn parse_vote_start_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    let vote_idx = words
        .iter()
        .position(|word| *word == "vote" || *word == "votes");
    let Some(vote_idx) = vote_idx else {
        return Ok(None);
    };

    let has_each = words[..vote_idx].contains(&"each");
    let has_player = words[..vote_idx]
        .iter()
        .any(|word| *word == "player" || *word == "players");
    if !has_each || !has_player {
        return Ok(None);
    }

    let for_idx = words
        .iter()
        .position(|word| *word == "for")
        .ok_or_else(|| CardTextError::ParseError("missing 'for' in vote clause".to_string()))?;
    if for_idx < vote_idx {
        return Ok(None);
    }

    let option_words = &words[for_idx + 1..];
    let mut options = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for word in option_words {
        if *word == "or" {
            if !current.is_empty() {
                options.push(current.join(" "));
                current.clear();
            }
            continue;
        }
        if is_article(word) {
            continue;
        }
        current.push(word);
    }
    if !current.is_empty() {
        options.push(current.join(" "));
    }

    if options.len() < 2 {
        return Err(CardTextError::ParseError(
            "vote clause requires at least two options".to_string(),
        ));
    }

    Ok(Some(EffectAst::VoteStart { options }))
}

fn parse_for_each_vote_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    if !words.starts_with(&["for", "each"]) {
        return Ok(None);
    }

    let vote_idx = words
        .iter()
        .position(|word| *word == "vote" || *word == "votes");
    let Some(vote_idx) = vote_idx else {
        return Ok(None);
    };
    if vote_idx <= 2 {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }

    let option_words: Vec<&str> = words[2..vote_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if option_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing vote option name".to_string(),
        ));
    }
    let option = option_words.join(" ");

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| {
            CardTextError::ParseError("missing comma in for each vote clause".to_string())
        })?;

    let effect_tokens = &tokens[comma_idx + 1..];
    let effects = parse_effect_chain(effect_tokens)?;
    Ok(Some(EffectAst::VoteOption { option, effects }))
}

fn parse_vote_extra_sentence(tokens: &[Token]) -> Option<EffectAst> {
    let words = words(tokens);
    if words.len() < 3 || words.first().copied() != Some("you") {
        return None;
    }

    let has_vote = words.iter().any(|word| *word == "vote" || *word == "votes");
    let has_additional = words.contains(&"additional");
    let has_time = words.iter().any(|word| *word == "time" || *word == "times");
    if !has_vote || !has_additional || !has_time {
        return None;
    }

    let optional = words.contains(&"may");
    Some(EffectAst::VoteExtra { count: 1, optional })
}

fn parse_after_turn_sentence(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let line_words = words(tokens);
    if line_words.len() < 3
        || line_words[0] != "after"
        || line_words[1] != "that"
        || line_words[2] != "turn"
    {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)));
    let remainder = if let Some(idx) = comma_idx {
        &tokens[idx + 1..]
    } else {
        &tokens[3..]
    };

    let remaining_words: Vec<&str> = words(remainder)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if remaining_words.len() < 4 {
        return Err(CardTextError::ParseError(
            "unsupported after turn clause".to_string(),
        ));
    }

    let player = if remaining_words.starts_with(&["that", "player"]) {
        PlayerAst::That
    } else if remaining_words.starts_with(&["target", "player"]) {
        PlayerAst::Target
    } else if remaining_words.starts_with(&["you"]) {
        PlayerAst::You
    } else {
        return Err(CardTextError::ParseError(
            "unsupported after turn player".to_string(),
        ));
    };

    if remaining_words.contains(&"extra") && remaining_words.contains(&"turn") {
        return Ok(Some(EffectAst::ExtraTurnAfterTurn { player }));
    }

    Err(CardTextError::ParseError(
        "unsupported after turn clause".to_string(),
    ))
}

fn parse_conditional_sentence(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let comma_indices = tokens
        .iter()
        .enumerate()
        .filter_map(|(idx, token)| matches!(token, Token::Comma(_)).then_some(idx))
        .collect::<Vec<_>>();
    if comma_indices.is_empty() {
        return Err(CardTextError::ParseError(
            "missing comma in if clause".to_string(),
        ));
    }

    // For result predicates ("if you do, ..."), always split at the first comma.
    // The effect tail frequently contains additional commas (search/reveal/put, etc.)
    // that should stay in the true branch.
    let first_comma_idx = comma_indices[0];
    if first_comma_idx > 1 {
        let predicate_tokens = &tokens[1..first_comma_idx];
        if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let effects = parse_effect_chain(effect_tokens)?;
            return Ok(vec![EffectAst::IfResult { predicate, effects }]);
        }
        if let Ok(predicate) = parse_predicate(predicate_tokens) {
            let effect_tokens = &tokens[first_comma_idx + 1..];
            let comma_fragment_looks_like_effect = if comma_indices.len() > 1 {
                let fragment_tokens = &tokens[first_comma_idx + 1..comma_indices[1]];
                parse_effect_chain(fragment_tokens)
                    .map(|effects| !effects.is_empty())
                    .unwrap_or(false)
            } else {
                true
            };
            if comma_fragment_looks_like_effect
                && let Ok(effects) = parse_effect_chain(effect_tokens)
                && !effects.is_empty()
            {
                return Ok(vec![EffectAst::Conditional {
                    predicate,
                    if_true: effects,
                    if_false: Vec::new(),
                }]);
            }
        }
    }

    // Prefer the rightmost comma that yields a parseable effect clause so
    // predicates like "if it's an artifact, creature, enchantment, or land card,"
    // keep their internal comma-separated type list intact.
    let mut split: Option<(usize, Vec<EffectAst>)> = None;
    for idx in comma_indices.iter().rev().copied() {
        let effect_tokens = &tokens[idx + 1..];
        if effect_tokens.is_empty() {
            continue;
        }
        if let Ok(effects) = parse_effect_chain(effect_tokens)
            && !effects.is_empty()
        {
            split = Some((idx, effects));
            break;
        }
    }

    let (comma_idx, effects) = if let Some(split) = split {
        split
    } else {
        let first_idx = comma_indices[0];
        let effect_tokens = &tokens[first_idx + 1..];
        (first_idx, parse_effect_chain(effect_tokens)?)
    };
    let predicate_tokens = &tokens[1..comma_idx];

    if let Some(predicate) = parse_if_result_predicate(predicate_tokens) {
        return Ok(vec![EffectAst::IfResult { predicate, effects }]);
    }

    let predicate = parse_predicate(predicate_tokens)?;
    Ok(vec![EffectAst::Conditional {
        predicate,
        if_true: effects,
        if_false: Vec::new(),
    }])
}

fn parse_if_result_predicate(tokens: &[Token]) -> Option<IfResultPredicate> {
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if words.len() >= 2 && words[0] == "you" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }
    if words.len() >= 2 && words[0] == "they" && words[1] == "do" {
        return Some(IfResultPredicate::Did);
    }

    if words.len() >= 5
        && (words[0] == "that" || words[0] == "it")
        && (words[1] == "creature" || words[1] == "permanent" || words[1] == "card")
        && words[2] == "dies"
        && words[3] == "this"
        && words[4] == "way"
    {
        return Some(IfResultPredicate::DiesThisWay);
    }

    if words.len() >= 2 && words[0] == "you" && (words[1] == "dont" || words[1] == "do") {
        if words.len() >= 3 && words[2] == "not" {
            return Some(IfResultPredicate::DidNot);
        }
        if words[1] == "dont" {
            return Some(IfResultPredicate::DidNot);
        }
    }
    if words.len() >= 2 && words[0] == "you" && words[1] == "cant" {
        return Some(IfResultPredicate::DidNot);
    }
    if words.len() >= 3 && words[0] == "you" && words[1] == "can" && words[2] == "not" {
        return Some(IfResultPredicate::DidNot);
    }
    if words.len() >= 2 && words[0] == "they" && (words[1] == "dont" || words[1] == "do") {
        if words.len() >= 3 && words[2] == "not" {
            return Some(IfResultPredicate::DidNot);
        }
        if words[1] == "dont" {
            return Some(IfResultPredicate::DidNot);
        }
    }
    if words.len() >= 2 && words[0] == "they" && words[1] == "cant" {
        return Some(IfResultPredicate::DidNot);
    }
    if words.len() >= 3 && words[0] == "they" && words[1] == "can" && words[2] == "not" {
        return Some(IfResultPredicate::DidNot);
    }

    None
}

fn parse_predicate(tokens: &[Token]) -> Result<PredicateAst, CardTextError> {
    let mut filtered: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word) && *word != "is")
        .collect();

    if filtered.is_empty() {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    if filtered.as_slice() == ["this", "tapped"] || filtered.as_slice() == ["thiss", "tapped"] {
        return Ok(PredicateAst::SourceIsTapped);
    }

    if filtered.as_slice() == ["you", "attacked", "this", "turn"] {
        return Ok(PredicateAst::YouAttackedThisTurn);
    }

    if filtered.as_slice() == ["no", "spells", "were", "cast", "last", "turn"]
        || filtered.as_slice() == ["no", "spell", "was", "cast", "last", "turn"]
    {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
    }
    if filtered.as_slice() == ["it", "was", "kicked"]
        || filtered.as_slice() == ["that", "was", "kicked"]
    {
        return Ok(PredicateAst::TargetWasKicked);
    }
    if filtered.as_slice() == ["its", "controller", "poisoned"]
        || filtered.as_slice() == ["that", "spells", "controller", "poisoned"]
    {
        return Ok(PredicateAst::TargetSpellControllerIsPoisoned);
    }
    if filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "it"]
        || filtered.as_slice() == ["no", "mana", "was", "spent", "to", "cast", "that", "spell"]
        || filtered.as_slice() == ["no", "mana", "were", "spent", "to", "cast", "that", "spell"]
    {
        return Ok(PredicateAst::TargetSpellNoManaSpentToCast);
    }
    if filtered.as_slice()
        == [
            "you",
            "control",
            "more",
            "creatures",
            "than",
            "that",
            "spells",
            "controller",
        ]
        || filtered.as_slice()
            == [
                "you",
                "control",
                "more",
                "creatures",
                "than",
                "its",
                "controller",
            ]
    {
        return Ok(PredicateAst::YouControlMoreCreaturesThanTargetSpellController);
    }
    if filtered.len() == 7
        && matches!(filtered[0], "w" | "u" | "b" | "r" | "g" | "c")
        && filtered[1] == "was"
        && filtered[2] == "spent"
        && filtered[3] == "to"
        && filtered[4] == "cast"
        && filtered[5] == "this"
        && filtered[6] == "spell"
        && let Ok(symbol) = parse_mana_symbol(filtered[0])
    {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    }

    if let Some((amount, symbol)) = parse_mana_spent_to_cast_predicate(&filtered) {
        return Ok(PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol });
    }

    if filtered[0] == "its" {
        filtered[0] = "it";
    }

    if filtered.len() >= 2 {
        let tag = if filtered.starts_with(&["equipped", "creature"]) {
            Some("equipped")
        } else if filtered.starts_with(&["enchanted", "creature"]) {
            Some("enchanted")
        } else {
            None
        };
        if let Some(tag) = tag {
            let remainder = filtered[2..].to_vec();
            let tokens = remainder
                .iter()
                .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
                .collect::<Vec<_>>();
            let mut filter = parse_object_filter(&tokens, false)?;
            if filter.card_types.is_empty() {
                filter.card_types.push(CardType::Creature);
            }
            return Ok(PredicateAst::TaggedMatches(TagKey::from(tag), filter));
        }
    }

    let is_it = filtered.first().is_some_and(|word| *word == "it");
    let has_card = filtered.contains(&"card");

    if is_it {
        if filtered.len() >= 3 && filtered[1] == "mana" && filtered[2] == "value" {
            let mana_value_tail = &filtered[3..];
            let compares_to_colors_spent = mana_value_tail
                == [
                    "less", "than", "or", "equal", "to", "number", "of", "colors", "of",
                    "mana", "spent", "to", "cast", "this", "spell",
                ]
                || mana_value_tail
                    == [
                        "less", "than", "or", "equal", "to", "number", "of", "color", "of",
                        "mana", "spent", "to", "cast", "this", "spell",
                    ];
            if compares_to_colors_spent {
                return Ok(PredicateAst::TargetManaValueLteColorsSpentToCastThisSpell);
            }

            if let Some((cmp, _consumed)) =
                parse_filter_comparison_tokens("mana value", mana_value_tail, &filtered)?
            {
                return Ok(PredicateAst::ItMatches(ObjectFilter {
                    mana_value: Some(cmp),
                    ..Default::default()
                }));
            }
        }

        let mut card_types = Vec::new();
        for word in &filtered {
            if let Some(card_type) = parse_card_type(word)
                && !card_types.contains(&card_type)
            {
                card_types.push(card_type);
            }
        }
        let mut subtypes = Vec::new();
        for word in &filtered {
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }
        if !card_types.is_empty() || !subtypes.is_empty() {
            if has_card && card_types.len() == 1 && card_types[0] == CardType::Land {
                return Ok(PredicateAst::ItIsLandCard);
            }
            return Ok(PredicateAst::ItMatches(ObjectFilter {
                card_types,
                subtypes,
                ..Default::default()
            }));
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
}

fn parse_sentence_counter_target_spell_if_it_was_kicked(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.as_slice() != ["counter", "target", "spell", "if", "it", "was", "kicked"] {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(&tokens[1..3]));
    let counter = EffectAst::Counter { target };
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetWasKicked,
        if_true: vec![counter],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}

fn parse_sentence_counter_target_spell_thats_second_cast_this_turn(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let matches = clause_words.as_slice()
        == [
            "counter", "target", "spell", "thats", "second", "spell", "cast", "this", "turn",
        ]
        || clause_words.as_slice()
            == [
                "counter", "target", "spell", "thats", "the", "second", "spell", "cast",
                "this", "turn",
            ];
    if !matches {
        return Ok(None);
    }

    let target = TargetAst::Spell(span_from_tokens(&tokens[1..3]));
    let counter = EffectAst::Counter { target };
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetSpellCastOrderThisTurn(2),
        if_true: vec![counter],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}

fn parse_sentence_exile_target_creature_with_greatest_power(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let clause_words = words(tokens);
    let is_shape = clause_words.starts_with(&["exile", "target", "creature"])
        && contains_word_sequence(
            &clause_words,
            &["greatest", "power", "among", "creatures"],
        )
        && (clause_words.windows(2).any(|pair| pair == ["on", "battlefield"])
            || clause_words
                .windows(3)
                .any(|triplet| triplet == ["on", "the", "battlefield"]));
    if !is_shape {
        return Ok(None);
    }

    let target_tokens = trim_commas(&tokens[1..3]);
    let target = parse_target_phrase(&target_tokens)?;
    let exile = EffectAst::Exile {
        target: target.clone(),
    };
    let effect = EffectAst::Conditional {
        predicate: PredicateAst::TargetHasGreatestPowerAmongCreatures,
        if_true: vec![exile],
        if_false: Vec::new(),
    };
    Ok(Some(vec![effect]))
}

fn parse_mana_spent_to_cast_predicate(words: &[&str]) -> Option<(u32, Option<ManaSymbol>)> {
    if words.len() < 10 || words[0] != "at" || words[1] != "least" {
        return None;
    }

    let amount_tokens = vec![Token::Word(words[2].to_string(), TextSpan::synthetic())];
    let (amount, _) = parse_number(&amount_tokens)?;

    let mut idx = 3;
    if words.get(idx).copied() == Some("of") {
        idx += 1;
    }

    let symbol = if let Some(word) = words.get(idx).copied() {
        if let Some(parsed) = parse_mana_symbol_word(word) {
            idx += 1;
            Some(parsed)
        } else {
            None
        }
    } else {
        None
    };

    let tail = &words[idx..];
    let canonical_tail = ["mana", "was", "spent", "to", "cast", "this", "spell"];
    let plural_tail = ["mana", "were", "spent", "to", "cast", "this", "spell"];
    if tail == canonical_tail || tail == plural_tail {
        return Some((amount, symbol));
    }

    None
}

fn parse_mana_symbol_word(word: &str) -> Option<ManaSymbol> {
    match word {
        "white" => Some(ManaSymbol::White),
        "blue" => Some(ManaSymbol::Blue),
        "black" => Some(ManaSymbol::Black),
        "red" => Some(ManaSymbol::Red),
        "green" => Some(ManaSymbol::Green),
        "colorless" => Some(ManaSymbol::Colorless),
        _ => None,
    }
}

fn parse_effect_chain(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let words = words(tokens);
    let starts_with_each_opponent =
        words.starts_with(&["each", "opponent"]) || words.starts_with(&["each", "opponents"]);
    let starts_with_each_player =
        words.starts_with(&["each", "player"]) || words.starts_with(&["each", "players"]);

    if tokens.first().is_some_and(|token| token.is_word("they"))
        && tokens.get(1).is_some_and(|token| token.is_word("may"))
    {
        let inner_tokens = &tokens[2..];
        let effects = parse_effect_chain_with_sentence_primitives(inner_tokens)?;
        return Ok(vec![EffectAst::MayByTaggedController {
            tag: TagKey::from("triggering"),
            effects,
        }]);
    }

    if let Some(player) = parse_leading_player_may(tokens) {
        let mut stripped = remove_through_first_word(tokens, "may");
        if stripped
            .first()
            .is_some_and(|token| token.is_word("have") || token.is_word("has"))
        {
            stripped.remove(0);
        }
        let mut effects = parse_effect_chain(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
    }

    if tokens.first().is_some_and(|token| token.is_word("may"))
        && !starts_with_each_opponent
        && !starts_with_each_player
    {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain(&stripped)?;
        return Ok(vec![EffectAst::May { effects }]);
    }

    if let Some(unless_action) = parse_or_action_clause(tokens)? {
        return Ok(vec![unless_action]);
    }

    parse_effect_chain_with_sentence_primitives(tokens)
}

fn parse_or_action_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let mut option_tokens = split_on_or(tokens);
    if option_tokens.len() != 2 {
        return Ok(None);
    }

    let normalize_option = |mut option: Vec<Token>| {
        while option
            .first()
            .is_some_and(|token| token.is_word("and") || token.is_word("or"))
        {
            option.remove(0);
        }
        trim_commas(&option).to_vec()
    };

    let first = normalize_option(option_tokens.remove(0));
    let second = normalize_option(option_tokens.remove(0));
    if first.is_empty() || second.is_empty() {
        return Ok(None);
    }

    let first_starts_effect =
        find_verb(&first).is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb(&first);
    let second_starts_effect =
        find_verb(&second).is_some_and(|(_, verb_idx)| verb_idx == 0)
            || has_effect_head_without_verb(&second);
    if !first_starts_effect || !second_starts_effect {
        return Ok(None);
    }

    let first_effects = match parse_effect_chain_with_sentence_primitives(&first) {
        Ok(effects) if !effects.is_empty() => effects,
        _ => return Ok(None),
    };
    let second_effects = match parse_effect_chain_with_sentence_primitives(&second) {
        Ok(effects) if !effects.is_empty() => effects,
        _ => return Ok(None),
    };

    Ok(Some(EffectAst::UnlessAction {
        effects: first_effects,
        alternative: second_effects,
        player: PlayerAst::Implicit,
    }))
}

fn parse_effect_chain_with_sentence_primitives(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = run_sentence_primitives(tokens, PRE_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }
    if let Some(effects) = run_sentence_primitives(tokens, POST_CONDITIONAL_SENTENCE_PRIMITIVES)? {
        return Ok(effects);
    }
    parse_effect_chain_inner(tokens)
}

fn parse_effect_chain_inner(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = parse_search_library_sentence(tokens)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let raw_segments = split_effect_chain_on_and(tokens);
    let mut segments: Vec<Vec<Token>> = Vec::new();
    for segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segments.is_empty() {
            segments.push(segment);
            continue;
        }
        if !segment_has_effect_head(&segment) {
            if let Some(previous) = segments.last()
                && let Some(expanded) = expand_missing_verb_segment(previous, &segment)
            {
                segments.push(expanded);
                continue;
            }
            let last = segments.last_mut().expect("non-empty segments");
            last.push(Token::Word("and".to_string(), TextSpan::synthetic()));
            last.extend(segment);
            continue;
        }
        segments.push(segment);
    }
    while segments.len() > 1 && !segment_has_effect_head(&segments[0]) {
        let mut first = segments.remove(0);
        first.push(Token::Word("and".to_string(), TextSpan::synthetic()));
        let mut next = segments.remove(0);
        first.append(&mut next);
        segments.insert(0, first);
    }
    // Split segments on ", then" when the part after "then" doesn't
    // back-reference the first part (no "that", "it", "them", "its").
    // This handles patterns like "discard your hand, then draw four cards".
    segments = split_segments_on_comma_then(segments);
    segments = split_segments_on_comma_effect_head(segments);
    segments = expand_segments_with_comma_action_clauses(segments);
    segments = expand_segments_with_multi_create_clauses(segments);
    let mut carried_context: Option<CarryContext> = None;
    for segment in segments {
        let mut effect = parse_effect_clause(&segment)?;
        if let Some(context) = carried_context {
            maybe_apply_carried_player(&mut effect, context);
        }
        if let Some(context) = explicit_player_for_carry(&effect) {
            carried_context = Some(context);
        }
        effects.push(effect);
    }
    collapse_token_copy_next_end_step_exile_followup(&mut effects, tokens);
    Ok(effects)
}

fn is_beginning_of_end_step_words(words: &[&str]) -> bool {
    words.windows(5)
        .any(|window| window == ["beginning", "of", "the", "end", "step"])
        || words
            .windows(5)
            .any(|window| window == ["beginning", "of", "next", "end", "step"])
        || words
            .windows(6)
            .any(|window| window == ["beginning", "of", "the", "next", "end", "step"])
}

fn target_is_generic_token_filter(target: &TargetAst) -> bool {
    let TargetAst::Object(filter, _, _) = target else {
        return false;
    };
    filter.token
        && filter.zone.is_none()
        && filter.card_types.is_empty()
        && filter.subtypes.is_empty()
        && filter.tagged_constraints.is_empty()
        && filter.controller.is_none()
        && filter.owner.is_none()
}

fn collapse_token_copy_next_end_step_exile_followup(effects: &mut Vec<EffectAst>, tokens: &[Token]) {
    let chain_words = words(tokens);
    if !chain_words.contains(&"exile")
        || !chain_words.contains(&"token")
        || !is_beginning_of_end_step_words(&chain_words)
    {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < effects.len() {
        let mark_next_end_step_exile = match (&effects[idx], &effects[idx + 1]) {
            (
                EffectAst::CreateTokenCopy { .. } | EffectAst::CreateTokenCopyFromSource { .. },
                EffectAst::MoveToZone {
                    target,
                    zone: Zone::Exile,
                    ..
                },
            ) => target_is_generic_token_filter(target),
            (
                EffectAst::CreateTokenCopy { .. } | EffectAst::CreateTokenCopyFromSource { .. },
                EffectAst::Exile { target },
            ) => target_is_generic_token_filter(target),
            _ => false,
        };

        if !mark_next_end_step_exile {
            idx += 1;
            continue;
        }

        match &mut effects[idx] {
            EffectAst::CreateTokenCopy {
                exile_at_next_end_step,
                ..
            }
            | EffectAst::CreateTokenCopyFromSource {
                exile_at_next_end_step,
                ..
            } => {
                *exile_at_next_end_step = true;
            }
            _ => {}
        }
        effects.remove(idx + 1);
    }
}

fn expand_segments_with_comma_action_clauses(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let segment_words = words(&segment);
        let looks_like_sac_discard_chain = (segment_words.contains(&"sacrifice")
            || segment_words.contains(&"sacrifices"))
            && (segment_words.contains(&"discard") || segment_words.contains(&"discards"));
        if !looks_like_sac_discard_chain {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<Token>> = Vec::new();
        let mut valid_split = true;

        for raw_part in comma_parts {
            let mut part = trim_commas(&raw_part).to_vec();
            while part.first().is_some_and(|token| token.is_word("and")) {
                part.remove(0);
            }
            if part.is_empty() {
                continue;
            }

            if segment_has_effect_head(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }

            valid_split = false;
            break;
        }

        if valid_split && local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

fn starts_like_create_fragment(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.is_empty() {
        return false;
    }
    let starts_like_count = words.first().is_some_and(|word| {
        matches!(*word, "a" | "an" | "one" | "two" | "three" | "four" | "five" | "six")
    }) || words
        .first()
        .is_some_and(|word| parse_number(&[Token::Word((*word).to_string(), TextSpan::synthetic())]).is_some())
        || words
            .first()
            .is_some_and(|word| word.contains('/') || word == &"x");
    starts_like_count && words.iter().any(|word| matches!(*word, "token" | "tokens"))
}

fn expand_segments_with_multi_create_clauses(segments: Vec<Vec<Token>>) -> Vec<Vec<Token>> {
    let mut expanded = Vec::new();

    for segment in segments {
        let Some((Verb::Create, _)) = find_verb(&segment) else {
            expanded.push(segment);
            continue;
        };
        let segment_words = words(&segment);
        let has_token_rules_tail = segment_words
            .windows(3)
            .any(|window| matches!(window, ["when", "this", "token"] | ["whenever", "this", "token"]))
            || segment_words
                .windows(2)
                .any(|window| matches!(window, ["this", "token"] | ["that", "token"] | ["those", "tokens"]))
            || segment_words
                .windows(2)
                .any(|window| matches!(window, ["it", "has"] | ["they", "have"]));
        if has_token_rules_tail {
            expanded.push(segment);
            continue;
        }
        let token_mentions = segment_words
            .into_iter()
            .filter(|word| matches!(*word, "token" | "tokens"))
            .count();
        if token_mentions < 2 {
            expanded.push(segment);
            continue;
        }

        let comma_parts = split_on_comma_or_semicolon(&segment);
        if comma_parts.len() < 2 {
            expanded.push(segment);
            continue;
        }

        let mut local_parts: Vec<Vec<Token>> = Vec::new();
        for part in comma_parts {
            if part.is_empty() {
                continue;
            }
            if let Some(previous) = local_parts.last()
                && is_token_creation_context(&words(previous))
                && starts_with_inline_token_rules_tail(&words(&part))
            {
                if let Some(last) = local_parts.last_mut() {
                    last.push(Token::Comma(TextSpan::synthetic()));
                    last.extend(part);
                }
                continue;
            }
            if segment_has_effect_head(&part) {
                local_parts.push(part);
                continue;
            }
            if let Some(previous) = local_parts.last()
                && let Some(expanded_part) = expand_missing_verb_segment(previous, &part)
            {
                local_parts.push(expanded_part);
                continue;
            }
            if let Some(last) = local_parts.last_mut() {
                last.push(Token::Comma(TextSpan::synthetic()));
                last.extend(part);
            } else {
                local_parts.push(part);
            }
        }

        if local_parts.len() > 1 {
            expanded.extend(local_parts);
        } else {
            expanded.push(segment);
        }
    }

    expanded
}

fn expand_missing_verb_segment(previous: &[Token], segment: &[Token]) -> Option<Vec<Token>> {
    let (verb, verb_idx) = find_verb(previous)?;
    match verb {
        Verb::Deal => {
            let segment_words = words(segment);
            if parse_value(segment).is_none() || !segment_words.contains(&"damage") {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Sacrifice => {
            let segment_words = words(segment);
            let starts_like_object_phrase = matches!(
                segment_words.first().copied(),
                Some("a" | "an" | "another" | "target")
            ) || parse_number(segment).is_some();
            if !starts_like_object_phrase {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        Verb::Create => {
            if !starts_like_create_fragment(segment) {
                return None;
            }
            let mut expanded = Vec::new();
            expanded.extend(previous.iter().take(verb_idx + 1).cloned());
            expanded.extend(segment.iter().cloned());
            Some(expanded)
        }
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CarryContext {
    Player(PlayerAst),
    ForEachPlayer,
    ForEachOpponent,
}

fn explicit_player_for_carry(effect: &EffectAst) -> Option<CarryContext> {
    if matches!(effect, EffectAst::ForEachPlayer { .. }) {
        return Some(CarryContext::ForEachPlayer);
    }
    if matches!(effect, EffectAst::ForEachOpponent { .. }) {
        return Some(CarryContext::ForEachOpponent);
    }

    let player = match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player } => *player,
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(CarryContext::Player(player))
    }
}

fn effect_uses_implicit_player(effect: &EffectAst) -> bool {
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::DiscardHand { player }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Sacrifice { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. }
        | EffectAst::PoisonCounters { player, .. }
        | EffectAst::EnergyCounters { player, .. }
        | EffectAst::RevealTop { player }
        | EffectAst::RevealHand { player } => matches!(*player, PlayerAst::Implicit),
        _ => false,
    }
}

fn maybe_apply_carried_player(effect: &mut EffectAst, carried_context: CarryContext) {
    match carried_context {
        CarryContext::Player(carried_player) => match effect {
            EffectAst::DiscardHand { player }
            | EffectAst::Discard { player, .. }
            | EffectAst::GainLife { player, .. }
            | EffectAst::LoseLife { player, .. }
            | EffectAst::Scry { player, .. }
            | EffectAst::Surveil { player, .. }
            | EffectAst::Mill { player, .. }
            | EffectAst::PoisonCounters { player, .. }
            | EffectAst::EnergyCounters { player, .. }
            | EffectAst::RevealTop { player }
            | EffectAst::RevealHand { player } => {
                if matches!(*player, PlayerAst::Implicit) {
                    *player = carried_player;
                }
            }
            _ => {}
        },
        CarryContext::ForEachPlayer => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachPlayer {
                    effects: vec![wrapped],
                };
            }
        }
        CarryContext::ForEachOpponent => {
            if effect_uses_implicit_player(effect) {
                let wrapped = effect.clone();
                *effect = EffectAst::ForEachOpponent {
                    effects: vec![wrapped],
                };
            }
        }
    }
}

fn bind_implicit_player_context(effect: &mut EffectAst, player: PlayerAst) {
    match effect {
        EffectAst::Draw {
            player: effect_player,
            ..
        }
        | EffectAst::DiscardHand {
            player: effect_player,
        }
        | EffectAst::Discard {
            player: effect_player,
            ..
        }
        | EffectAst::GainLife {
            player: effect_player,
            ..
        }
        | EffectAst::LoseLife {
            player: effect_player,
            ..
        }
        | EffectAst::Sacrifice {
            player: effect_player,
            ..
        }
        | EffectAst::Scry {
            player: effect_player,
            ..
        }
        | EffectAst::Surveil {
            player: effect_player,
            ..
        }
        | EffectAst::Mill {
            player: effect_player,
            ..
        }
        | EffectAst::PoisonCounters {
            player: effect_player,
            ..
        }
        | EffectAst::EnergyCounters {
            player: effect_player,
            ..
        }
        | EffectAst::RevealTop {
            player: effect_player,
        }
        | EffectAst::RevealHand {
            player: effect_player,
        }
        | EffectAst::PayMana {
            player: effect_player,
            ..
        }
        | EffectAst::PayEnergy {
            player: effect_player,
            ..
        }
        | EffectAst::AddMana {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaScaled {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaAnyColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaAnyOneColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaChosenColor {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaFromLandCouldProduce {
            player: effect_player,
            ..
        }
        | EffectAst::AddManaCommanderIdentity {
            player: effect_player,
            ..
        }
        | EffectAst::PutIntoHand {
            player: effect_player,
            ..
        }
        | EffectAst::CopySpell {
            player: effect_player,
            ..
        } => {
            if matches!(*effect_player, PlayerAst::Implicit) {
                *effect_player = player;
            }
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects }
        | EffectAst::ForEachPlayerDoesNot { effects }
        | EffectAst::ForEachOpponentDid { effects }
        | EffectAst::ForEachPlayerDid { effects }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
        | EffectAst::UnlessPays { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            for nested in effects {
                bind_implicit_player_context(nested, player);
            }
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            for nested in effects {
                bind_implicit_player_context(nested, player);
            }
            for nested in alternative {
                bind_implicit_player_context(nested, player);
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            for nested in if_true {
                bind_implicit_player_context(nested, player);
            }
            for nested in if_false {
                bind_implicit_player_context(nested, player);
            }
        }
        _ => {}
    }
}

fn parse_leading_player_may(tokens: &[Token]) -> Option<PlayerAst> {
    let mut words = words(tokens);
    while words
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        words.remove(0);
    }
    if words.len() < 2 {
        return None;
    }

    if words.starts_with(&["you", "may"]) {
        return Some(PlayerAst::You);
    }
    if words.starts_with(&["target", "opponent", "may"])
        || words.starts_with(&["target", "opponents", "may"])
    {
        return Some(PlayerAst::TargetOpponent);
    }
    if words.starts_with(&["target", "player", "may"])
        || words.starts_with(&["target", "players", "may"])
    {
        return Some(PlayerAst::Target);
    }
    if words.starts_with(&["that", "player", "may"])
        || words.starts_with(&["that", "players", "may"])
    {
        return Some(PlayerAst::That);
    }
    if words.len() >= 4
        && words[0] == "that"
        && matches!(words[1], "creatures" | "permanents" | "sources" | "spells")
        && words[2] == "controller"
        && words[3] == "may"
    {
        return Some(PlayerAst::ItsController);
    }
    if words.len() >= 4
        && words[0] == "that"
        && matches!(words[1], "creatures" | "permanents" | "sources" | "spells")
        && words[2] == "owner"
        && words[3] == "may"
    {
        return Some(PlayerAst::ItsOwner);
    }
    if words.starts_with(&["the", "player", "may"])
        || words.starts_with(&["the", "players", "may"])
    {
        return Some(PlayerAst::That);
    }
    if words.starts_with(&["defending", "player", "may"]) {
        return Some(PlayerAst::Defending);
    }
    if words.starts_with(&["attacking", "player", "may"])
        || words.starts_with(&["the", "attacking", "player", "may"])
    {
        return Some(PlayerAst::Attacking);
    }
    if words.starts_with(&["its", "controller", "may"])
        || words.starts_with(&["their", "controller", "may"])
    {
        return Some(PlayerAst::ItsController);
    }
    if words.starts_with(&["its", "owner", "may"])
        || words.starts_with(&["their", "owner", "may"])
    {
        return Some(PlayerAst::ItsOwner);
    }
    if words.starts_with(&["opponent", "may"])
        || words.starts_with(&["opponents", "may"])
        || words.starts_with(&["an", "opponent", "may"])
    {
        return Some(PlayerAst::Opponent);
    }

    None
}

fn remove_first_word(tokens: &[Token], word: &str) -> Vec<Token> {
    let mut removed = false;
    let mut out = Vec::with_capacity(tokens.len());
    for token in tokens {
        if !removed && token.is_word(word) {
            removed = true;
            continue;
        }
        out.push(token.clone());
    }
    out
}

fn remove_through_first_word(tokens: &[Token], word: &str) -> Vec<Token> {
    let mut seen = false;
    let mut out = Vec::new();
    for token in tokens {
        if !seen {
            if token.is_word(word) {
                seen = true;
            }
            continue;
        }
        out.push(token.clone());
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    Add,
    Move,
    Deal,
    Draw,
    Counter,
    Destroy,
    Exile,
    Untap,
    Scry,
    Discard,
    Transform,
    Regenerate,
    Mill,
    Get,
    Reveal,
    Lose,
    Gain,
    Put,
    Sacrifice,
    Create,
    Investigate,
    Proliferate,
    Tap,
    Attach,
    Remove,
    Return,
    Exchange,
    Become,
    Skip,
    Surveil,
    Shuffle,
    Pay,
    Goad,
}

type ClausePrimitiveParser = fn(&[Token]) -> Result<Option<EffectAst>, CardTextError>;

struct ClausePrimitive {
    parser: ClausePrimitiveParser,
}

fn run_clause_primitives(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    const PRIMITIVES: &[ClausePrimitive] = &[
        ClausePrimitive {
            parser: parse_copy_spell_clause,
        },
        ClausePrimitive {
            parser: parse_deal_damage_equal_to_power_clause,
        },
        ClausePrimitive {
            parser: parse_fight_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_opponent_clause,
        },
        ClausePrimitive {
            parser: parse_for_each_player_clause,
        },
        ClausePrimitive {
            parser: parse_double_counters_clause,
        },
        ClausePrimitive {
            parser: parse_distribute_counters_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_next_damage_clause,
        },
        ClausePrimitive {
            parser: parse_prevent_all_damage_clause,
        },
        ClausePrimitive {
            parser: parse_can_attack_as_though_no_defender_clause,
        },
        ClausePrimitive {
            parser: parse_keyword_mechanic_clause,
        },
        ClausePrimitive {
            parser: parse_connive_clause,
        },
        ClausePrimitive {
            parser: parse_verb_first_clause,
        },
    ];

    for primitive in PRIMITIVES {
        if let Some(effect) = (primitive.parser)(tokens)? {
            return Ok(Some(effect));
        }
    }
    Ok(None)
}

fn parse_power_reference_word_count(words: &[&str]) -> Option<usize> {
    if words.starts_with(&["its", "power"]) || words.starts_with(&["that", "power"]) {
        return Some(2);
    }
    if words.starts_with(&["this", "source", "power"])
        || words.starts_with(&["this", "creature", "power"])
        || words.starts_with(&["that", "creature", "power"])
        || words.starts_with(&["that", "objects", "power"])
    {
        return Some(3);
    }
    None
}

fn is_damage_source_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::Source(_) | TargetAst::Object(_, _, _) | TargetAst::Tagged(_, _)
    )
}

fn parse_deal_damage_equal_to_power_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(deal_idx) = tokens
        .iter()
        .position(|token| token.is_word("deal") || token.is_word("deals"))
    else {
        return Ok(None);
    };
    if deal_idx == 0 {
        return Ok(None);
    }

    let source_tokens = trim_commas(&tokens[..deal_idx]);
    let source = parse_target_phrase(&source_tokens)?;
    if !is_damage_source_target(&source) {
        return Err(CardTextError::ParseError(format!(
            "unsupported damage source target phrase (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let rest = trim_commas(&tokens[deal_idx + 1..]);
    if rest.is_empty() || !rest[0].is_word("damage") {
        return Ok(None);
    }

    let Some(equal_idx) = rest
        .windows(2)
        .position(|window| window[0].is_word("equal") && window[1].is_word("to"))
    else {
        return Ok(None);
    };

    let power_ref_words = words(&rest[equal_idx + 2..]);
    let Some(power_ref_len) = parse_power_reference_word_count(&power_ref_words) else {
        return Ok(None);
    };

    let tail_after_power = trim_commas(&rest[equal_idx + 2 + power_ref_len..]);
    let pre_equal_words = words(&rest[..equal_idx]);

    let target = if pre_equal_words == ["damage"] {
        let mut target_tokens = tail_after_power.as_slice();
        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("to"))
        {
            target_tokens = &target_tokens[1..];
        }
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing damage target after power reference (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parse_target_phrase(target_tokens)?
    } else if pre_equal_words.starts_with(&["damage", "to"]) {
        let target_tokens = trim_commas(&rest[2..equal_idx]);
        let target_words = words(&target_tokens);
        if target_words == ["itself"] || target_words == ["it"] {
            if !tail_after_power.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after self-damage power clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            source.clone()
        } else {
            if !tail_after_power.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing target after explicit power-damage target (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            parse_target_phrase(&target_tokens)?
        }
    } else {
        return Ok(None);
    };

    Ok(Some(EffectAst::DealDamageEqualToPower { source, target }))
}

fn parse_fight_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(fight_idx) = tokens
        .iter()
        .position(|token| token.is_word("fight") || token.is_word("fights"))
    else {
        return Ok(None);
    };

    if fight_idx + 1 >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let right_tokens = trim_commas(&tokens[fight_idx + 1..]);
    if right_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "fight clause requires two creatures (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let creature1 = if fight_idx == 0 {
        TargetAst::Source(None)
    } else {
        let left_tokens = trim_commas(&tokens[..fight_idx]);
        if left_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "fight clause requires two creatures (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(filter) = parse_for_each_object_subject(&left_tokens)? {
            let creature2 = parse_target_phrase(&right_tokens)?;
            if matches!(
                creature2,
                TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
            ) {
                return Err(CardTextError::ParseError(format!(
                    "fight target must be a creature (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            return Ok(Some(EffectAst::ForEachObject {
                filter,
                effects: vec![EffectAst::FightIterated { creature2 }],
            }));
        }
        parse_target_phrase(&left_tokens)?
    };
    let creature2 = parse_target_phrase(&right_tokens)?;

    for target in [&creature1, &creature2] {
        if matches!(
            target,
            TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
        ) {
            return Err(CardTextError::ParseError(format!(
                "fight target must be a creature (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    Ok(Some(EffectAst::Fight {
        creature1,
        creature2,
    }))
}

fn parse_effect_clause(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError("empty effect clause".to_string()));
    }

    if let Some(player) = parse_leading_player_may(tokens) {
        let mut stripped = remove_through_first_word(tokens, "may");
        if stripped
            .first()
            .is_some_and(|token| token.is_word("have") || token.is_word("has"))
        {
            stripped.remove(0);
        }
        let mut effects = parse_effect_chain_with_sentence_primitives(&stripped)?;
        for effect in &mut effects {
            bind_implicit_player_context(effect, player);
        }
        return Ok(EffectAst::MayByPlayer { player, effects });
    }

    if tokens.first().is_some_and(|token| token.is_word("may")) {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_with_sentence_primitives(&stripped)?;
        return Ok(EffectAst::May { effects });
    }

    let clause_words = words(tokens);
    if clause_words
        .iter()
        .any(|word| *word == "choose" || *word == "chooses")
        && clause_words.contains(&"creature")
        && clause_words.contains(&"type")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-type choice clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_mana_replacement_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana replacement clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if is_mana_trigger_additional_clause_words(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-triggered additional-mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(effect) = run_clause_primitives(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_clause(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    if let Some((chooser, choose_filter, choose_count)) =
        parse_target_player_choose_objects_clause(tokens)?
    {
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: choose_count,
            player: chooser,
            tag: TagKey::from(IT_TAG),
        });
    }

    if tokens.first().is_some_and(|token| token.is_word("target")) && find_verb(tokens).is_none() {
        let clause_words = words(tokens);
        let looks_like_restriction_clause = find_negation_span(tokens).is_some()
            || clause_words.contains(&"blocked")
            || clause_words.contains(&"except")
            || clause_words.contains(&"unless")
            || clause_words.contains(&"attack")
            || clause_words.contains(&"attacks")
            || clause_words.contains(&"block")
            || clause_words.contains(&"blocks");
        if looks_like_restriction_clause {
            return Err(CardTextError::ParseError(format!(
                "unsupported target-only restriction clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = parse_target_phrase(tokens)?;
        return Ok(EffectAst::TargetOnly { target });
    }

    let (verb, verb_idx) = find_verb(tokens).ok_or_else(|| {
        let clause = words(tokens).join(" ");
        let known_verbs = [
            "add",
            "move",
            "deal",
            "draw",
            "counter",
            "destroy",
            "exile",
            "untap",
            "scry",
            "discard",
            "transform",
            "regenerate",
            "mill",
            "get",
            "reveal",
            "lose",
            "gain",
            "put",
            "sacrifice",
            "create",
            "investigate",
            "attach",
            "remove",
            "return",
            "exchange",
            "become",
            "skip",
            "surveil",
            "shuffle",
            "pay",
            "goad",
        ];
        CardTextError::ParseError(format!(
            "could not find verb in effect clause (clause: '{clause}'; known verbs: {})",
            known_verbs.join(", ")
        ))
    })?;
    parser_trace_stack("parse_effect_clause:verb-found", tokens);

    if matches!(verb, Verb::Counter)
        && verb_idx > 0
        && tokens.iter().any(|token| token.is_word("on"))
    {
        if let Ok(effect) = parse_put_counters(tokens) {
            parser_trace("parse_effect_clause:counter-noun-treated-as-put", tokens);
            return Ok(effect);
        }
    }

    if matches!(verb, Verb::Get) {
        let subject_tokens = &tokens[..verb_idx];
        if !subject_tokens.is_empty() {
            let subject_words = words(subject_tokens);
            if let Some(mod_token) = tokens.get(verb_idx + 1).and_then(Token::as_word)
                && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
            {
                let modifier_tail = &tokens[verb_idx + 1..];
                if let Some(count_filter) = parse_get_for_each_count_filter(modifier_tail)? {
                    let modifier_words = words(modifier_tail);
                    let duration = if modifier_words.starts_with(&["until", "end", "of", "turn"])
                        || modifier_words
                            .windows(4)
                            .any(|window| window == ["until", "end", "of", "turn"])
                    {
                        Until::EndOfTurn
                    } else {
                        Until::EndOfTurn
                    };
                    let target = parse_target_phrase(subject_tokens)?;
                    let power_per = match power {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each power modifier (clause: '{}')",
                                words(tokens).join(" ")
                            )));
                        }
                    };
                    let toughness_per = match toughness {
                        Value::Fixed(value) => value,
                        _ => {
                            return Err(CardTextError::ParseError(format!(
                                "unsupported dynamic gets-for-each toughness modifier (clause: '{}')",
                                words(tokens).join(" ")
                            )));
                        }
                    };
                    return Ok(EffectAst::PumpForEach {
                        power_per,
                        toughness_per,
                        target,
                        count_filter,
                        duration,
                    });
                }

                let (power, toughness, duration) =
                    parse_get_modifier_values_with_tail(modifier_tail, power, toughness)?;

                let normalized_subject_words: Vec<&str> = subject_words
                    .iter()
                    .copied()
                    .filter(|word| *word != "each")
                    .collect();
                if normalized_subject_words.as_slice() == ["it"]
                    || normalized_subject_words.as_slice() == ["they"]
                {
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(subject_tokens)),
                        duration,
                    });
                }

                let is_demonstrative_subject = normalized_subject_words
                    .first()
                    .is_some_and(|word| *word == "that" || *word == "those");
                if is_demonstrative_subject {
                    let target = parse_target_phrase(subject_tokens)?;
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target,
                        duration,
                    });
                }

                if subject_words.contains(&"target") {
                    let target_tokens = if subject_tokens
                        .first()
                        .is_some_and(|token| token.is_word("have") || token.is_word("has"))
                    {
                        &subject_tokens[1..]
                    } else {
                        subject_tokens
                    };
                    let target = parse_target_phrase(target_tokens)?;
                    return Ok(EffectAst::Pump {
                        power: power.clone(),
                        toughness: toughness.clone(),
                        target,
                        duration,
                    });
                }

                if !subject_words.contains(&"this")
                    && !subject_words.contains(&"that")
                    && !subject_words.contains(&"it")
                    && let Ok(filter) = parse_object_filter(subject_tokens, false)
                    && filter != ObjectFilter::default()
                {
                    return Ok(EffectAst::PumpAll {
                        filter,
                        power: power.clone(),
                        toughness: toughness.clone(),
                        duration,
                    });
                }
            }
        }
    }

    let subject_tokens = &tokens[..verb_idx];
    let subject_words = words(subject_tokens);
    if is_target_player_dealt_damage_by_this_turn_subject(&subject_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history player subject (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    if matches!(verb, Verb::Gain) && !subject_tokens.is_empty() {
        let rest_words = words(&tokens[verb_idx + 1..]);
        let has_protection = rest_words.contains(&"protection");
        let has_choice = rest_words.contains(&"choice");
        let has_color = rest_words.contains(&"color");
        let has_colorless = rest_words.contains(&"colorless");
        if has_protection && has_choice && (has_color || has_colorless) {
            let target = parse_target_phrase(subject_tokens)?;
            return Ok(EffectAst::GrantProtectionChoice {
                target,
                allow_colorless: has_colorless,
            });
        }
    }
    let for_each_subject_filter = parse_for_each_object_subject(subject_tokens)?;
    let subject = parse_subject(subject_tokens);
    let rest = &tokens[verb_idx + 1..];
    let mut effect = parse_effect_with_verb(verb, Some(subject), rest)?;
    if let Some(filter) = for_each_subject_filter {
        effect = EffectAst::ForEachObject {
            filter,
            effects: vec![effect],
        };
    }
    Ok(effect)
}

fn parse_for_each_object_subject(
    subject_tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(subject_tokens);
    if subject_words.is_empty() {
        return Ok(None);
    }

    let mut filter_tokens = if subject_words.starts_with(&["for", "each"]) {
        &subject_tokens[2..]
    } else if subject_words.first() == Some(&"each") {
        &subject_tokens[1..]
    } else {
        return Ok(None);
    };
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("of"))
    {
        filter_tokens = &filter_tokens[1..];
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let filter_words = words(filter_tokens);
    if filter_words.is_empty() {
        return Ok(None);
    }

    // Player-iteration forms are handled by dedicated ForEachPlayer/Opponent parsers.
    if filter_words.starts_with(&["player"])
        || filter_words.starts_with(&["players"])
        || filter_words.starts_with(&["opponent"])
        || filter_words.starts_with(&["opponents"])
        || filter_words.starts_with(&["target", "player"])
        || filter_words.starts_with(&["target", "players"])
        || filter_words.starts_with(&["target", "opponent"])
        || filter_words.starts_with(&["target", "opponents"])
    {
        return Ok(None);
    }

    Ok(Some(parse_object_filter(filter_tokens, false)?))
}

fn is_target_player_dealt_damage_by_this_turn_subject(words: &[&str]) -> bool {
    if words.len() < 8 {
        return false;
    }
    if !(words.starts_with(&["target", "player"]) || words.starts_with(&["target", "players"])) {
        return false;
    }
    words
        .windows(6)
        .any(|window| window == ["dealt", "damage", "by", "this", "creature", "this"])
        && words.windows(2).any(|window| window == ["this", "turn"])
}

fn is_mana_replacement_clause_words(words: &[&str]) -> bool {
    let has_if = words.contains(&"if");
    let has_tap = words.contains(&"tap") || words.contains(&"taps");
    let has_for_mana = words.windows(2).any(|window| window == ["for", "mana"]);
    let has_produce = words.contains(&"produce") || words.contains(&"produces");
    let has_instead = words.contains(&"instead");
    has_if && has_tap && has_for_mana && has_produce && has_instead
}

fn is_mana_trigger_additional_clause_words(words: &[&str]) -> bool {
    let has_whenever = words.contains(&"whenever");
    let has_tap = words.contains(&"tap") || words.contains(&"taps");
    let has_for_mana = words.windows(2).any(|window| window == ["for", "mana"]);
    let has_add = words.contains(&"add") || words.contains(&"adds");
    let has_additional = words.contains(&"additional");
    has_whenever && has_tap && has_for_mana && has_add && has_additional
}

fn parse_has_base_power_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = words(tokens);
    let Some(has_idx) = words_all
        .iter()
        .position(|word| *word == "has" || *word == "have")
    else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..has_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(subject_tokens);

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 3 || !rest_words.starts_with(&["base", "power"]) {
        return Ok(None);
    }
    if rest_words.get(2).is_some_and(|word| *word == "and") {
        return Ok(None);
    }

    let has_token_idx = tokens
        .iter()
        .position(|token| token.is_word("has") || token.is_word("have"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing has/have token in base-power clause (clause: '{}')",
                words_all.join(" ")
            ))
        })?;
    let rest_tokens = &tokens[has_token_idx + 1..];

    let mut seen_words = 0usize;
    let mut value_token_idx = None;
    for (idx, token) in rest_tokens.iter().enumerate() {
        if token.as_word().is_some() {
            seen_words += 1;
            if seen_words == 3 {
                value_token_idx = Some(idx);
                break;
            }
        }
    }
    let Some(value_token_idx) = value_token_idx else {
        return Ok(None);
    };
    let (power, value_used) = parse_value(&rest_tokens[value_token_idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "invalid base power value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let tail_words: Vec<&str> = rest_tokens[value_token_idx + value_used..]
        .iter()
        .filter_map(Token::as_word)
        .collect();
    if tail_words.is_empty() {
        let has_target_subject = subject_words.contains(&"target");
        let has_leading_until_eot = subject_words.starts_with(&["until", "end", "of", "turn"]);
        let has_temporal_words = words_all
            .windows(4)
            .any(|window| window == ["until", "end", "of", "turn"])
            || words_all
                .windows(2)
                .any(|window| window == ["this", "turn"] || window == ["next", "turn"]);
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    } else if tail_words.as_slice() != ["until", "end", "of", "turn"] {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_tokens: Vec<Token> = if subject_words.starts_with(&["until", "end", "of", "turn"]) {
        let mut skip_idx = 4usize;
        if subject_tokens
            .get(skip_idx)
            .is_some_and(|token| matches!(token, Token::Comma(_)))
        {
            skip_idx += 1;
        }
        trim_commas(&subject_tokens[skip_idx..]).to_vec()
    } else {
        subject_tokens.to_vec()
    };
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::SetBasePower {
        power,
        target,
        duration: Until::EndOfTurn,
    }))
}

fn parse_has_base_power_toughness_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let words_all = words(tokens);
    let Some(has_idx) = words_all
        .iter()
        .position(|word| *word == "has" || *word == "have")
    else {
        return Ok(None);
    };
    let subject_tokens = &tokens[..has_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }
    let subject_words = words(subject_tokens);

    let rest_words = &words_all[has_idx + 1..];
    if rest_words.len() < 5 || !rest_words.starts_with(&["base", "power", "and", "toughness"]) {
        return Ok(None);
    }

    let (power, toughness) = parse_pt_modifier(rest_words[4]).map_err(|_| {
        CardTextError::ParseError(format!(
            "invalid base power/toughness value (clause: '{}')",
            words_all.join(" ")
        ))
    })?;

    let tail = &rest_words[5..];
    if tail.is_empty() {
        let has_target_subject = subject_words.contains(&"target");
        let has_leading_until_eot = subject_words.starts_with(&["until", "end", "of", "turn"]);
        let has_temporal_words = words_all
            .windows(4)
            .any(|window| window == ["until", "end", "of", "turn"])
            || words_all
                .windows(2)
                .any(|window| window == ["this", "turn"] || window == ["next", "turn"]);
        if !has_target_subject && !has_leading_until_eot && !has_temporal_words {
            return Ok(None);
        }
    }
    if !tail.is_empty() && tail != ["until", "end", "of", "turn"] {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target_tokens: Vec<Token> = if subject_words.starts_with(&["until", "end", "of", "turn"]) {
        let mut skip_idx = 4usize;
        if subject_tokens
            .get(skip_idx)
            .is_some_and(|token| matches!(token, Token::Comma(_)))
        {
            skip_idx += 1;
        }
        trim_commas(&subject_tokens[skip_idx..]).to_vec()
    } else {
        subject_tokens.to_vec()
    };
    let target = parse_target_phrase(&target_tokens)?;
    Ok(Some(EffectAst::SetBasePowerToughness {
        power: Value::Fixed(power),
        toughness: Value::Fixed(toughness),
        target,
        duration: Until::EndOfTurn,
    }))
}

fn parse_get_for_each_count_filter(
    tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let mut for_each_idx = None;
    for idx in 0..tokens.len().saturating_sub(1) {
        if tokens[idx].is_word("for") && tokens[idx + 1].is_word("each") {
            for_each_idx = Some(idx);
            break;
        }
    }

    let Some(idx) = for_each_idx else {
        return Ok(None);
    };

    let mut filter_tokens = &tokens[idx + 2..];
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    }

    let mut other = false;
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("other") || token.is_word("another"))
    {
        other = true;
        filter_tokens = &filter_tokens[1..];
    }

    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing filter after 'for each' in gets clause".to_string(),
        ));
    }

    Ok(Some(parse_object_filter(filter_tokens, other)?))
}

fn value_contains_unbound_x(value: &Value) -> bool {
    matches!(value, Value::X | Value::XTimes(_))
}

fn replace_unbound_x_with_value(
    value: Value,
    replacement: &Value,
    clause: &str,
) -> Result<Value, CardTextError> {
    match value {
        Value::X => Ok(replacement.clone()),
        Value::XTimes(multiplier) => {
            if multiplier == 1 {
                return Ok(replacement.clone());
            }
            if let Value::Fixed(fixed) = replacement {
                return Ok(Value::Fixed(fixed * multiplier));
            }
            Err(CardTextError::ParseError(format!(
                "unsupported signed dynamic X replacement in gets clause (clause: '{}')",
                clause
            )))
        }
        other => Ok(other),
    }
}

fn parse_get_modifier_values_with_tail(
    modifier_tokens: &[Token],
    power: Value,
    toughness: Value,
) -> Result<(Value, Value, Until), CardTextError> {
    let clause = words(modifier_tokens).join(" ");
    let mut out_power = power;
    let mut out_toughness = toughness;
    let duration = Until::EndOfTurn;

    if modifier_tokens.is_empty() {
        return Ok((out_power, out_toughness, duration));
    }

    let after_modifier = &modifier_tokens[1..];
    let after_modifier_words = words(after_modifier);
    let until_word_count = if after_modifier_words.starts_with(&["until", "end", "of", "turn"]) {
        4usize
    } else {
        0usize
    };
    let tail_start = token_index_for_word_index(after_modifier, until_word_count)
        .unwrap_or(after_modifier.len());
    let tail_tokens = trim_commas(&after_modifier[tail_start..]);

    if tail_tokens.is_empty() {
        return Ok((out_power, out_toughness, duration));
    }

    let tail_words = words(&tail_tokens);
    if !tail_words.starts_with(&["where", "x", "is"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing gets clause (clause: '{}')",
            clause
        )));
    }

    if !value_contains_unbound_x(&out_power) && !value_contains_unbound_x(&out_toughness) {
        return Err(CardTextError::ParseError(format!(
            "where-X gets clause missing X modifier (clause: '{}')",
            clause
        )));
    }

    let x_value = parse_where_x_value_clause(&tail_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-X gets clause (clause: '{}')",
            clause
        ))
    })?;
    out_power = replace_unbound_x_with_value(out_power, &x_value, &clause)?;
    out_toughness = replace_unbound_x_with_value(out_toughness, &x_value, &clause)?;

    Ok((out_power, out_toughness, duration))
}

fn force_implicit_token_controller_you(effects: &mut [EffectAst]) {
    for effect in effects {
        match effect {
            EffectAst::CreateToken { player, .. }
            | EffectAst::CreateTokenWithMods { player, .. }
            | EffectAst::CreateTokenCopy { player, .. }
            | EffectAst::CreateTokenCopyFromSource { player, .. } => {
                if matches!(player, PlayerAst::Implicit) {
                    *player = PlayerAst::You;
                }
            }
            EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::MayByTaggedController { effects, .. }
            | EffectAst::IfResult { effects, .. }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachObject { effects, .. }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachOpponentDoesNot { effects }
            | EffectAst::ForEachPlayerDoesNot { effects }
            | EffectAst::ForEachOpponentDid { effects }
            | EffectAst::ForEachPlayerDid { effects }
            | EffectAst::ForEachTaggedPlayer { effects, .. }
            | EffectAst::DelayedUntilNextEndStep { effects, .. }
            | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
            | EffectAst::DelayedUntilEndOfCombat { effects }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects }
            | EffectAst::UnlessPays { effects, .. }
            | EffectAst::VoteOption { effects, .. } => force_implicit_token_controller_you(effects),
            EffectAst::UnlessAction {
                effects,
                alternative,
                ..
            } => {
                force_implicit_token_controller_you(effects);
                force_implicit_token_controller_you(alternative);
            }
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                force_implicit_token_controller_you(if_true);
                force_implicit_token_controller_you(if_false);
            }
            _ => {}
        }
    }
}

fn parse_for_each_opponent_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let start = if clause_words.starts_with(&["for", "each", "opponent"])
        || clause_words.starts_with(&["for", "each", "opponents"])
    {
        3
    } else if clause_words.starts_with(&["each", "opponent"])
        || clause_words.starts_with(&["each", "opponents"])
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    let inner_words = words(&inner_tokens);
    if inner_words.starts_with(&["who", "has", "less", "life", "than", "you"]) {
        let effect_start =
            token_index_for_word_index(&inner_tokens, 6).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who has less life than you' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
            let stripped = remove_first_word(&effect_tokens, "may");
            let inner_effects = parse_effect_chain_inner(&stripped)?;
            vec![EffectAst::May {
                effects: inner_effects,
            }]
        } else {
            parse_effect_chain(&effect_tokens)?
        };
        force_implicit_token_controller_you(&mut branch_effects);
        return Ok(Some(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::Conditional {
                predicate: PredicateAst::PlayerHasLessLifeThanYou {
                    player: PlayerAst::That,
                },
                if_true: branch_effects,
                if_false: Vec::new(),
            }],
        }));
    }

    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = inner_words
            .windows(2)
            .position(|window| window == ["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each opponent who ... this way' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        return Ok(Some(EffectAst::ForEachOpponentDid { effects }));
    }

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain(&inner_tokens)?
    };
    Ok(Some(EffectAst::ForEachOpponent { effects }))
}

fn parse_for_each_player_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let mut clause_tokens = tokens;
    let mut clause_words = words(clause_tokens);
    if clause_words.first().copied() == Some("then") {
        clause_tokens = &clause_tokens[1..];
        clause_words = words(clause_tokens);
    }
    if clause_words.len() < 2 {
        return Ok(None);
    }

    let start = if clause_words.starts_with(&["for", "each", "player"])
        || clause_words.starts_with(&["for", "each", "players"])
    {
        3
    } else if clause_words.starts_with(&["each", "player"])
        || clause_words.starts_with(&["each", "players"])
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&clause_tokens[start..]);
    if inner_tokens.len() > 3
        && inner_tokens[0].is_word("who")
        && inner_tokens[1].is_word("controls")
    {
        let mut effect_start = None;
        for idx in 2..inner_tokens.len() {
            if let Some(word) = inner_tokens[idx].as_word()
                && (word == "may"
                    || find_verb(&inner_tokens[idx..]).is_some_and(|(_, verb_idx)| verb_idx == 0))
            {
                effect_start = Some(idx);
                break;
            }
        }
        let effect_start = effect_start.ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing effect clause after 'each player who controls' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

        let filter_tokens = trim_commas(&inner_tokens[2..effect_start]);
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'each player who controls' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let filter = parse_object_filter(&filter_tokens, false)?;

        let effect_tokens = trim_commas(&inner_tokens[effect_start..]);
        let branch_effects = if effect_tokens.iter().any(|token| token.is_word("may")) {
            let stripped = remove_first_word(&effect_tokens, "may");
            let inner_effects = parse_effect_chain_inner(&stripped)?;
            vec![EffectAst::May {
                effects: inner_effects,
            }]
        } else {
            parse_effect_chain_inner(&effect_tokens)?
        };

        let effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::PlayerControls {
                player: PlayerAst::That,
                filter,
            },
            if_true: branch_effects,
            if_false: Vec::new(),
        }];
        return Ok(Some(EffectAst::ForEachPlayer { effects }));
    }

    let inner_words = words(&inner_tokens);
    if inner_words.first().copied() == Some("who")
        && let Some(this_way_idx) = inner_words
            .windows(2)
            .position(|window| window == ["this", "way"])
    {
        let effect_start = this_way_idx + 2;
        let effect_token_start =
            token_index_for_word_index(&inner_tokens, effect_start).unwrap_or(inner_tokens.len());
        let effect_tokens = trim_commas(&inner_tokens[effect_token_start..]);
        if effect_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing effect after 'each player who ... this way' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effects = parse_effect_chain_inner(&effect_tokens)?;
        return Ok(Some(EffectAst::ForEachPlayerDid { effects }));
    }

    let effects = if inner_tokens.iter().any(|token| token.is_word("may")) {
        let stripped = remove_first_word(&inner_tokens, "may");
        let inner_effects = parse_effect_chain_inner(&stripped)?;
        vec![EffectAst::May {
            effects: inner_effects,
        }]
    } else {
        parse_effect_chain_inner(&inner_tokens)?
    };

    Ok(Some(EffectAst::ForEachPlayer { effects }))
}

fn parse_double_counters_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["double", "the", "number", "of"]) {
        return Ok(None);
    }

    let counters_idx = tokens
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counters keyword (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    if counters_idx <= 4 {
        return Err(CardTextError::ParseError(format!(
            "missing counter type (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let counter_type =
        parse_counter_type_from_tokens(&tokens[4..counters_idx]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported counter type in double-counters clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let on_idx = tokens[counters_idx + 1..]
        .iter()
        .position(|token| token.is_word("on"))
        .map(|offset| counters_idx + 1 + offset)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing 'on' in double-counters clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let mut filter_tokens = &tokens[on_idx + 1..];
    if filter_tokens
        .first()
        .is_some_and(|token| token.is_word("each") || token.is_word("all"))
    {
        filter_tokens = &filter_tokens[1..];
    }
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing filter in double-counters clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(EffectAst::DoubleCountersOnEach {
        counter_type,
        filter,
    }))
}

fn parse_distribute_counters_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    parse_distribute_counters_sentence(tokens)
}

fn parse_prevent_next_damage_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.first().copied() != Some("prevent") {
        return Ok(None);
    }

    let mut idx = 1usize;
    if clause_words.get(idx) == Some(&"the") {
        idx += 1;
    }
    if clause_words.get(idx) != Some(&"next") {
        return Ok(None);
    }
    idx += 1;

    let amount_token = Token::Word(
        clause_words
            .get(idx)
            .copied()
            .unwrap_or_default()
            .to_string(),
        TextSpan::synthetic(),
    );
    let Some((amount, amount_used)) = parse_value(&[amount_token]) else {
        return Err(CardTextError::ParseError(format!(
            "missing prevent damage amount (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    idx += amount_used;

    if clause_words.get(idx) != Some(&"damage") {
        return Ok(None);
    }
    idx += 1;

    if clause_words.get(idx..idx + 4) != Some(["that", "would", "be", "dealt"].as_slice()) {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage clause tail (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 4;

    if clause_words.get(idx) != Some(&"to") {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-next damage target scope (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    idx += 1;

    let this_turn_rel = clause_words[idx..]
        .windows(2)
        .position(|window| window == ["this", "turn"])
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported prevent-next damage duration (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
    let this_turn_idx = idx + this_turn_rel;
    if this_turn_idx + 2 != clause_words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing prevent-next damage clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = &clause_words[idx..this_turn_idx];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-next damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target_tokens = target_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventDamage {
        amount,
        target,
        duration: Until::EndOfTurn,
    }))
}

fn parse_prevent_all_damage_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let prefix = ["prevent", "all", "damage", "that", "would", "be", "dealt", "to"];
    if !clause_words.starts_with(&prefix) {
        return Ok(None);
    }
    if clause_words.len() <= prefix.len() + 1 {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words[clause_words.len().saturating_sub(2)..] != ["this", "turn"] {
        return Err(CardTextError::ParseError(format!(
            "unsupported prevent-all damage duration (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = &clause_words[prefix.len()..clause_words.len() - 2];
    if target_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing prevent-all damage target (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_tokens = target_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let target = parse_target_phrase(&target_tokens)?;

    Ok(Some(EffectAst::PreventAllDamageToTarget {
        target,
        duration: Until::EndOfTurn,
    }))
}

fn parse_can_attack_as_though_no_defender_clause(
    tokens: &[Token],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(can_idx) = clause_words.iter().position(|word| *word == "can") else {
        return Ok(None);
    };
    let subject_words = &clause_words[..can_idx];
    let tail = &clause_words[can_idx..];
    let has_core = tail.starts_with(&["can", "attack"])
        && tail.windows(2).any(|window| window == ["as", "though"])
        && tail.contains(&"turn")
        && tail.contains(&"have")
        && tail.last().copied() == Some("defender");
    if !has_core {
        return Ok(None);
    }

    let target = if subject_words.is_empty() {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        let subject_tokens = subject_words
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        parse_target_phrase(&subject_tokens)?
    };

    Ok(Some(EffectAst::GrantAbilitiesToTarget {
        target,
        abilities: vec![StaticAbility::can_attack_as_though_no_defender()],
        duration: Until::EndOfTurn,
    }))
}

fn parse_copy_spell_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    let Some(copy_idx) = tokens
        .iter()
        .position(|token| token.is_word("copy") || token.is_word("copies"))
    else {
        return Ok(None);
    };
    if !clause_words.contains(&"spell") && !clause_words.contains(&"spells") {
        return Ok(None);
    }

    let subject = parse_subject(&tokens[..copy_idx]);
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => PlayerAst::Implicit,
    };

    let tail = &tokens[copy_idx + 1..];
    if tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut split_idx = None;
    for idx in 0..tail.len() {
        if !tail[idx].is_word("and") {
            continue;
        }
        let mut after = words(&tail[idx + 1..]);
        if after.first().copied() == Some("may") {
            after.remove(0);
        }
        if after.first().copied() == Some("choose")
            && after
                .iter()
                .any(|word| *word == "target" || *word == "targets")
            && after.iter().any(|word| *word == "copy")
        {
            split_idx = Some(idx);
            break;
        }
    }

    let copy_target_tokens = trim_commas(if let Some(idx) = split_idx {
        &tail[..idx]
    } else {
        tail
    });
    if copy_target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing spell target in copy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_words = words(&copy_target_tokens);
    let target = if target_words.as_slice() == ["this", "spell"]
        || target_words.as_slice() == ["that", "spell"]
    {
        TargetAst::Source(None)
    } else {
        parse_target_phrase(&copy_target_tokens)?
    };

    let mut may_choose_new_targets = false;
    if let Some(idx) = split_idx {
        let mut choose_words = words(&tail[idx + 1..]);
        if choose_words.first().copied() == Some("may") {
            may_choose_new_targets = true;
            choose_words.remove(0);
        }
        let has_choose = choose_words.first().copied() == Some("choose");
        let has_new = choose_words.contains(&"new");
        let has_target = choose_words
            .iter()
            .any(|word| *word == "target" || *word == "targets");
        let has_copy = choose_words.contains(&"copy");
        if !has_choose || !has_target || !has_copy {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing copy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if !has_new {
            return Err(CardTextError::ParseError(format!(
                "missing 'new' in copy retarget clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    Ok(Some(EffectAst::CopySpell {
        target,
        count: Value::Fixed(1),
        player,
        may_choose_new_targets,
    }))
}

fn parse_verb_first_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(Token::Word(word, _)) = tokens.first() else {
        return Ok(None);
    };

    let verb = match word.as_str() {
        "add" => Verb::Add,
        "move" => Verb::Move,
        "counter" => Verb::Counter,
        "destroy" => Verb::Destroy,
        "exile" => Verb::Exile,
        "draw" => Verb::Draw,
        "deal" => Verb::Deal,
        "sacrifice" => Verb::Sacrifice,
        "create" => Verb::Create,
        "investigate" => Verb::Investigate,
        "proliferate" => Verb::Proliferate,
        "tap" => Verb::Tap,
        "attach" => Verb::Attach,
        "untap" => Verb::Untap,
        "scry" => Verb::Scry,
        "discard" => Verb::Discard,
        "transform" => Verb::Transform,
        "regenerate" => Verb::Regenerate,
        "mill" => Verb::Mill,
        "get" => Verb::Get,
        "remove" => Verb::Remove,
        "return" => Verb::Return,
        "exchange" => Verb::Exchange,
        "become" => Verb::Become,
        "skip" => Verb::Skip,
        "surveil" => Verb::Surveil,
        "shuffle" => Verb::Shuffle,
        "pay" => Verb::Pay,
        "goad" => Verb::Goad,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
}

fn parse_keyword_mechanic_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut start = 0usize;
    if tokens.get(start).is_some_and(|token| token.is_word("then")) {
        start += 1;
    }
    if tokens.get(start).is_some_and(|token| token.is_word("you")) {
        start += 1;
    }
    if start >= tokens.len() {
        return Ok(None);
    }

    let clause_tokens = &tokens[start..];
    let clause_words = words(clause_tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }

    if clause_words.first() == Some(&"amass") {
        return Err(CardTextError::ParseError(format!(
            "unsupported amass mechanic (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words.starts_with(&["open", "an", "attraction"])
        || clause_words.starts_with(&["opens", "an", "attraction"])
    {
        return Ok(Some(EffectAst::OpenAttraction));
    }

    if clause_words == ["manifest", "dread"] {
        return Ok(Some(EffectAst::ManifestDread));
    }

    if matches!(clause_words.first().copied(), Some("bolster" | "support" | "adapt")) {
        let keyword = clause_words[0];
        let (amount, used) = parse_number(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing numeric amount for {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing {keyword} clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let effect = match keyword {
            "bolster" => EffectAst::Bolster { amount },
            "support" => EffectAst::Support { amount },
            "adapt" => EffectAst::Adapt { amount },
            _ => unreachable!(),
        };
        return Ok(Some(effect));
    }

    if clause_words.first() == Some(&"fateseal") {
        let (count, used) = parse_value(&clause_tokens[1..]).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing amount for fateseal clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if 1 + used != clause_tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing fateseal clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        return Ok(Some(EffectAst::Scry {
            count,
            player: PlayerAst::Opponent,
        }));
    }

    if matches!(clause_words.last().copied(), Some("explore" | "explores")) {
        let subject_tokens = &clause_tokens[..clause_tokens.len().saturating_sub(1)];
        let subject_words = words(subject_tokens);
        let target = if subject_words.is_empty()
            || subject_words == ["it"]
            || subject_words == ["this"]
            || subject_words == ["this", "creature"]
            || subject_words == ["this", "permanent"]
        {
            TargetAst::Source(span_from_tokens(subject_tokens))
        } else {
            parse_target_phrase(subject_tokens)?
        };
        return Ok(Some(EffectAst::Explore { target }));
    }

    Ok(None)
}

fn parse_connive_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let Some(connive_idx) = tokens
        .iter()
        .rposition(|token| token.is_word("connive") || token.is_word("connives"))
    else {
        return Ok(None);
    };

    // We currently only support trailing "connive/connives" clauses.
    if tokens[connive_idx + 1..]
        .iter()
        .any(|token| token.as_word().is_some())
    {
        return Ok(None);
    }

    let subject_tokens = &tokens[..connive_idx];
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let subject_words = words(subject_tokens);
    if subject_words == ["each", "creature", "that", "convoked", "this", "spell"] {
        return Ok(Some(EffectAst::ForEachTagged {
            tag: TagKey::from("convoked_this_spell"),
            effects: vec![EffectAst::ConniveIterated],
        }));
    }

    let target = parse_target_phrase(subject_tokens)?;
    Ok(Some(EffectAst::Connive { target }))
}

fn find_verb(tokens: &[Token]) -> Option<(Verb, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "counter" | "counters")
            && tokens
                .get(idx + 1)
                .and_then(Token::as_word)
                .is_some_and(|next| matches!(next, "on" | "from" | "among"))
        {
            continue;
        }
        let verb = match word {
            "adds" | "add" => Verb::Add,
            "moves" | "move" => Verb::Move,
            "deals" | "deal" => Verb::Deal,
            "draws" | "draw" => Verb::Draw,
            "counters" | "counter" => Verb::Counter,
            "destroys" | "destroy" => Verb::Destroy,
            "exiles" | "exile" => Verb::Exile,
            "reveals" | "reveal" => Verb::Reveal,
            "loses" | "lose" => Verb::Lose,
            "gains" | "gain" => Verb::Gain,
            "puts" | "put" => Verb::Put,
            "sacrifices" | "sacrifice" => Verb::Sacrifice,
            "creates" | "create" => Verb::Create,
            "investigates" | "investigate" => Verb::Investigate,
            "proliferates" | "proliferate" => Verb::Proliferate,
            "taps" | "tap" => Verb::Tap,
            "attaches" | "attach" => Verb::Attach,
            "untaps" | "untap" => Verb::Untap,
            "scries" | "scry" => Verb::Scry,
            "discards" | "discard" => Verb::Discard,
            "transforms" | "transform" => Verb::Transform,
            "regenerates" | "regenerate" => Verb::Regenerate,
            "mills" | "mill" => Verb::Mill,
            "gets" | "get" => Verb::Get,
            "removes" | "remove" => Verb::Remove,
            "returns" | "return" => Verb::Return,
            "exchanges" | "exchange" => Verb::Exchange,
            "becomes" | "become" => Verb::Become,
            "skips" | "skip" => Verb::Skip,
            "surveils" | "surveil" => Verb::Surveil,
            "shuffles" | "shuffle" => Verb::Shuffle,
            "pays" | "pay" => Verb::Pay,
            "goads" | "goad" => Verb::Goad,
            _ => continue,
        };
        return Some((verb, idx));
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SubjectAst {
    This,
    Player(PlayerAst),
}

fn parse_subject(tokens: &[Token]) -> SubjectAst {
    let words = words(tokens);
    if words.is_empty() {
        return SubjectAst::This;
    }

    let mut start = 0usize;
    if words.starts_with(&["any", "number", "of"]) {
        start = 3;
    }

    let mut slice = &words[start..];
    while slice
        .first()
        .is_some_and(|word| *word == "then" || *word == "and")
    {
        slice = &slice[1..];
    }
    if let Some(have_idx) = slice
        .iter()
        .position(|word| *word == "have" || *word == "has")
    {
        if have_idx + 1 < slice.len() {
            slice = &slice[have_idx + 1..];
        }
    }

    if slice.starts_with(&["you"]) {
        return SubjectAst::Player(PlayerAst::You);
    }

    if slice.starts_with(&["target", "opponent"]) || slice.starts_with(&["target", "opponents"]) {
        return SubjectAst::Player(PlayerAst::TargetOpponent);
    }

    if slice.starts_with(&["target", "player"]) || slice.starts_with(&["target", "players"]) {
        return SubjectAst::Player(PlayerAst::Target);
    }

    if slice.starts_with(&["opponent"])
        || slice.starts_with(&["opponents"])
        || slice.starts_with(&["an", "opponent"])
    {
        return SubjectAst::Player(PlayerAst::Opponent);
    }

    if slice.starts_with(&["defending", "player"]) {
        return SubjectAst::Player(PlayerAst::Defending);
    }
    if slice.ends_with(&["defending", "player"]) {
        return SubjectAst::Player(PlayerAst::Defending);
    }
    if slice.starts_with(&["attacking", "player"])
        || slice.starts_with(&["the", "attacking", "player"])
    {
        return SubjectAst::Player(PlayerAst::Attacking);
    }
    if slice.ends_with(&["attacking", "player"]) {
        return SubjectAst::Player(PlayerAst::Attacking);
    }

    if slice.starts_with(&["that", "player"]) {
        return SubjectAst::Player(PlayerAst::That);
    }

    // Handle possessive references like "that creature's controller" /
    // "that permanent's controller" after tokenizer apostrophe normalization.
    if slice.len() >= 3
        && slice[0] == "that"
        && (slice[2] == "controller" || slice[2] == "owner")
        && (slice[1] == "creatures"
            || slice[1] == "permanents"
            || slice[1] == "sources"
            || slice[1] == "spells")
    {
        let player = if slice[2] == "owner" {
            PlayerAst::ItsOwner
        } else {
            PlayerAst::ItsController
        };
        return SubjectAst::Player(player);
    }

    if slice.starts_with(&["its", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if slice.starts_with(&["its", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if slice.starts_with(&["their", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }
    if slice.ends_with(&["its", "controller"]) || slice.ends_with(&["their", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }
    if slice.ends_with(&["its", "owner"]) || slice.ends_with(&["their", "owner"]) {
        return SubjectAst::Player(PlayerAst::ItsOwner);
    }

    if slice.starts_with(&["this"]) || slice.starts_with(&["thiss"]) {
        return SubjectAst::This;
    }

    SubjectAst::This
}

fn parse_effect_with_verb(
    verb: Verb,
    subject: Option<SubjectAst>,
    tokens: &[Token],
) -> Result<EffectAst, CardTextError> {
    match verb {
        Verb::Add => parse_add_mana(tokens, subject),
        Verb::Move => parse_move(tokens),
        Verb::Deal => parse_deal_damage(tokens),
        Verb::Draw => parse_draw(tokens, subject),
        Verb::Counter => parse_counter(tokens),
        Verb::Destroy => parse_destroy(tokens),
        Verb::Exile => parse_exile(tokens, subject),
        Verb::Reveal => parse_reveal(tokens, subject),
        Verb::Lose => parse_lose_life(tokens, subject),
        Verb::Gain => {
            if tokens.first().is_some_and(|token| token.is_word("control")) {
                parse_gain_control(tokens, subject)
            } else {
                parse_gain_life(tokens, subject)
            }
        }
        Verb::Put => {
            if tokens
                .iter()
                .any(|token| token.is_word("counter") || token.is_word("counters"))
            {
                parse_put_counters(tokens)
            } else {
                parse_put_into_hand(tokens, subject)
            }
        }
        Verb::Sacrifice => parse_sacrifice(tokens, subject),
        Verb::Create => parse_create(tokens, subject),
        Verb::Investigate => parse_investigate(tokens),
        Verb::Proliferate => Ok(EffectAst::Proliferate),
        Verb::Tap => parse_tap(tokens),
        Verb::Attach => parse_attach(tokens),
        Verb::Untap => parse_untap(tokens),
        Verb::Scry => parse_scry(tokens, subject),
        Verb::Discard => parse_discard(tokens, subject),
        Verb::Transform => parse_transform(tokens),
        Verb::Regenerate => parse_regenerate(tokens),
        Verb::Mill => parse_mill(tokens, subject),
        Verb::Get => parse_get(tokens, subject),
        Verb::Remove => parse_remove(tokens),
        Verb::Return => parse_return(tokens),
        Verb::Exchange => parse_exchange(tokens),
        Verb::Become => parse_become(tokens, subject),
        Verb::Skip => parse_skip(tokens, subject),
        Verb::Surveil => parse_surveil(tokens, subject),
        Verb::Shuffle => parse_shuffle(tokens, subject),
        Verb::Pay => parse_pay(tokens, subject),
        Verb::Goad => parse_goad(tokens),
    }
}

fn parse_shuffle(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    fn is_simple_library_phrase(words: &[&str]) -> bool {
        matches!(
            words,
            ["library"]
                | ["your", "library"]
                | ["their", "library"]
                | ["that", "player", "library"]
                | ["that", "players", "library"]
                | ["its", "owner", "library"]
                | ["its", "owners", "library"]
                | ["his", "or", "her", "library"]
        )
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    if tokens.is_empty() {
        if matches!(subject, Some(SubjectAst::Player(_))) {
            return Ok(EffectAst::ShuffleLibrary { player });
        }
        return Err(CardTextError::ParseError(
            "unsupported implicit shuffle clause".to_string(),
        ));
    }

    let clause_words = words(tokens);
    if clause_words.contains(&"graveyard")
        || clause_words.contains(&"cards")
        || clause_words.contains(&"card")
        || clause_words.contains(&"into")
        || clause_words.contains(&"from")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported shuffle clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if is_simple_library_phrase(&clause_words) {
        return Ok(EffectAst::ShuffleLibrary { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported shuffle clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_goad(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let target_tokens = trim_commas(tokens);
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing goad target".to_string(),
        ));
    }

    let target_words = words(&target_tokens);
    if target_words.as_slice() == ["it"] || target_words.as_slice() == ["them"] {
        return Ok(EffectAst::Goad {
            target: TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
        });
    }

    let target = parse_target_phrase(&target_tokens)?;
    if matches!(
        target,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "goad target must be a creature (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::Goad { target })
}

fn parse_attach_object_phrase(tokens: &[Token]) -> Result<TargetAst, CardTextError> {
    let object_words = words(tokens);
    let object_span = span_from_tokens(tokens);
    if object_words.is_empty() {
        return Err(CardTextError::ParseError(
            "missing object to attach".to_string(),
        ));
    }

    let is_source_attachment = is_source_reference_words(&object_words)
        || object_words.starts_with(&["this", "equipment"])
        || object_words.starts_with(&["this", "aura"])
        || object_words.starts_with(&["this", "enchantment"])
        || object_words.starts_with(&["this", "artifact"]);
    if is_source_attachment {
        return Ok(TargetAst::Source(object_span));
    }

    if matches!(object_words.as_slice(), ["it"] | ["them"]) {
        return Ok(TargetAst::Tagged(TagKey::from(IT_TAG), object_span));
    }

    let mut tagged_filter = ObjectFilter::default();
    if matches!(object_words.as_slice(), ["that", "equipment"] | ["those", "equipment"]) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
        tagged_filter.subtypes.push(Subtype::Equipment);
    } else if matches!(object_words.as_slice(), ["that", "aura"] | ["those", "auras"]) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
        tagged_filter.subtypes.push(Subtype::Aura);
    } else if matches!(object_words.as_slice(), ["that", "artifact"] | ["those", "artifacts"]) {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Artifact);
    } else if object_words.as_slice() == ["that", "enchantment"] {
        tagged_filter.zone = Some(Zone::Battlefield);
        tagged_filter.card_types.push(CardType::Enchantment);
    }

    if tagged_filter.zone.is_some() {
        tagged_filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(IT_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(TargetAst::Object(tagged_filter, object_span, None));
    }

    if tokens.first().is_some_and(|token| token.is_word("target"))
        && let Some(attached_idx) = tokens.iter().position(|token| token.is_word("attached"))
        && tokens
            .get(attached_idx + 1)
            .is_some_and(|token| token.is_word("to"))
    {
        let head_tokens = trim_commas(&tokens[..attached_idx]);
        if !head_tokens.is_empty() {
            return parse_target_phrase(&head_tokens);
        }
    }

    parse_target_phrase(tokens)
}

fn parse_attach(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "attach clause missing object and destination".to_string(),
        ));
    }

    let Some(to_idx) = tokens.iter().rposition(|token| token.is_word("to")) else {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing destination (clause: '{}')",
            clause_words.join(" ")
        )));
    };
    if to_idx == 0 || to_idx + 1 >= tokens.len() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object_tokens = trim_commas(&tokens[..to_idx]);
    let target_tokens = trim_commas(&tokens[to_idx + 1..]);
    if object_tokens.is_empty() || target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "attach clause missing object or destination (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let object = parse_attach_object_phrase(&object_tokens)?;
    let target_words = words(&target_tokens);
    let target = if matches!(target_words.as_slice(), ["it"] | ["them"]) {
        TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens))
    } else {
        parse_target_phrase(&target_tokens)?
    };

    Ok(EffectAst::Attach { object, target })
}

fn parse_deal_damage(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let is_divided_as_you_choose_clause = clause_words.contains(&"divided")
        && clause_words.contains(&"choose")
        && clause_words.contains(&"among");
    if is_divided_as_you_choose_clause {
        return Err(CardTextError::ParseError(format!(
            "unsupported divided-damage distribution clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(effect) = parse_deal_damage_equal_to_clause(tokens)? {
        return Ok(effect);
    }
    if clause_words.starts_with(&["that", "much"]) {
        return parse_deal_damage_with_amount(
            tokens,
            Value::EventValue(EventValueSpec::Amount),
            2,
        );
    }

    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    if clause_words.starts_with(&["damage", "to", "each", "opponent"])
        && clause_words.contains(&"number")
        && clause_words.contains(&"cards")
        && clause_words.contains(&"hand")
    {
        let value = Value::CardsInHand(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: value,
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }

    Err(CardTextError::ParseError(format!(
        "missing damage amount (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_deal_damage_equal_to_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["damage", "equal", "to"]) {
        return Ok(None);
    }

    let mut target_to_idx = None;
    for idx in 3..tokens.len() {
        if !tokens[idx].is_word("to") {
            continue;
        }
        let tail_words = words(&tokens[idx + 1..]);
        if tail_words.is_empty() {
            continue;
        }
        let looks_like_target = tail_words.contains(&"target")
            || matches!(
                tail_words.first().copied(),
                Some(
                    "any"
                        | "each"
                        | "all"
                        | "it"
                        | "itself"
                        | "them"
                        | "him"
                        | "her"
                        | "that"
                        | "this"
                        | "you"
                        | "player"
                        | "opponent"
                        | "creature"
                        | "planeswalker"
                )
            );
        if looks_like_target {
            target_to_idx = Some(idx);
            break;
        }
    }

    let Some(target_to_idx) = target_to_idx else {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    };

    let amount_tokens = &tokens[..target_to_idx];
    let amount = parse_add_mana_equal_amount_value(amount_tokens)
        .or(parse_dynamic_cost_modifier_value(amount_tokens)?)
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing damage amount (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let target_tokens = &tokens[target_to_idx + 1..];
    if target_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing damage target in equal-to clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let target = parse_target_phrase(target_tokens)?;
    Ok(Some(EffectAst::DealDamage { amount, target }))
}

fn parse_deal_damage_with_amount(
    tokens: &[Token],
    amount: Value,
    used: usize,
) -> Result<EffectAst, CardTextError> {
    let rest = &tokens[used..];
    let Some(Token::Word(word, _)) = rest.first() else {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    };
    if word != "damage" {
        return Err(CardTextError::ParseError(
            "missing damage keyword".to_string(),
        ));
    }

    let mut target_tokens = &rest[1..];
    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("to"))
    {
        target_tokens = &target_tokens[1..];
    }
    if let Some(among_idx) = target_tokens
        .iter()
        .position(|token| token.is_word("among"))
    {
        let among_tail = &target_tokens[among_idx + 1..];
        if among_tail.iter().any(|token| token.is_word("target"))
            && among_tail.iter().any(|token| {
                token.is_word("player")
                    || token.is_word("players")
                    || token.is_word("creature")
                    || token.is_word("creatures")
            })
        {
            target_tokens = among_tail;
        }
    }

    if target_tokens.iter().any(|token| token.is_word("where")) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing where damage clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if let Some(instead_idx) = target_tokens.iter().position(|token| token.is_word("instead"))
        && target_tokens
            .get(instead_idx + 1)
            .is_some_and(|token| token.is_word("if"))
    {
        let pre_target_tokens = trim_commas(&target_tokens[..instead_idx]);
        let condition_tokens = trim_commas(&target_tokens[instead_idx + 2..]);
        if let Some(predicate) = parse_instead_if_control_predicate(&condition_tokens)? {
            let target = if pre_target_tokens.is_empty() {
                TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
            } else {
                parse_target_phrase(&pre_target_tokens)?
            };
            return Ok(EffectAst::Conditional {
                predicate,
                if_true: vec![EffectAst::DealDamage {
                    amount: amount.clone(),
                    target,
                }],
                if_false: Vec::new(),
            });
        }
    }

    let target_words = words(target_tokens);
    if target_words.starts_with(&["each", "of"]) {
        let each_of_tokens = &target_tokens[2..];
        let each_of_words = words(each_of_tokens);
        if each_of_words.iter().any(|word| *word == "target") {
            let target = parse_target_phrase(each_of_tokens)?;
            return Ok(EffectAst::DealDamage { amount, target });
        }
    }
    if target_words.as_slice() == ["each", "player"]
        || target_words.as_slice() == ["each", "players"]
    {
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }
    if target_words.as_slice() == ["each", "opponent"]
        || target_words.as_slice() == ["each", "opponents"]
    {
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![EffectAst::DealDamage {
                amount: amount.clone(),
                target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            }],
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all"))
        && let Some(and_each_idx) = target_words.windows(3).position(|window| {
            window == ["and", "each", "player"] || window == ["and", "each", "players"]
        })
        && and_each_idx >= 1
        && and_each_idx + 3 == target_words.len()
    {
        let filter_tokens = &target_tokens[1..and_each_idx];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        if filter.controller.is_none() {
            filter.controller = Some(PlayerFilter::IteratedPlayer);
        }
        return Ok(EffectAst::ForEachPlayer {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if target_words.starts_with(&["each", "opponent", "and", "each"])
        && target_words.contains(&"creature")
        && target_words.contains(&"planeswalker")
        && (target_words
            .windows(2)
            .any(|pair| pair == ["they", "control"])
            || target_words
                .windows(3)
                .any(|triplet| triplet == ["that", "player", "controls"]))
    {
        let mut filter = ObjectFilter::default();
        filter.card_types = vec![CardType::Creature, CardType::Planeswalker];
        filter.controller = Some(PlayerFilter::IteratedPlayer);
        return Ok(EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::DealDamage {
                    amount: amount.clone(),
                    target: TargetAst::Player(PlayerFilter::IteratedPlayer, None),
                },
                EffectAst::DealDamageEach {
                    amount: amount.clone(),
                    filter,
                },
            ],
        });
    }

    if matches!(target_words.first(), Some(&"each") | Some(&"all")) {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing damage target filter after 'each'".to_string(),
            ));
        }
        let filter_tokens = &target_tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::DealDamageEach {
            amount: amount.clone(),
            filter,
        });
    }

    let target = parse_target_phrase(target_tokens)?;
    Ok(EffectAst::DealDamage { amount, target })
}

fn parse_instead_if_control_predicate(
    tokens: &[Token],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause_words = words(tokens);
    let starts_with_you_control = clause_words.starts_with(&["you", "control"])
        || clause_words.starts_with(&["you", "controlled"]);
    if !starts_with_you_control {
        return Ok(None);
    }

    let mut filter_tokens = &tokens[2..];
    let cut_markers: &[&[&str]] = &[&["as", "you", "cast", "this", "spell"], &["this", "turn"]];
    for marker in cut_markers {
        if let Some(idx) = words(filter_tokens)
            .windows(marker.len())
            .position(|window| window == *marker)
        {
            let cut_idx = token_index_for_word_index(filter_tokens, idx).unwrap_or(filter_tokens.len());
            filter_tokens = &filter_tokens[..cut_idx];
            break;
        }
    }
    let filter_tokens = trim_commas(filter_tokens);
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(&filter_tokens, false)?;
    Ok(Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    }))
}

fn parse_move(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["all", "counters", "from"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let from_idx = tokens
        .iter()
        .position(|token| token.is_word("from"))
        .unwrap_or(2);
    let onto_idx = tokens
        .iter()
        .position(|token| token.is_word("onto"))
        .or_else(|| tokens.iter().position(|token| token.is_word("to")))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing move destination (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;

    let from_tokens = &tokens[from_idx + 1..onto_idx];
    let to_tokens = &tokens[onto_idx + 1..];
    let from = parse_target_phrase(from_tokens)?;
    let to = parse_target_phrase(to_tokens)?;

    Ok(EffectAst::MoveAllCounters { from, to })
}

fn parse_draw(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let mut parsed_that_many_minus_one = false;
    let (mut count, used) = if clause_words.starts_with(&["that", "many"]) {
        let mut value = Value::EventValue(EventValueSpec::Amount);
        let consumed = 2usize;
        let rest = &tokens[consumed..];
        if rest
            .first()
            .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let trailing = trim_commas(&rest[1..]);
            let trailing_words = words(&trailing);
            if trailing_words.as_slice() == ["minus", "one"] {
                value = Value::EventValueOffset(EventValueSpec::Amount, -1);
                parsed_that_many_minus_one = true;
            } else if !trailing_words.is_empty()
                && !(trailing_words
                    .windows(2)
                    .any(|window| window[0] == "for" && window[1] == "each"))
            {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing draw clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
        (value, consumed)
    } else {
        parse_value(tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing draw count (clause: '{}')",
                clause_words.join(" ")
            ))
        })?
    };

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    }
    let tail = trim_commas(&rest[1..]);
    if !tail.is_empty() {
        let tail_words = words(&tail);
        if !(parsed_that_many_minus_one && tail_words.as_slice() == ["minus", "one"]) {
            let has_for_each = tail
                .windows(2)
                .any(|window| window[0].is_word("for") && window[1].is_word("each"));
            if has_for_each {
                let dynamic = parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                    CardTextError::ParseError(format!(
                        "unsupported draw for-each clause (clause: '{}')",
                        words(tokens).join(" ")
                    ))
                })?;
                match count {
                    Value::Fixed(1) => count = dynamic,
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported multiplied draw count (clause: '{}')",
                            words(tokens).join(" ")
                        )));
                    }
                }
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing draw clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Draw { count, player })
}

fn parse_counter(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.contains(&"ability")
        && (clause_words.contains(&"activated") || clause_words.contains(&"triggered"))
    {
        if clause_words == ["target", "activated", "or", "triggered", "ability"] {
            return Ok(EffectAst::CounterActivatedOrTriggeredAbility);
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-ability target clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(if_idx) = tokens.iter().position(|token| token.is_word("if")) {
        if if_idx == 0 || if_idx + 1 >= tokens.len() {
            return Err(CardTextError::ParseError(format!(
                "missing conditional counter target or predicate (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let target_tokens = trim_commas(&tokens[..if_idx]);
        let predicate_tokens = trim_commas(&tokens[if_idx + 1..]);
        let target = parse_target_phrase(&target_tokens)?;
        let predicate = parse_predicate(&predicate_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![EffectAst::Counter { target }],
            if_false: Vec::new(),
        });
    }

    if let Some(unless_idx) = tokens.iter().position(|token| token.is_word("unless")) {
        let target_tokens = &tokens[..unless_idx];
        let target = parse_target_phrase(target_tokens)?;

        let unless_tokens = &tokens[unless_idx + 1..];
        let pays_idx = unless_tokens
            .iter()
            .position(|token| token.is_word("pays"))
            .ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;

        // Parse the contiguous mana payment immediately following "pays".
        // Stop at the first non-mana word so trailing dynamic qualifiers
        // ("for each ...", "where X is ...", "plus an additional ...") do not
        // accidentally duplicate symbols.
        let mut mana = Vec::new();
        let mut trailing_start: Option<usize> = None;
        for (offset, token) in unless_tokens[pays_idx + 1..].iter().enumerate() {
            let Some(word) = token.as_word() else {
                continue;
            };
            match parse_mana_symbol(word) {
                Ok(symbol) => mana.push(symbol),
                Err(_) => {
                    trailing_start = Some(pays_idx + 1 + offset);
                    break;
                }
            }
        }

        if mana.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                words(tokens).join(" ")
            )));
        }

        if let Some(start_idx) = trailing_start {
            let trailing_words = words(&unless_tokens[start_idx..]);
            if !trailing_words.is_empty() {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing counter-unless payment clause (clause: '{}')",
                    words(tokens).join(" ")
                )));
            }
        }

        return Ok(EffectAst::CounterUnlessPays { target, mana });
    }

    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Counter { target })
}

fn parse_reveal(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let words = words(tokens);
    if words.contains(&"hand") {
        let is_full_hand_reveal = matches!(words.as_slice(), ["your", "hand"] | ["their", "hand"])
            || words.as_slice() == ["his", "or", "her", "hand"];
        if !is_full_hand_reveal {
            return Err(CardTextError::ParseError(format!(
                "unsupported reveal-hand clause (clause: '{}')",
                words.join(" ")
            )));
        }
        return Ok(EffectAst::RevealHand { player });
    }

    let has_top = words.contains(&"top");
    let has_card = words.contains(&"card");

    if !has_top || !has_card {
        return Err(CardTextError::ParseError(format!(
            "unsupported reveal clause (clause: '{}')",
            words.join(" ")
        )));
    }

    Ok(EffectAst::RevealTop { player })
}

fn parse_life_amount(tokens: &[Token], amount_kind: &str) -> Result<(Value, usize), CardTextError> {
    let clause_words = words(tokens);
    if clause_words == ["that", "much", "life"] {
        // "that much life" binds to the triggering event amount.
        return Ok((Value::EventValue(EventValueSpec::Amount), 2));
    }

    parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing {amount_kind} amount (clause: '{}')",
            clause_words.join(" ")
        ))
    })
}

fn parse_life_equal_to_value(tokens: &[Token]) -> Result<Option<Value>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["life", "equal", "to"]) {
        return Ok(None);
    }

    if let Some(value) = parse_add_mana_equal_amount_value(&tokens[1..]) {
        return Ok(Some(value));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(&tokens[1..])? {
        return Ok(Some(value));
    }

    Err(CardTextError::ParseError(format!(
        "missing life amount in equal-to clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_life_amount_from_trailing(
    base_amount: &Value,
    trailing: &[Token],
) -> Result<Option<Value>, CardTextError> {
    if trailing.is_empty() {
        return Ok(None);
    }

    if let Some(dynamic) = parse_dynamic_cost_modifier_value(trailing)? {
        if let Some(multiplier) = match base_amount {
            Value::Fixed(value) => Some(*value),
            Value::X => Some(1),
            _ => None,
        } {
            if multiplier <= 1 {
                return Ok(Some(dynamic));
            }
            if let Value::Count(filter) = dynamic {
                return Ok(Some(Value::CountScaled(filter, multiplier)));
            }
            return Ok(Some(dynamic));
        }
    }

    if let Some(where_value) = parse_where_x_value_clause(trailing) {
        match base_amount {
            Value::X | Value::Fixed(1) => return Ok(Some(where_value)),
            _ => {}
        }
    }

    Ok(None)
}

fn validate_life_keyword(rest: &[Token]) -> Result<(), CardTextError> {
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "life")
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
    }
    Ok(())
}

fn parse_lose_life(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let clause_words = words(tokens);
    if let Some(amount) = parse_life_equal_to_value(tokens)? {
        return Ok(EffectAst::LoseLife { amount, player });
    }
    if clause_words.as_slice() == ["the", "game"] {
        return Ok(EffectAst::LoseGame { player });
    }

    let (mut amount, used) = parse_life_amount(tokens, "life loss")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::LoseLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-loss clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::LoseLife { amount, player })
}

fn parse_gain_life(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    if let Some(amount) = parse_life_equal_to_value(tokens)? {
        return Ok(EffectAst::GainLife { amount, player });
    }

    let (mut amount, used) = parse_life_amount(tokens, "life gain")?;

    let rest = &tokens[used..];
    validate_life_keyword(rest)?;
    let trailing = trim_commas(&rest[1..]);
    if !trailing.is_empty() {
        let trailing_words = words(&trailing);
        if trailing_words
            .windows(6)
            .any(|window| window == ["then", "shuffle", "your", "graveyard", "into", "your"])
            && trailing_words.contains(&"library")
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing life-gain shuffle-graveyard clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        if let Some(resolved) = parse_life_amount_from_trailing(&amount, &trailing)? {
            amount = resolved;
            return Ok(EffectAst::GainLife { amount, player });
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing life-gain clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::GainLife { amount, player })
}

fn parse_gain_control(
    tokens: &[Token],
    _subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    let has_dynamic_power_bound = clause_words.contains(&"power")
        && clause_words.contains(&"number")
        && clause_words
            .windows(2)
            .any(|window| window == ["you", "control"]);
    if has_dynamic_power_bound {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic power-bound control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut idx = 0;
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("control"))
    {
        idx += 1;
    } else {
        return Err(CardTextError::ParseError(
            "missing control keyword".to_string(),
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
        idx += 1;
    }

    let duration_idx = tokens[idx..]
        .iter()
        .position(|token| token.is_word("during") || token.is_word("until"))
        .map(|offset| idx + offset)
        .or_else(|| {
            tokens[idx..]
                .windows(4)
                .position(|window| {
                    window[0].is_word("for")
                        && window[1].is_word("as")
                        && window[2].is_word("long")
                        && window[3].is_word("as")
                })
                .map(|offset| idx + offset)
        });

    let target_tokens = if let Some(dur_idx) = duration_idx {
        &tokens[idx..dur_idx]
    } else {
        &tokens[idx..]
    };
    if target_tokens
        .iter()
        .any(|token| token.is_word("if") || token.is_word("unless"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional gain-control clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let target_ast = parse_target_phrase(target_tokens)?;
    let duration_tokens = duration_idx
        .map(|dur_idx| &tokens[dur_idx..])
        .unwrap_or(&[]);
    let duration = parse_control_duration(duration_tokens)?;
    match target_ast {
        TargetAst::Player(filter, _) => Ok(EffectAst::ControlPlayer {
            player: PlayerFilter::Target(Box::new(filter)),
            duration,
        }),
        _ => {
            let until = match duration {
                ControlDurationAst::UntilEndOfTurn => Until::EndOfTurn,
                ControlDurationAst::Forever => Until::Forever,
                ControlDurationAst::AsLongAsYouControlSource => Until::YouStopControllingThis,
                ControlDurationAst::DuringNextTurn => {
                    return Err(CardTextError::ParseError(
                        "unsupported control duration for permanents".to_string(),
                    ));
                }
            };
            Ok(EffectAst::GainControl {
                target: target_ast,
                duration: until,
            })
        }
    }
}

fn parse_control_duration(tokens: &[Token]) -> Result<ControlDurationAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(ControlDurationAst::Forever);
    }

    let words = words(tokens);
    let has_for_as_long_as = words
        .windows(4)
        .any(|window| window == ["for", "as", "long", "as"]);
    if has_for_as_long_as
        && words.contains(&"you")
        && words.contains(&"control")
        && (words.contains(&"this")
            || words.contains(&"thiss")
            || words.contains(&"source")
            || words.contains(&"creature")
            || words.contains(&"permanent"))
    {
        return Ok(ControlDurationAst::AsLongAsYouControlSource);
    }

    let has_during = words.contains(&"during");
    let has_next = words.contains(&"next");
    let has_turn = words.contains(&"turn");
    if has_during && has_next && has_turn {
        return Ok(ControlDurationAst::DuringNextTurn);
    }

    let has_until = words.contains(&"until");
    let has_end = words.contains(&"end");
    if has_until && has_end && has_turn {
        return Ok(ControlDurationAst::UntilEndOfTurn);
    }

    Err(CardTextError::ParseError(
        "unsupported control duration".to_string(),
    ))
}

fn parse_put_into_hand(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let clause_words = words(tokens);
    if clause_words.contains(&"from") && clause_words.contains(&"among") {
        return Err(CardTextError::ParseError(format!(
            "unsupported put-from-among clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_it = clause_words.contains(&"it");
    let has_them = clause_words.contains(&"them");
    let has_hand = clause_words.contains(&"hand");
    let has_into = clause_words.contains(&"into");

    if has_hand && has_into && (has_it || has_them) {
        if clause_words.contains(&"rest")
            || clause_words.contains(&"graveyard")
            || clause_words.contains(&"battlefield")
            || clause_words.contains(&"library")
            || clause_words
                .windows(2)
                .any(|window| window == ["and", "the"] || window == ["and", "rest"])
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-destination put clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        return Ok(EffectAst::PutIntoHand {
            player,
            object: ObjectRefAst::It,
        });
    }

    if let Some(on_idx) = tokens.iter().position(|token| token.is_word("on"))
        && tokens
            .get(on_idx + 1)
            .is_some_and(|token| token.is_word("top"))
        && tokens
            .get(on_idx + 2)
            .is_some_and(|token| token.is_word("of"))
    {
        let target_tokens = trim_commas(&tokens[..on_idx]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let destination_words = words(&tokens[on_idx + 3..]);
        if !destination_words.contains(&"library") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'on top of' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let target = if let Some((count, used)) = parse_number(&target_tokens)
            && target_tokens
                .get(used)
                .is_some_and(|token| token.is_word("card") || token.is_word("cards"))
        {
            let inner = parse_target_phrase(&target_tokens[used..])?;
            TargetAst::WithCount(Box::new(inner), ChoiceCount::exactly(count as usize))
        } else {
            parse_target_phrase(&target_tokens)?
        };
        return Ok(EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
            battlefield_controller: ReturnControllerAst::Preserve,
        });
    }

    if let Some(onto_idx) = tokens.iter().position(|token| token.is_word("onto")) {
        let target_tokens = trim_commas(&tokens[..onto_idx]);
        if target_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing target before 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }

        let destination_words: Vec<&str> = words(&tokens[onto_idx + 1..])
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        if destination_words.first() != Some(&"battlefield") {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let destination_tail = &destination_words[1..];
        let supported_control_tail = destination_tail.is_empty()
            || destination_tail == ["under", "your", "control"]
            || destination_tail == ["under", "its", "owners", "control"]
            || destination_tail == ["under", "their", "owners", "control"]
            || destination_tail == ["under", "that", "players", "control"];
        if !supported_control_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported put destination after 'onto' (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let battlefield_controller = if destination_tail == ["under", "your", "control"] {
            ReturnControllerAst::You
        } else if destination_tail == ["under", "its", "owners", "control"]
            || destination_tail == ["under", "their", "owners", "control"]
            || destination_tail == ["under", "that", "players", "control"]
        {
            ReturnControllerAst::Owner
        } else {
            ReturnControllerAst::Preserve
        };

        if target_tokens
            .first()
            .is_some_and(|token| token.is_word("all") || token.is_word("each"))
        {
            let mut filter = parse_object_filter(&target_tokens[1..], false)?;
            let target_words = words(&target_tokens[1..]);
            if target_words.windows(2).any(|window| window == ["from", "it"]) {
                filter.zone = Some(Zone::Hand);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::You);
                }
                filter
                    .tagged_constraints
                    .retain(|constraint| constraint.tag.as_str() != IT_TAG);
            }
            if clause_words.contains(&"among") && clause_words.contains(&"them") {
                filter.zone = Some(Zone::Exile);
                if filter.owner.is_none() {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                if clause_words.contains(&"permanent") {
                    filter.card_types = vec![
                        CardType::Artifact,
                        CardType::Creature,
                        CardType::Enchantment,
                        CardType::Land,
                        CardType::Planeswalker,
                        CardType::Battle,
                    ];
                }
            }
            return Ok(EffectAst::ReturnAllToBattlefield {
                filter,
                tapped: false,
            });
        }

        return Ok(EffectAst::MoveToZone {
            target: parse_target_phrase(&target_tokens)?,
            zone: Zone::Battlefield,
            to_top: false,
            battlefield_controller,
        });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported put clause (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn parse_counter_type_word(word: &str) -> Option<CounterType> {
    match word {
        "+1/+1" => Some(CounterType::PlusOnePlusOne),
        "-1/-1" => Some(CounterType::MinusOneMinusOne),
        "-0/-1" => Some(CounterType::MinusOneMinusOne),
        "deathtouch" => Some(CounterType::Deathtouch),
        "flying" => Some(CounterType::Flying),
        "haste" => Some(CounterType::Haste),
        "hexproof" => Some(CounterType::Hexproof),
        "indestructible" => Some(CounterType::Indestructible),
        "lifelink" => Some(CounterType::Lifelink),
        "menace" => Some(CounterType::Menace),
        "reach" => Some(CounterType::Reach),
        "trample" => Some(CounterType::Trample),
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
        "stun" => Some(CounterType::Stun),
        "depletion" => Some(CounterType::Depletion),
        "storage" => Some(CounterType::Storage),
        "ki" => Some(CounterType::Ki),
        "energy" => Some(CounterType::Energy),
        "age" => Some(CounterType::Age),
        "time" => Some(CounterType::Time),
        "brain" => Some(CounterType::Brain),
        "level" => Some(CounterType::Level),
        "lore" => Some(CounterType::Lore),
        _ => None,
    }
}

fn parse_counter_type_from_tokens(tokens: &[Token]) -> Option<CounterType> {
    for token in tokens {
        if let Some(word) = token.as_word()
            && let Some(parsed) = parse_counter_type_word(word)
        {
            return Some(parsed);
        }
    }

    let token_words = words(tokens);
    for window in token_words.windows(2) {
        match window {
            ["-1", "-1"] => return Some(CounterType::MinusOneMinusOne),
            ["-0", "-1"] => return Some(CounterType::MinusOneMinusOne),
            ["+1", "+1"] => return Some(CounterType::PlusOnePlusOne),
            ["double", "strike"] => return Some(CounterType::DoubleStrike),
            ["first", "strike"] => return Some(CounterType::FirstStrike),
            _ => {}
        }
    }
    None
}

fn parse_counter_descriptor(tokens: &[Token]) -> Result<(u32, CounterType), CardTextError> {
    let descriptor = trim_commas(tokens);
    let (count, used) = parse_number(&descriptor).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            words(&descriptor).join(" ")
        ))
    })?;
    let rest = &descriptor[used..];
    if !rest
        .iter()
        .any(|token| token.is_word("counter") || token.is_word("counters"))
    {
        return Err(CardTextError::ParseError(format!(
            "missing counter keyword (clause: '{}')",
            words(&descriptor).join(" ")
        )));
    }
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            words(&descriptor).join(" ")
        ))
    })?;
    Ok((count, counter_type))
}

fn parse_put_counters(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let (count, used) = parse_number(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    let counter_type = parse_counter_type_from_tokens(rest).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported counter type (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let on_idx = rest
        .iter()
        .position(|token| token.is_word("on"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing counter target (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let target_tokens = &rest[on_idx + 1..];
    if let Some((target_count, used)) = parse_counter_target_count_prefix(target_tokens)? {
        let target_phrase = &target_tokens[used..];
        if target_phrase.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing counter target after count clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        let target = parse_target_phrase(target_phrase)?;
        return Ok(EffectAst::PutCounters {
            counter_type,
            count: Value::Fixed(count as i32),
            target,
            target_count: Some(target_count),
            distributed: false,
        });
    }

    if target_tokens
        .first()
        .is_some_and(|token| token.is_word("each"))
    {
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(EffectAst::PutCountersAll {
            counter_type,
            count: Value::Fixed(count as i32),
            filter,
        });
    }
    if let Some(for_each_idx) = (0..target_tokens.len().saturating_sub(1)).find(|idx| {
        target_tokens[*idx].is_word("for") && target_tokens[*idx + 1].is_word("each")
    }) {
        let base_target_tokens = trim_commas(&target_tokens[..for_each_idx]);
        let count_filter_tokens = trim_commas(&target_tokens[for_each_idx + 2..]);
        if !base_target_tokens.is_empty() && !count_filter_tokens.is_empty() {
            let target = parse_target_phrase(&base_target_tokens)?;
            let count_filter = parse_object_filter(&count_filter_tokens, false)?;
            return Ok(EffectAst::PutCounters {
                counter_type,
                count: Value::Count(count_filter),
                target,
                target_count: None,
                distributed: false,
            });
        }
    }
    let target = parse_target_phrase(target_tokens)?;
    Ok(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target,
        target_count: None,
        distributed: false,
    })
}

fn parse_counter_target_count_prefix(
    tokens: &[Token],
) -> Result<Option<(ChoiceCount, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let mut idx = 0usize;
    let mut each_prefix = false;

    if tokens[idx].is_word("each") {
        each_prefix = true;
        idx += 1;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
    }

    if each_prefix
        && tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic target count 'each of X target' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if each_prefix && tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        return Ok(Some((ChoiceCount::any_number(), idx)));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
    {
        idx += 2;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::any_number(), idx)));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        let Some((value, used)) = parse_number(&tokens[idx + 2..]) else {
            return Err(CardTextError::ParseError(format!(
                "missing count after 'up to' in counter target clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        };
        idx += 2 + used;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::up_to(value as usize), idx)));
    }

    if let Some((first, used_first)) = parse_number(&tokens[idx..]) {
        let mut pos = idx + used_first;
        let mut values = vec![first];
        loop {
            while matches!(tokens.get(pos), Some(Token::Comma(_))) {
                pos += 1;
            }
            if tokens.get(pos).is_some_and(|token| token.is_word("or")) {
                pos += 1;
                while matches!(tokens.get(pos), Some(Token::Comma(_))) {
                    pos += 1;
                }
            }

            let Some((next, used_next)) = parse_number(&tokens[pos..]) else {
                break;
            };
            values.push(next);
            pos += used_next;
        }

        if values.len() >= 2 {
            if tokens.get(pos).is_some_and(|token| token.is_word("of")) {
                pos += 1;
            }
            let min = values.iter().copied().min().unwrap_or(first) as usize;
            let max = values.iter().copied().max().unwrap_or(first) as usize;
            return Ok(Some((
                ChoiceCount {
                    min,
                    max: Some(max),
                    dynamic_x: false,
                },
                pos,
            )));
        }
    }

    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        idx += used;
        if tokens.get(idx).is_some_and(|token| token.is_word("of")) {
            idx += 1;
        }
        return Ok(Some((ChoiceCount::exactly(value as usize), idx)));
    }

    Ok(None)
}

fn parse_tap(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "tap clause missing target".to_string(),
        ));
    }
    let words = words(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::TapAll { filter });
    }
    // Handle "tap or untap <target>" as a choice between tapping and untapping.
    if tokens.first().is_some_and(|t| t.is_word("or"))
        && tokens.get(1).is_some_and(|t| t.is_word("untap"))
    {
        let target_tokens = &tokens[2..];
        let target = parse_target_phrase(target_tokens)?;
        return Ok(EffectAst::TapOrUntap {
            target: target.clone(),
        });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Tap { target })
}

fn parse_sacrifice(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.contains(&"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported sacrifice-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_greatest_mana_value = clause_words.contains(&"greatest")
        && clause_words.contains(&"mana")
        && clause_words.contains(&"value");
    if has_greatest_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported greatest-mana-value sacrifice clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_for_each_graveyard_history = clause_words.contains(&"for")
        && clause_words.contains(&"each")
        && clause_words.contains(&"graveyard")
        && clause_words.contains(&"turn");
    if has_for_each_graveyard_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported graveyard-history sacrifice clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    if tokens
        .first()
        .is_some_and(|token| token.is_word("all") || token.is_word("each"))
    {
        let mut idx = 1usize;
        let mut other = false;
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_word("other") || token.is_word("another"))
        {
            other = true;
            idx += 1;
        }
        let mut filter = parse_object_filter(&tokens[idx..], other)?;
        if other {
            filter.other = true;
        }
        return Ok(EffectAst::SacrificeAll { filter, player });
    }

    let mut idx = 0;
    let mut count = 1u32;
    let mut other = false;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        other = true;
        idx += 1;
    }
    if count == 1
        && let Some((value, used)) = parse_number(&tokens[idx..])
    {
        count = value;
        idx += used;
    }

    let filter_tokens = trim_sacrifice_choice_suffix_tokens(&tokens[idx..]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing sacrifice object after chooser suffix (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let mut filter = if let Ok(target) = parse_target_phrase(filter_tokens) {
        target_ast_to_object_filter(target).unwrap_or(parse_object_filter(filter_tokens, other)?)
    } else {
        parse_object_filter(filter_tokens, other)?
    };
    if other {
        filter.other = true;
    }
    if filter.source && count != 1 {
        return Err(CardTextError::ParseError(format!(
            "source sacrifice only supports count 1 (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::Sacrifice {
        filter,
        player,
        count,
    })
}

fn trim_sacrifice_choice_suffix_tokens(tokens: &[Token]) -> &[Token] {
    let token_words = words(tokens);
    let suffix_word_count = if token_words.ends_with(&["of", "their", "choice"])
        || token_words.ends_with(&["of", "your", "choice"])
        || token_words.ends_with(&["of", "its", "choice"])
    {
        3usize
    } else if token_words.ends_with(&["of", "his", "or", "her", "choice"]) {
        5usize
    } else {
        0usize
    };

    if suffix_word_count == 0 {
        return tokens;
    }

    let keep_words = token_words.len().saturating_sub(suffix_word_count);
    let cut_idx = token_index_for_word_index(tokens, keep_words).unwrap_or(tokens.len());
    &tokens[..cut_idx]
}

fn parse_discard(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let clause_words = words(tokens);
    if clause_words.contains(&"hand") {
        return Ok(EffectAst::DiscardHand { player });
    }

    let (count, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing discard count (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    }

    let rest_words = words(rest);
    let trailing = if rest_words.len() > 1 {
        &rest_words[1..]
    } else {
        &[][..]
    };
    let random = trailing == ["at", "random"];
    if !trailing.is_empty() && !random {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing discard clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(EffectAst::Discard {
        count,
        player,
        random,
    })
}

fn parse_return(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.contains(&"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported return-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let to_idx = (0..tokens.len())
        .rev()
        .find(|idx| {
            if !tokens[*idx].is_word("to") {
                return false;
            }
            let tail_words = words(&tokens[*idx + 1..]);
            tail_words.contains(&"hand")
                || tail_words.contains(&"hands")
                || tail_words.contains(&"battlefield")
        })
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing return destination (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let target_tokens = &tokens[..to_idx];
    let destination_words_full = words(&tokens[to_idx + 1..]);
    let mut destination_words = destination_words_full.clone();
    let mut destination_excluded_subtypes: Vec<Subtype> = Vec::new();
    if let Some(except_idx) = destination_words_full
        .windows(2)
        .position(|window| window == ["except", "for"])
    {
        let exception_words = &destination_words_full[except_idx + 2..];
        if exception_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing return exception qualifiers (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        for word in exception_words {
            if matches!(*word, "and" | "or") {
                continue;
            }
            let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported return exception qualifier '{}' (clause: '{}')",
                    word,
                    words(tokens).join(" ")
                )));
            };
            if !destination_excluded_subtypes.contains(&subtype) {
                destination_excluded_subtypes.push(subtype);
            }
        }
        if destination_excluded_subtypes.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing subtype return exception qualifiers (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        destination_words.truncate(except_idx);
    }
    let is_hand = destination_words.contains(&"hand") || destination_words.contains(&"hands");
    let is_battlefield = destination_words.contains(&"battlefield");
    let tapped = destination_words.contains(&"tapped");
    let return_controller = if destination_words
        .windows(3)
        .any(|window| window == ["under", "your", "control"])
    {
        ReturnControllerAst::You
    } else if destination_words
        .iter()
        .any(|word| *word == "owner" || *word == "owners")
        && destination_words.contains(&"control")
    {
        ReturnControllerAst::Owner
    } else {
        ReturnControllerAst::Preserve
    };
    if destination_words.contains(&"transformed") {
        return Err(CardTextError::ParseError(format!(
            "unsupported transformed return clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let has_delayed_timing = destination_words.contains(&"beginning")
        || destination_words.contains(&"upkeep")
        || destination_words
            .windows(3)
            .any(|window| window == ["end", "of", "combat"])
        || destination_words.contains(&"end")
            && (destination_words.contains(&"next") || destination_words.contains(&"step"));
    if has_delayed_timing {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed return timing clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    if !is_hand && !is_battlefield {
        return Err(CardTextError::ParseError(format!(
            "unsupported return destination (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let target_words = words(target_tokens);
    if let Some(and_idx) = target_tokens.iter().position(|token| token.is_word("and"))
        && and_idx > 0
    {
        let tail_words = words(&target_tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (tail_words.starts_with(&["up", "to"]) && tail_words.contains(&"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target return clause (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    }
    if !target_words.contains(&"target")
        && target_words.contains(&"exiled")
        && target_words.contains(&"cards")
    {
        let filter = parse_object_filter(target_tokens, false)?;
        return if is_battlefield {
            Ok(EffectAst::ReturnAllToBattlefield { filter, tapped })
        } else {
            Ok(EffectAst::ReturnAllToHand { filter })
        };
    }
    if target_words
        .first()
        .is_some_and(|word| *word == "all" || *word == "each")
    {
        let has_unsupported_return_all_qualifier = target_words.contains(&"dealt")
            || target_words.contains(&"without") && target_words.contains(&"counter");
        if has_unsupported_return_all_qualifier {
            return Err(CardTextError::ParseError(format!(
                "unsupported qualified return-all filter (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing return-all filter".to_string(),
            ));
        }
        let mut filter = parse_object_filter(&target_tokens[1..], false)?;
        for subtype in destination_excluded_subtypes {
            if !filter.excluded_subtypes.contains(&subtype) {
                filter.excluded_subtypes.push(subtype);
            }
        }
        return if is_battlefield {
            Ok(EffectAst::ReturnAllToBattlefield { filter, tapped })
        } else {
            Ok(EffectAst::ReturnAllToHand { filter })
        };
    }
    if !destination_excluded_subtypes.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported return exception on non-return-all clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let target = parse_target_phrase(target_tokens)?;
    if is_battlefield {
        Ok(EffectAst::ReturnToBattlefield {
            target,
            tapped,
            controller: return_controller,
        })
    } else {
        Ok(EffectAst::ReturnToHand { target })
    }
}

fn parse_exchange(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.starts_with(&["control", "of"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    // Heterogeneous "exchange control of A and B" forms (e.g. "this artifact and target ...")
    // cannot be represented by the current single-filter ExchangeControl primitive.
    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) {
        let left_words = words(&tokens[..and_idx]);
        let right_words = words(&tokens[and_idx + 1..]);
        let left_mentions_this = left_words.contains(&"this");
        let right_mentions_this = right_words.contains(&"this");
        let left_mentions_target = left_words.contains(&"target");
        let right_mentions_target = right_words.contains(&"target");
        if left_mentions_this
            || right_mentions_this
            || left_mentions_target
            || right_mentions_target
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported heterogeneous exchange clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    let mut idx = 2usize;
    let mut count = 2u32;
    if let Some((value, used)) = parse_number(&tokens[idx..]) {
        count = value;
        idx += used;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        idx += 1;
    }
    if idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing exchange target filter".to_string(),
        ));
    }

    let filter = parse_object_filter(&tokens[idx..], false)?;
    Ok(EffectAst::ExchangeControl { filter, count })
}

fn parse_become(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported become clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    let amount = parse_value(tokens).map(|(value, _)| value).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life total amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    Ok(EffectAst::SetLifeTotal { amount, player })
}

fn parse_skip(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let Some(SubjectAst::Player(player)) = subject else {
        return Err(CardTextError::ParseError(format!(
            "unsupported skip clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    };

    let words = words(tokens);
    let skips_next_combat_phase_this_turn = words.contains(&"combat")
        && words.contains(&"phase")
        && words.contains(&"next")
        && words.contains(&"this")
        && words.contains(&"turn");
    if skips_next_combat_phase_this_turn {
        return Ok(EffectAst::SkipNextCombatPhaseThisTurn { player });
    }
    if words.contains(&"combat")
        && (words.contains(&"phase") || words.contains(&"phases"))
        && words.contains(&"turn")
    {
        return Ok(EffectAst::SkipCombatPhases { player });
    }
    if words.contains(&"draw") && words.contains(&"step") {
        return Ok(EffectAst::SkipDrawStep { player });
    }
    if words.contains(&"turn") {
        return Ok(EffectAst::SkipTurn { player });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported skip clause (clause: '{}')",
        words.join(" ")
    )))
}

fn parse_transform(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Ok(EffectAst::Transform {
            target: TargetAst::Source(None),
        });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Transform { target })
}

fn parse_regenerate(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        if tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "regenerate clause missing filter after each/all".to_string(),
            ));
        }
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::RegenerateAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Regenerate { target })
}

fn parse_mill(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let (count, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing mill count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "card" && word != "cards")
    {
        return Err(CardTextError::ParseError(
            "missing card keyword".to_string(),
        ));
    }
    let trailing_words: Vec<&str> = rest.iter().skip(1).filter_map(Token::as_word).collect();
    if !trailing_words.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mill clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Mill { count, player })
}

fn parse_get(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
    if words.contains(&"poison") && words.contains(&"counter") {
        let player = match subject {
            Some(SubjectAst::Player(player)) => player,
            _ => PlayerAst::Implicit,
        };
        return Ok(EffectAst::PoisonCounters {
            count: Value::Fixed(1),
            player,
        });
    }

    let energy_count = tokens.iter().filter(|token| token.is_word("e")).count();
    if energy_count > 0 {
        let player = match subject {
            Some(SubjectAst::Player(player)) => player,
            _ => PlayerAst::Implicit,
        };
        let count = parse_add_mana_equal_amount_value(tokens)
            .or(parse_dynamic_cost_modifier_value(tokens)?)
            .unwrap_or(Value::Fixed(energy_count as i32));
        return Ok(EffectAst::EnergyCounters {
            count,
            player,
        });
    }

    if let Some(mod_token) = tokens.first().and_then(Token::as_word)
        && let Ok((power, toughness)) = parse_pt_modifier_values(mod_token)
    {
        let (power, toughness, duration) =
            parse_get_modifier_values_with_tail(tokens, power, toughness)?;
        let target = match subject {
            Some(SubjectAst::This) => TargetAst::Source(None),
            _ => {
                return Err(CardTextError::ParseError(
                    "unsupported get clause (missing subject)".to_string(),
                ));
            }
        };
        return Ok(EffectAst::Pump {
            power,
            toughness,
            target,
            duration,
        });
    }

    Err(CardTextError::ParseError(format!(
        "unsupported get clause (clause: '{}')",
        words.join(" ")
    )))
}

fn parse_add_mana(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };
    parser_trace_stack("parse_add_mana:entry", tokens);
    let clause_words = words(tokens);

    let has_card_word = clause_words
        .iter()
        .any(|word| *word == "card" || *word == "cards");
    if clause_words.contains(&"exiled") && has_card_word && clause_words.contains(&"colors") {
        return Ok(EffectAst::AddManaImprintedColors);
    }

    if (clause_words.contains(&"commander") || clause_words.contains(&"commanders"))
        && clause_words.contains(&"color")
        && clause_words.contains(&"identity")
    {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaCommanderIdentity { amount, player });
    }

    if let Some(available_colors) = parse_any_combination_mana_colors(tokens)? {
        let amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        return Ok(EffectAst::AddManaAnyColor {
            amount,
            player,
            available_colors: Some(available_colors),
        });
    }

    if let Some(available_colors) = parse_or_mana_color_choices(tokens)? {
        return Ok(EffectAst::AddManaAnyColor {
            amount: Value::Fixed(1),
            player,
            available_colors: Some(available_colors),
        });
    }

    let any_one = clause_words
        .windows(3)
        .any(|window| window == ["any", "one", "color"] || window == ["any", "one", "type"]);
    let any_color = clause_words
        .windows(2)
        .any(|window| window == ["any", "color"] || window == ["one", "color"]);
    let any_type = clause_words
        .windows(2)
        .any(|window| window == ["any", "type"] || window == ["one", "type"]);
    if any_color || any_type {
        let mut amount = parse_value(tokens)
            .map(|(value, _)| value)
            .unwrap_or(Value::Fixed(1));
        let allow_colorless = any_type;
        let phrase_end = tokens
            .iter()
            .enumerate()
            .find_map(|(idx, token)| {
                let word = token.as_word()?;
                if (word == "color" && any_color) || (word == "type" && any_type) {
                    Some(idx + 1)
                } else {
                    None
                }
            })
            .unwrap_or(tokens.len());
        let tail_tokens = trim_leading_commas(&tokens[phrase_end..]);

        if tail_tokens.is_empty() || is_mana_pool_tail_tokens(tail_tokens) {
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        if let Some(filter) = parse_land_could_produce_filter(tail_tokens)? {
            parser_trace_stack("parse_add_mana:land-could-produce", tokens);
            return Ok(EffectAst::AddManaFromLandCouldProduce {
                amount,
                player,
                land_filter: filter,
                allow_colorless,
                same_type: any_one,
            });
        }

        if matches!(amount, Value::X)
            && let Some(dynamic_amount) = parse_where_x_is_number_of_filter_value(tail_tokens)
        {
            amount = dynamic_amount;
            if any_type {
                return Err(CardTextError::ParseError(format!(
                    "unsupported any-type mana clause without producer filter (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
            if any_one {
                return Ok(EffectAst::AddManaAnyOneColor { amount, player });
            }
            return Ok(EffectAst::AddManaAnyColor {
                amount,
                player,
                available_colors: None,
            });
        }

        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let for_each_idx = tokens
        .windows(2)
        .position(|window| window[0].is_word("for") && window[1].is_word("each"));
    let mana_scan_end = for_each_idx.unwrap_or(tokens.len());

    let mut mana = Vec::new();
    let mut last_mana_idx = None;
    for (idx, token) in tokens[..mana_scan_end].iter().enumerate() {
        if let Some(word) = token.as_word() {
            if word == "mana" || word == "to" || word == "your" || word == "pool" {
                continue;
            }
            if let Ok(symbol) = parse_mana_symbol(word) {
                mana.push(symbol);
                last_mana_idx = Some(idx);
            }
        }
    }

    if !mana.is_empty() {
        if let Some(amount) = parse_add_mana_that_much_value(tokens) {
            parser_trace_stack("parse_add_mana:scaled-that-much", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(amount) = parse_devotion_value_from_add_clause(tokens)? {
            parser_trace_stack("parse_add_mana:scaled-devotion", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(for_each_idx) = for_each_idx {
            let amount_tokens = &tokens[for_each_idx..];
            let amount = parse_dynamic_cost_modifier_value(amount_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "unsupported dynamic mana amount (clause: '{}')",
                    words(tokens).join(" ")
                ))
            })?;
            parser_trace_stack("parse_add_mana:scaled", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        if let Some(amount) = parse_add_mana_equal_amount_value(tokens) {
            parser_trace_stack("parse_add_mana:scaled-equal", tokens);
            return Ok(EffectAst::AddManaScaled {
                mana,
                amount,
                player,
            });
        }
        let trailing_words = if let Some(last_idx) = last_mana_idx {
            words(&tokens[last_idx + 1..])
        } else {
            Vec::new()
        };
        if !trailing_words.is_empty() {
            let chosen_color_tail =
                trailing_words.starts_with(&["or", "one", "mana", "of", "the", "chosen", "color"]);
            let pool_tail = if chosen_color_tail {
                trailing_words[7..].to_vec()
            } else {
                Vec::new()
            };
            let has_only_pool_tail = chosen_color_tail
                && (pool_tail.is_empty()
                    || pool_tail
                        .iter()
                        .all(|word| matches!(*word, "to" | "your" | "mana" | "pool")));
            if chosen_color_tail && has_only_pool_tail {
                if mana.len() != 1 {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with multiple symbols (clause: '{}')",
                        clause_words.join(" ")
                    )));
                }
                let Some(color) = mana_symbol_to_color(mana[0]) else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported chosen-color mana clause with non-colored symbol (clause: '{}')",
                        clause_words.join(" ")
                    )));
                };
                parser_trace_stack("parse_add_mana:chosen-color-option", tokens);
                return Ok(EffectAst::AddManaChosenColor {
                    amount: Value::Fixed(1),
                    player,
                    fixed_option: Some(color),
                });
            }
        }
        let has_only_pool_tail = !trailing_words.is_empty()
            && trailing_words
                .iter()
                .all(|word| matches!(*word, "to" | "your" | "mana" | "pool"));
        if !trailing_words.is_empty() && !has_only_pool_tail {
            return Err(CardTextError::ParseError(format!(
                "unsupported trailing mana clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        parser_trace_stack("parse_add_mana:flat", tokens);
        return Ok(EffectAst::AddMana { mana, player });
    }

    Err(CardTextError::ParseError(format!(
        "missing mana symbols (clause: '{}')",
        clause_words.join(" ")
    )))
}

fn mana_symbol_to_color(symbol: ManaSymbol) -> Option<crate::color::Color> {
    match symbol {
        ManaSymbol::White => Some(crate::color::Color::White),
        ManaSymbol::Blue => Some(crate::color::Color::Blue),
        ManaSymbol::Black => Some(crate::color::Color::Black),
        ManaSymbol::Red => Some(crate::color::Color::Red),
        ManaSymbol::Green => Some(crate::color::Color::Green),
        _ => None,
    }
}

fn parse_or_mana_color_choices(
    tokens: &[Token],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_words = words(tokens);
    if !clause_words.contains(&"or") {
        return Ok(None);
    }

    let mut colors = Vec::new();
    let mut has_or = false;
    for token in tokens {
        let Some(word) = token.as_word() else {
            continue;
        };
        if word == "or" {
            has_or = true;
            continue;
        }
        if matches!(word, "to" | "your" | "their" | "its" | "mana" | "pool") {
            continue;
        }
        if let Ok(symbol) = parse_mana_symbol(word) {
            let Some(color) = mana_symbol_to_color(symbol) else {
                return Ok(None);
            };
            if !colors.contains(&color) {
                colors.push(color);
            }
            continue;
        }
        return Ok(None);
    }

    if !has_or || colors.len() < 2 {
        return Ok(None);
    }

    Ok(Some(colors))
}

fn parse_any_combination_mana_colors(
    tokens: &[Token],
) -> Result<Option<Vec<crate::color::Color>>, CardTextError> {
    let clause_words = words(tokens);
    let Some(combination_idx) = clause_words
        .windows(3)
        .position(|window| window == ["any", "combination", "of"])
    else {
        return Ok(None);
    };

    let mut colors = Vec::new();
    for word in &clause_words[combination_idx + 3..] {
        if matches!(
            *word,
            "and" | "or" | "mana" | "to" | "your" | "their" | "its" | "pool"
        ) {
            continue;
        }
        let symbol = parse_mana_symbol(word).map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported restricted mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        let color = mana_symbol_to_color(symbol).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "unsupported non-colored mana symbol '{}' in any-combination clause (clause: '{}')",
                word,
                clause_words.join(" ")
            ))
        })?;
        if !colors.contains(&color) {
            colors.push(color);
        }
    }

    if colors.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing color options in any-combination mana clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(Some(colors))
}

fn trim_leading_commas(tokens: &[Token]) -> &[Token] {
    let start = tokens
        .iter()
        .position(|token| !matches!(token, Token::Comma(_)))
        .unwrap_or(tokens.len());
    &tokens[start..]
}

fn is_mana_pool_tail_tokens(tokens: &[Token]) -> bool {
    let words = words(tokens);
    if words.is_empty() || words[0] != "to" || !words.contains(&"mana") || !words.contains(&"pool")
    {
        return false;
    }
    words.iter().all(|word| {
        matches!(
            *word,
            "to" | "your" | "their" | "its" | "that" | "player" | "players" | "mana" | "pool"
        )
    })
}

fn parse_land_could_produce_filter(
    tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 || words[0] != "that" {
        return Ok(None);
    }
    let Some(could_idx) = words
        .windows(2)
        .position(|window| window == ["could", "produce"])
    else {
        return Ok(None);
    };
    if could_idx + 2 != words.len() {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing mana clause (tail: '{}')",
            words.join(" ")
        )));
    }

    let could_token_idx = tokens
        .iter()
        .position(|token| token.is_word("could"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing 'could' in mana tail '{}'",
                words.join(" ")
            ))
        })?;
    let filter_tokens = trim_leading_commas(&tokens[1..could_token_idx]);
    if filter_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing land filter in mana clause (tail: '{}')",
            words.join(" ")
        )));
    }
    let filter = parse_object_filter(filter_tokens, false)?;
    Ok(Some(filter))
}

fn looks_like_pt_word(word: &str) -> bool {
    let Some((power, toughness)) = word.split_once('/') else {
        return false;
    };
    let is_component = |part: &str| {
        let part = part.trim_matches(|ch| matches!(ch, '+' | '-'));
        part == "x" || part == "*" || part.parse::<i32>().is_ok()
    };
    is_component(power) && is_component(toughness)
}

fn parse_unsigned_pt_word(word: &str) -> Option<(i32, i32)> {
    let (power, toughness) = word.split_once('/')?;
    if power.starts_with('+')
        || toughness.starts_with('+')
        || power.starts_with('-')
        || toughness.starts_with('-')
    {
        return None;
    }
    let power = power.parse::<i32>().ok()?;
    let toughness = toughness.parse::<i32>().ok()?;
    Some((power, toughness))
}

fn is_probable_token_name_word(word: &str) -> bool {
    if !word.chars().all(|ch| ch.is_ascii_alphabetic() || ch == '\'' || ch == '-') {
        return false;
    }
    !matches!(
        word,
        "legendary"
            | "artifact"
            | "enchantment"
            | "creature"
            | "token"
            | "tokens"
            | "white"
            | "blue"
            | "black"
            | "red"
            | "green"
            | "colorless"
    )
}

fn parse_copy_modifiers_from_tail(
    tail_words: &[&str],
) -> (
    Vec<CardType>,
    Vec<Subtype>,
    Vec<Supertype>,
    Option<(i32, i32)>,
    Vec<StaticAbility>,
) {
    let mut added_card_types = Vec::new();
    let mut added_subtypes = Vec::new();
    let mut removed_supertypes = Vec::new();
    let mut set_base_power_toughness = None;
    let mut granted_abilities = Vec::new();

    let except_idx = tail_words.iter().rposition(|word| *word == "except");
    let modifier_words = except_idx
        .map(|idx| &tail_words[idx + 1..])
        .unwrap_or_default();
    if modifier_words.is_empty() {
        return (
            added_card_types,
            added_subtypes,
            removed_supertypes,
            set_base_power_toughness,
            granted_abilities,
        );
    }

    if modifier_words
        .windows(2)
        .any(|window| window == ["isnt", "legendary"] || window == ["isn't", "legendary"])
        || modifier_words
            .windows(3)
            .any(|window| window == ["is", "not", "legendary"])
    {
        removed_supertypes.push(Supertype::Legendary);
    }

    if let Some((power, toughness)) = modifier_words
        .iter()
        .find_map(|word| parse_unsigned_pt_word(word))
    {
        set_base_power_toughness = Some((power, toughness));
    }

    let has_grant_verb = modifier_words.contains(&"has")
        || modifier_words.contains(&"have")
        || modifier_words.contains(&"gain")
        || modifier_words.contains(&"gains");
    if has_grant_verb && modifier_words.contains(&"flying") {
        granted_abilities.push(StaticAbility::flying());
    }

    let addition_idx = modifier_words.windows(6).position(|window| {
        window == ["in", "addition", "to", "its", "other", "types"]
            || window == ["in", "addition", "to", "their", "other", "types"]
    });
    if let Some(addition_idx) = addition_idx {
        let descriptor_words = &modifier_words[..addition_idx];
        for word in descriptor_words {
            if let Some(card_type) = parse_card_type(word) && !added_card_types.contains(&card_type) {
                added_card_types.push(card_type);
            }
            if let Some(subtype) =
                parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !added_subtypes.contains(&subtype)
            {
                added_subtypes.push(subtype);
            }
        }
    }

    (
        added_card_types,
        added_subtypes,
        removed_supertypes,
        set_base_power_toughness,
        granted_abilities,
    )
}

fn parse_create(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };
    let clause_words = words(tokens);
    let has_unsupported_dynamic_count = clause_words.starts_with(&["a", "number", "of"])
        || clause_words.starts_with(&["the", "number", "of"]);
    if has_unsupported_dynamic_count {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic token count in create clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut idx = 0;
    let mut count_value = Value::Fixed(1);
    if tokens.first().is_some_and(|token| token.is_word("that"))
        && tokens.get(1).is_some_and(|token| token.is_word("many"))
    {
        count_value = Value::EventValue(EventValueSpec::Amount);
        idx = 2;
    } else if tokens.first().is_some_and(|token| token.is_word("x")) {
        count_value = Value::X;
        idx = 1;
    } else if let Some((parsed_count, used)) = parse_number(tokens) {
        count_value = Value::Fixed(parsed_count as i32);
        idx = used;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("a") || token.is_word("an"))
    {
        idx += 1;
    }

    let remaining_words = words(&tokens[idx..]);
    let token_idx = remaining_words
        .iter()
        .position(|word| *word == "token" || *word == "tokens")
        .ok_or_else(|| CardTextError::ParseError("create clause missing token".to_string()))?;

    let mut name_words: Vec<&str> = remaining_words[..token_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    let tail_tokens = &tokens[idx + token_idx + 1..];
    let tail_words = remaining_words[token_idx + 1..].to_vec();
    let for_each_idx = tail_words
        .windows(2)
        .position(|window| window == ["for", "each"]);
    let mut for_each_count_value: Option<Value> = None;
    let mut for_each_wrap_filter: Option<ObjectFilter> = None;
    if let Some(for_each_idx) = for_each_idx {
        let filter_tokens = &tail_tokens[for_each_idx + 2..];
        if filter_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing filter after 'for each' in create clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if let Some(dynamic) = parse_create_for_each_dynamic_count(filter_tokens) {
            for_each_count_value = Some(dynamic);
        } else {
            let filter = parse_object_filter(filter_tokens, false)?;
            for_each_count_value = Some(Value::Count(filter.clone()));
            for_each_wrap_filter = Some(filter);
        }
    }
    let wrap_for_each = |effect: EffectAst| {
        if let Some(filter) = for_each_wrap_filter.clone() {
            EffectAst::ForEachObject {
                filter,
                effects: vec![effect],
            }
        } else {
            effect
        }
    };
    let mut tapped = false;
    let mut attacking = false;
    if let Some(named_idx) = tail_words.iter().position(|word| *word == "named") {
        let range_end = for_each_idx.unwrap_or(tail_words.len());
        if named_idx + 1 < range_end {
            let after_named = &tail_words[named_idx + 1..range_end];
            let name_end = after_named
                .iter()
                .position(|word| matches!(*word, "with" | "that" | "which"))
                .map(|offset| named_idx + 1 + offset)
                .unwrap_or(range_end);
            if named_idx + 1 < name_end {
                name_words.extend(tail_words[named_idx + 1..name_end].iter().copied());
            }
        }
    }
    name_words.retain(|word| {
        if *word == "tapped" {
            tapped = true;
            return false;
        }
        if *word == "attacking" {
            attacking = true;
            return false;
        }
        true
    });
    let name_words_primary_len = name_words.len();
    if name_words.is_empty() {
        if tail_words
            .iter()
            .any(|word| *word == "copy" || *word == "copies")
        {
            let (
                added_card_types,
                added_subtypes,
                removed_supertypes,
                set_base_power_toughness,
                granted_abilities,
            ) = parse_copy_modifiers_from_tail(&tail_words);
            let half_pt = tail_words.contains(&"half")
                && tail_words.contains(&"power")
                && tail_words.contains(&"toughness");
            let has_haste = tail_words
                .windows(2)
                .any(|window| matches!(window, ["has", "haste"] | ["gain", "haste"] | ["gains", "haste"]))
                || tail_words.contains(&"haste");
            let has_beginning_of_end_step = tail_words.windows(6).any(|window| {
                window == ["beginning", "of", "the", "next", "end", "step"]
            }) || tail_words
                .windows(5)
                .any(|window| window == ["beginning", "of", "next", "end", "step"])
                || tail_words
                    .windows(5)
                    .any(|window| window == ["beginning", "of", "the", "end", "step"])
                || tail_words
                    .windows(4)
                    .any(|window| window == ["beginning", "of", "end", "step"]);
            let has_sacrifice_reference = tail_words.contains(&"sacrifice")
                && (tail_words.contains(&"token")
                    || tail_words.contains(&"permanent")
                    || tail_words.contains(&"it")
                    || tail_words.contains(&"them"));
            let has_exile_reference = tail_words.contains(&"exile")
                && (tail_words.contains(&"token")
                    || tail_words.contains(&"permanent")
                    || tail_words.contains(&"it")
                    || tail_words.contains(&"them"));
            let sacrifice_at_next_end_step = has_beginning_of_end_step && has_sacrifice_reference;
            let exile_at_next_end_step = has_beginning_of_end_step && has_exile_reference;
            if let Some(of_idx) = tail_tokens.iter().position(|token| token.is_word("of")) {
                let source_tokens = &tail_tokens[of_idx + 1..];
                let source_end = source_tokens
                    .iter()
                    .position(|token| matches!(token, Token::Comma(_)) || token.is_word("except"))
                    .unwrap_or(source_tokens.len());
                let source_tokens = &source_tokens[..source_end];
                if !source_tokens.is_empty() {
                    let source = parse_target_phrase(source_tokens)?;
                    return Ok(wrap_for_each(EffectAst::CreateTokenCopyFromSource {
                        source,
                        count: count_value.clone(),
                        player,
                        half_power_toughness_round_up: half_pt,
                        has_haste,
                        sacrifice_at_next_end_step,
                        exile_at_next_end_step,
                        added_card_types,
                        added_subtypes,
                        removed_supertypes,
                        set_base_power_toughness,
                        granted_abilities,
                    }));
                }
            }
            return Ok(wrap_for_each(EffectAst::CreateTokenCopy {
                object: ObjectRefAst::It,
                count: count_value.clone(),
                player,
                half_power_toughness_round_up: half_pt,
                has_haste,
                sacrifice_at_next_end_step,
                exile_at_next_end_step,
                added_card_types,
                added_subtypes,
                removed_supertypes,
                set_base_power_toughness,
                granted_abilities,
            }));
        }
        return Err(CardTextError::ParseError(
            "create clause missing token name".to_string(),
        ));
    }
    if let Some(with_idx) = tail_words.iter().position(|word| *word == "with") {
        let with_tail_end = for_each_idx.unwrap_or(tail_words.len());
        if with_idx + 1 < with_tail_end {
            let with_words = &tail_words[with_idx + 1..with_tail_end];
            let rules_text_start = with_words.iter().position(|word| {
                matches!(
                    *word,
                    "when"
                        | "whenever"
                        | "if"
                        | "t"
                        | "this"
                        | "that"
                        | "it"
                        | "those"
                        | "sacrifice"
                        | "add"
                        | "draw"
                        | "deals"
                        | "deal"
                )
            });
            let include_end = rules_text_start.unwrap_or(with_words.len());
            let preserve_rules_tail = rules_text_start
                .is_some_and(|start| start < with_words.len())
                && with_words[include_end..].iter().any(|word| {
                    matches!(
                        *word,
                        "when"
                            | "whenever"
                            | "at"
                            | "sacrifice"
                            | "return"
                            | "counter"
                            | "draw"
                            | "add"
                            | "deals"
                            | "deal"
                            | "gets"
                            | "gain"
                            | "gains"
                            | "cant"
                            | "can"
                            | "block"
                    )
                });
            if include_end > 0 {
                name_words.extend(with_words[..include_end].iter().copied());
                if preserve_rules_tail {
                    // Keep quoted token rules text tails so token lowering can
                    // reconstruct granted abilities instead of dropping them.
                    name_words.extend(with_words[include_end..].iter().copied());
                }
            } else {
                // Preserve quoted token rules text so token compilation can
                // attach the ability to the created token definition.
                name_words.extend(with_words.iter().copied());
            }
        }
    }
    if let Some(pt_idx) = name_words.iter().position(|word| looks_like_pt_word(word))
        && pt_idx > 0
        && pt_idx < name_words_primary_len
    {
        let prefix_words = &name_words[..pt_idx];
        let keep_prefix = prefix_words.contains(&"legendary")
            || prefix_words
                .first()
                .is_some_and(|word| is_probable_token_name_word(word));
        if !keep_prefix {
            name_words = name_words[pt_idx..].to_vec();
        }
    }
    let name = normalize_token_name(&name_words);

    tapped |= tail_words.contains(&"tapped");
    attacking |= tail_words.contains(&"attacking");
    if let Some(count_override) = for_each_count_value {
        count_value = count_override;
    }

    let create = EffectAst::CreateTokenWithMods {
        name,
        count: count_value,
        player,
        tapped,
        attacking,
        exile_at_end_of_combat: false,
    };
    Ok(create)
}

fn parse_create_for_each_dynamic_count(tokens: &[Token]) -> Option<Value> {
    let clause_words = words(tokens);
    if clause_words.starts_with(&["color", "of", "mana", "spent", "to", "cast", "this", "spell"])
        || clause_words
            .starts_with(&["colors", "of", "mana", "spent", "to", "cast", "this", "spell"])
        || clause_words.starts_with(&["color", "of", "mana", "used", "to", "cast", "this", "spell"])
        || clause_words
            .starts_with(&["colors", "of", "mana", "used", "to", "cast", "this", "spell"])
    {
        return Some(Value::ColorsOfManaSpentToCastThisSpell);
    }
    None
}

fn normalize_token_name(words: &[&str]) -> String {
    words.join(" ")
}

fn parse_investigate(_tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    Ok(EffectAst::Investigate)
}

fn parse_remove(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let mut idx = 0;
    let mut up_to = false;
    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        up_to = true;
        idx += 2;
    }

    let (amount, used) = parse_value(&tokens[idx..]).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing counter removal amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;
    idx += used;

    let counter_idx = tokens[idx..]
        .iter()
        .position(|token| token.is_word("counter") || token.is_word("counters"))
        .map(|offset| idx + offset)
        .ok_or_else(|| CardTextError::ParseError("missing counter keyword".to_string()))?;
    if counter_idx >= tokens.len() {
        return Err(CardTextError::ParseError(
            "missing counter keyword".to_string(),
        ));
    }
    idx = counter_idx + 1;

    if tokens.get(idx).is_some_and(|token| token.is_word("from")) {
        idx += 1;
    }

    let target_tokens = &tokens[idx..];
    let target = parse_target_phrase(target_tokens)?;

    let _ = up_to;
    Ok(EffectAst::RemoveUpToAnyCounters { amount, target })
}

fn parse_destroy(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if clause_words
        .windows(3)
        .any(|window| window == ["end", "of", "combat"])
        || (clause_words.contains(&"beginning") && clause_words.contains(&"end"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported delayed destroy timing clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_combat_history = (clause_words.contains(&"dealt")
        && clause_words.contains(&"damage")
        && clause_words.contains(&"turn"))
        || clause_words
            .windows(2)
            .any(|window| matches!(window, ["was", "blocked"] | ["was", "blocking"]))
        || clause_words.windows(2).any(|window| {
            matches!(
                window,
                ["blocking", "it"] | ["blocked", "it"] | ["it", "blocked"]
            )
        });
    if has_combat_history {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history destroy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        if let Some(attached_idx) = tokens.iter().position(|token| token.is_word("attached"))
            && tokens.get(attached_idx + 1).is_some_and(|token| token.is_word("to"))
            && attached_idx > 1
        {
            let mut filter_tokens = trim_commas(&tokens[1..attached_idx]).to_vec();
            while filter_tokens
                .last()
                .and_then(Token::as_word)
                .is_some_and(|word| matches!(word, "that" | "were" | "was" | "is" | "are"))
            {
                filter_tokens.pop();
            }
            let target_tokens = trim_commas(&tokens[attached_idx + 2..]);
            let target_words = words(&target_tokens);
            let has_timing_tail = target_words.iter().any(|word| {
                matches!(
                    *word,
                    "at" | "beginning" | "end" | "combat" | "turn" | "step" | "until"
                )
            });
            let supported_target = target_words.starts_with(&["target"])
                || target_words == ["it"]
                || target_words.starts_with(&["that", "creature"])
                || target_words.starts_with(&["that", "permanent"])
                || target_words.starts_with(&["that", "land"])
                || target_words.starts_with(&["that", "artifact"])
                || target_words.starts_with(&["that", "enchantment"]);
            if !filter_tokens.is_empty()
                && !target_tokens.is_empty()
                && supported_target
                && !has_timing_tail
            {
                let filter = parse_object_filter(&filter_tokens, false)?;
                let target = parse_target_phrase(&target_tokens)?;
                return Ok(EffectAst::DestroyAllAttachedTo { filter, target });
            }
        }
        let filter_tokens = &tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::DestroyAll { filter });
    }

    if clause_words.contains(&"unless") {
        return Err(CardTextError::ParseError(format!(
            "unsupported destroy-unless clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if clause_words.contains(&"if") {
        return Err(CardTextError::ParseError(format!(
            "unsupported conditional destroy clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and")) {
        let tail_words = words(&tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (tail_words.starts_with(&["up", "to"]) && tail_words.contains(&"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target destroy clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if clause_words.starts_with(&["target", "blocked"]) {
        let mut target_tokens = tokens.to_vec();
        if let Some(blocked_idx) = target_tokens.iter().position(|token| token.is_word("blocked")) {
            target_tokens.remove(blocked_idx);
        }
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate: PredicateAst::TargetIsBlocked,
            if_true: vec![EffectAst::Destroy { target }],
            if_false: Vec::new(),
        });
    }

    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Destroy { target })
}

fn parse_exile(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let (tokens, until_source_leaves) = split_until_source_leaves_tail(tokens);
    let clause_words = words(tokens);
    let has_face_down_manifest_tail = (clause_words.contains(&"face-down")
        || clause_words.contains(&"facedown")
        || clause_words.contains(&"manifest")
        || clause_words.contains(&"pile"))
        && clause_words.contains(&"then");
    if has_face_down_manifest_tail {
        return Err(CardTextError::ParseError(format!(
            "unsupported face-down/manifest exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        let filter_tokens = &tokens[1..];
        let mut filter = parse_object_filter(filter_tokens, false)?;
        apply_exile_subject_owner_context(&mut filter, subject);
        return Ok(if until_source_leaves {
            EffectAst::ExileUntilSourceLeaves {
                target: TargetAst::Object(filter, None, None),
            }
        } else {
            EffectAst::ExileAll { filter }
        });
    }
    if let Some(filter) = parse_target_player_graveyard_filter(tokens) {
        return Ok(if until_source_leaves {
            EffectAst::ExileUntilSourceLeaves {
                target: TargetAst::Object(filter, None, None),
            }
        } else {
            EffectAst::ExileAll { filter }
        });
    }

    if clause_words.contains(&"dealt")
        && clause_words.contains(&"damage")
        && clause_words.contains(&"turn")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history exile clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_until_total_mana_value = clause_words.contains(&"until")
        && clause_words.contains(&"exiled")
        && clause_words.contains(&"total")
        && clause_words.contains(&"mana")
        && clause_words.contains(&"value");
    if has_until_total_mana_value {
        return Err(CardTextError::ParseError(format!(
            "unsupported iterative exile-total clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_attached_bundle = clause_words.contains(&"and")
        && clause_words.contains(&"all")
        && clause_words.contains(&"attached");
    if has_attached_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported attached-object exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let has_same_name_token_bundle = clause_words.contains(&"and")
        && clause_words.contains(&"tokens")
        && clause_words.contains(&"same")
        && clause_words.contains(&"name");
    if has_same_name_token_bundle {
        return Err(CardTextError::ParseError(format!(
            "unsupported same-name token exile bundle (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    if let Some(and_idx) = tokens.iter().position(|token| token.is_word("and"))
        && and_idx > 0
    {
        let tail_words = words(&tokens[and_idx + 1..]);
        let starts_multi_target = tail_words.first() == Some(&"target")
            || (tail_words.starts_with(&["up", "to"]) && tail_words.contains(&"target"));
        if starts_multi_target {
            return Err(CardTextError::ParseError(format!(
                "unsupported multi-target exile clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
    }

    if let Some(if_idx) = tokens.iter().position(|token| token.is_word("if"))
        && if_idx > 0
    {
        let target_tokens = trim_commas(&tokens[..if_idx]);
        let predicate_tokens = trim_commas(&tokens[if_idx + 1..]);
        if target_tokens.is_empty() || predicate_tokens.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "unsupported conditional exile clause (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        let mut target = parse_target_phrase(&target_tokens)?;
        apply_exile_subject_hand_owner_context(&mut target, subject);
        let predicate = parse_predicate(&predicate_tokens)?;
        return Ok(EffectAst::Conditional {
            predicate,
            if_true: vec![if until_source_leaves {
                EffectAst::ExileUntilSourceLeaves { target }
            } else {
                EffectAst::Exile { target }
            }],
            if_false: Vec::new(),
        });
    }

    let mut target = parse_target_phrase(tokens)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    Ok(if until_source_leaves {
        EffectAst::ExileUntilSourceLeaves { target }
    } else {
        EffectAst::Exile { target }
    })
}

fn split_until_source_leaves_tail(tokens: &[Token]) -> (&[Token], bool) {
    let Some(until_idx) = tokens.iter().rposition(|token| token.is_word("until")) else {
        return (tokens, false);
    };
    if until_idx == 0 {
        return (tokens, false);
    }
    let tail_words = words(&tokens[until_idx + 1..]);
    let has_source_leaves_tail = tail_words.len() >= 3
        && tail_words[tail_words.len() - 3] == "leaves"
        && tail_words[tail_words.len() - 2] == "the"
        && tail_words[tail_words.len() - 1] == "battlefield";
    if has_source_leaves_tail {
        (&tokens[..until_idx], true)
    } else {
        (tokens, false)
    }
}

fn parse_target_player_graveyard_filter(tokens: &[Token]) -> Option<ObjectFilter> {
    let words = words(tokens);
    if words.as_slice() == ["target", "player", "graveyard"]
        || words.as_slice() == ["target", "players", "graveyard"]
        || words.as_slice() == ["that", "player", "graveyard"]
        || words.as_slice() == ["that", "players", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::target_player());
        return Some(filter);
    }
    if words.as_slice() == ["target", "opponent", "graveyard"]
        || words.as_slice() == ["target", "opponents", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)));
        return Some(filter);
    }
    if words.as_slice() == ["its", "controller", "graveyard"]
        || words.as_slice() == ["its", "controllers", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(
            "triggering",
        )));
        return Some(filter);
    }
    if words.as_slice() == ["its", "owner", "graveyard"]
        || words.as_slice() == ["its", "owners", "graveyard"]
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
            "triggering",
        )));
        return Some(filter);
    }
    None
}

fn apply_exile_subject_hand_owner_context(target: &mut TargetAst, subject: Option<SubjectAst>) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Hand) {
        return;
    }
    apply_exile_subject_owner_context(filter, subject);
}

fn apply_exile_subject_owner_context(filter: &mut ObjectFilter, subject: Option<SubjectAst>) {
    let Some(owner_filter) = exile_subject_owner_filter(subject) else {
        return;
    };
    if !matches!(
        filter.zone,
        Some(Zone::Hand) | Some(Zone::Graveyard) | Some(Zone::Library) | Some(Zone::Exile)
    ) {
        return;
    }
    match filter.owner {
        Some(PlayerFilter::Target(_)) | Some(PlayerFilter::IteratedPlayer) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}

fn apply_shuffle_subject_graveyard_owner_context(target: &mut TargetAst, subject: SubjectAst) {
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Graveyard) {
        return;
    }

    let owner_filter = match subject {
        SubjectAst::Player(PlayerAst::Target) => Some(PlayerFilter::target_player()),
        SubjectAst::Player(PlayerAst::TargetOpponent) => Some(PlayerFilter::target_opponent()),
        SubjectAst::Player(PlayerAst::You) => Some(PlayerFilter::You),
        _ => None,
    };
    let Some(owner_filter) = owner_filter else {
        return;
    };

    match filter.owner {
        Some(PlayerFilter::IteratedPlayer) | Some(PlayerFilter::Target(_)) | None => {
            filter.owner = Some(owner_filter);
        }
        _ => {}
    }
}

fn exile_subject_owner_filter(subject: Option<SubjectAst>) -> Option<PlayerFilter> {
    match subject {
        Some(SubjectAst::Player(PlayerAst::Target)) => Some(PlayerFilter::target_player()),
        Some(SubjectAst::Player(PlayerAst::TargetOpponent)) => {
            Some(PlayerFilter::Target(Box::new(PlayerFilter::Opponent)))
        }
        _ => None,
    }
}

fn target_object_filter_mut(target: &mut TargetAst) -> Option<&mut ObjectFilter> {
    match target {
        TargetAst::Object(filter, _, _) => Some(filter),
        TargetAst::WithCount(inner, _) => target_object_filter_mut(inner),
        _ => None,
    }
}

fn parse_untap(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "untap clause missing target".to_string(),
        ));
    }
    let words = words(tokens);
    if matches!(words.first().copied(), Some("all" | "each")) {
        let filter = parse_object_filter(&tokens[1..], false)?;
        return Ok(EffectAst::UntapAll { filter });
    }
    if words.as_slice() == ["them"] {
        let mut filter = ObjectFilter::default();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(EffectAst::UntapAll { filter });
    }
    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Untap { target })
}

fn parse_scry(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing scry count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Scry { count, player })
}

fn parse_surveil(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let (count, _) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing surveil count (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Surveil { count, player })
}

fn parse_pay(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    if let Some((amount, used)) = parse_value(tokens)
        && tokens.get(used).is_some_and(|token| token.is_word("life"))
    {
        return Ok(EffectAst::LoseLife { amount, player });
    }
    if let Some((amount, used)) = parse_value(tokens)
        && tokens.get(used).is_some_and(|token| token.is_word("energy"))
    {
        return Ok(EffectAst::PayEnergy { amount, player });
    }
    if tokens.iter().any(|token| token.is_word("e")) {
        let mut energy_count = 0u32;
        for token in tokens {
            let Some(word) = token.as_word() else {
                continue;
            };
            if is_article(word)
                || word == "and"
                || word == "or"
                || word == "energy"
                || word == "counter"
                || word == "counters"
            {
                continue;
            }
            if word == "e" {
                energy_count += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported pay clause token '{word}' (clause: '{}')",
                words(tokens).join(" ")
            )));
        }
        if energy_count > 0 {
            return Ok(EffectAst::PayEnergy {
                amount: Value::Fixed(energy_count as i32),
                player,
            });
        }
    }

    let mut pips = Vec::new();
    for token in tokens {
        let Some(word) = token.as_word() else {
            continue;
        };
        if is_article(word) || word == "mana" {
            continue;
        }
        if let Ok(symbols) = parse_mana_symbol_group(&word) {
            pips.push(symbols);
            continue;
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported pay clause token '{word}' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if pips.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing payment cost (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    Ok(EffectAst::PayMana {
        cost: ManaCost::from_pips(pips),
        player,
    })
}

fn parse_filter_comparison_tokens(
    axis: &str,
    tokens: &[&str],
    clause_words: &[&str],
) -> Result<Option<(crate::filter::Comparison, usize)>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let parse_operand = |operand: &str, extra_words: &[&str]| -> Result<i32, CardTextError> {
        let value = operand.parse::<i32>().map_err(|_| {
            CardTextError::ParseError(format!(
                "unsupported dynamic {axis} comparison operand '{operand}' (clause: '{}')",
                clause_words.join(" ")
            ))
        })?;
        if extra_words
            .first()
            .is_some_and(|word| matches!(*word, "plus" | "minus"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported arithmetic {axis} comparison (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        Ok(value)
    };

    let first = tokens[0];
    if let Ok(value) = first.parse::<i32>() {
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| word.parse::<i32>().is_ok())
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported disjunctive {axis} comparison (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if tokens
            .get(1)
            .is_some_and(|word| matches!(*word, "plus" | "minus"))
        {
            return Err(CardTextError::ParseError(format!(
                "unsupported arithmetic {axis} comparison (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "greater" | "more"))
        {
            return Ok(Some((
                crate::filter::Comparison::GreaterThanOrEqual(value),
                3,
            )));
        }
        if tokens.get(1) == Some(&"or")
            && tokens
                .get(2)
                .is_some_and(|word| matches!(*word, "less" | "fewer"))
        {
            return Ok(Some((crate::filter::Comparison::LessThanOrEqual(value), 3)));
        }
        return Ok(Some((crate::filter::Comparison::Equal(value), 1)));
    }

    if first == "equal" && tokens.get(1) == Some(&"to") {
        let Some(operand) = tokens.get(2).copied() else {
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after 'equal to' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let value = parse_operand(operand, &tokens[3..])?;
        return Ok(Some((crate::filter::Comparison::Equal(value), 3)));
    }

    if matches!(first, "less" | "greater") && tokens.get(1) == Some(&"than") {
        let mut operand_idx = 2usize;
        let mut inclusive = false;
        if tokens.get(operand_idx) == Some(&"or")
            && tokens.get(operand_idx + 1) == Some(&"equal")
            && tokens.get(operand_idx + 2) == Some(&"to")
        {
            inclusive = true;
            operand_idx += 3;
        }
        let Some(operand) = tokens.get(operand_idx).copied() else {
            return Err(CardTextError::ParseError(format!(
                "missing {axis} comparison operand after '{first} than' (clause: '{}')",
                clause_words.join(" ")
            )));
        };
        let value = parse_operand(operand, &tokens[operand_idx + 1..])?;
        let cmp = match (first, inclusive) {
            ("less", true) => crate::filter::Comparison::LessThanOrEqual(value),
            ("less", false) => crate::filter::Comparison::LessThan(value),
            ("greater", true) => crate::filter::Comparison::GreaterThanOrEqual(value),
            ("greater", false) => crate::filter::Comparison::GreaterThan(value),
            _ => unreachable!("first is constrained above"),
        };
        return Ok(Some((cmp, operand_idx + 1)));
    }

    if first == "x" {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic {axis} comparison operand 'x' (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    Ok(None)
}

fn parse_target_phrase(tokens: &[Token]) -> Result<TargetAst, CardTextError> {
    let mut tokens = tokens;
    while tokens
        .first()
        .is_some_and(|token| token.is_word("then"))
    {
        tokens = &tokens[1..];
    }
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target phrase".to_string(),
        ));
    }

    let mut idx = 0;
    let mut other = false;
    let span = span_from_tokens(tokens);
    let mut target_count: Option<ChoiceCount> = None;
    let mut explicit_target = false;

    let all_words = words(tokens);
    if all_words.as_slice() == ["that", "permanent"] || all_words.as_slice() == ["that", "creature"]
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }

    let remaining_words: Vec<&str> = all_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if remaining_words.as_slice() == ["equipped", "creature"]
        || remaining_words.as_slice() == ["equipped", "creatures"]
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["enchanted", "creature"]
        || remaining_words.as_slice() == ["enchanted", "creatures"]
    {
        let filter = parse_object_filter(tokens, false)?;
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("any"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("number"))
        && tokens.get(idx + 2).is_some_and(|token| token.is_word("of"))
    {
        target_count = Some(ChoiceCount::any_number());
        idx += 3;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("up"))
        && tokens.get(idx + 1).is_some_and(|token| token.is_word("to"))
    {
        idx += 2;
        if let Some((count, used)) = parse_number(&tokens[idx..]) {
            target_count = Some(ChoiceCount::up_to(count as usize));
            idx += used;
        } else {
            let next_word = tokens.get(idx).and_then(Token::as_word).unwrap_or("?");
            return Err(CardTextError::ParseError(format!(
                "unsupported dynamic or missing target count after 'up to' (found '{next_word}' in clause: '{}')",
                words(tokens).join(" ")
            )));
        }
    } else if let Some((count, used)) = parse_target_count_range_prefix(&tokens[idx..]) {
        target_count = Some(count);
        idx += used;
    } else if let Some((count, used)) = parse_number(&tokens[idx..]) {
        let next_is_target = tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("target"));
        let next_is_other_target = tokens
            .get(idx + used)
            .is_some_and(|token| token.is_word("other"))
            && tokens
                .get(idx + used + 1)
                .is_some_and(|token| token.is_word("target"));
        let next_is_object_selector = tokens
            .get(idx + used)
            .and_then(Token::as_word)
            .is_some_and(|word| {
                parse_card_type(word).is_some()
                    || parse_non_type(word).is_some()
                    || parse_subtype_word(word).is_some()
                    || word
                        .strip_suffix('s')
                        .and_then(parse_subtype_word)
                        .is_some()
            });
        if next_is_target || next_is_other_target {
            target_count = Some(ChoiceCount::exactly(count as usize));
            idx += used;
        } else if next_is_object_selector {
            target_count = Some(ChoiceCount::exactly(count as usize));
            idx += used;
        }
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        // Preserve explicit X-target wording while reusing open-ended target
        // selection internals.
        target_count = Some(ChoiceCount::dynamic_x());
        idx += 1;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    while tokens
        .get(idx)
        .and_then(Token::as_word)
        .is_some_and(is_article)
    {
        idx += 1;
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("other"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        other = true;
        explicit_target = true;
        idx += 2;
    } else {
        if tokens
            .get(idx)
            .is_some_and(|token| token.is_word("another") || token.is_word("other"))
        {
            other = true;
            idx += 1;
        }

        if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
            explicit_target = true;
            idx += 1;
        }
    }

    if let Some(ordinal_word) = tokens.get(idx).and_then(Token::as_word)
        && matches!(
            ordinal_word,
            "first"
                | "second"
                | "third"
                | "fourth"
                | "fifth"
                | "sixth"
                | "seventh"
                | "eighth"
                | "ninth"
                | "tenth"
        )
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        if ordinal_word != "first" {
            other = true;
        }
        explicit_target = true;
        idx += 2;
    }

    let words_all = words(&tokens[idx..]);
    if words_all.as_slice() == ["any", "target"] {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = words(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let target_span = if explicit_target { span } else { None };

    if remaining_words.as_slice() == ["player", "on", "your", "team"]
        || remaining_words.as_slice() == ["players", "on", "your", "team"]
    {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["player"] || remaining_words.as_slice() == ["players"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Any, target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["that", "player"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["that", "opponent"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_opponent(), target_span),
            target_count,
        ));
    }
    let second_word_is_object_head = remaining_words
        .get(1)
        .is_some_and(|word| {
            matches!(
                *word,
                "creature"
                    | "creatures"
                    | "permanent"
                    | "permanents"
                    | "spell"
                    | "spells"
                    | "source"
                    | "sources"
                    | "card"
                    | "cards"
            ) || parse_card_type(word).is_some()
                || word
                    .strip_suffix('s')
                    .is_some_and(|singular| parse_card_type(singular).is_some())
        });
    if remaining_words.len() >= 3
        && remaining_words[0] == "that"
        && second_word_is_object_head
        && matches!(remaining_words[2], "controller" | "controllers" | "owner" | "owners")
    {
        let player = if remaining_words[2].starts_with("owner") {
            PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        } else {
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG))
        };
        return Ok(wrap_target_count(
            TargetAst::Player(player, target_span),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["its", "controller"])
        || remaining_words.starts_with(&["its", "controllers"])
        || remaining_words.starts_with(&["their", "controller"])
        || remaining_words.starts_with(&["their", "controllers"])
    {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }
    if remaining_words.starts_with(&["its", "owner"])
        || remaining_words.starts_with(&["its", "owners"])
        || remaining_words.starts_with(&["their", "owner"])
        || remaining_words.starts_with(&["their", "owners"])
    {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(IT_TAG)),
                target_span,
            ),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["you"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["opponent"] || remaining_words.as_slice() == ["opponents"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["spell"] || remaining_words.as_slice() == ["spells"] {
        return Ok(wrap_target_count(TargetAst::Spell(target_span), target_count));
    }

    if is_source_reference_words(&remaining_words) {
        return Ok(wrap_target_count(TargetAst::Source(target_span), target_count));
    }
    if is_source_from_your_graveyard_words(&remaining_words) {
        return Ok(wrap_target_count(TargetAst::Source(target_span), target_count));
    }
    if remaining_words.starts_with(&["thiss", "power", "and", "toughness"]) {
        return Ok(wrap_target_count(TargetAst::Source(target_span), target_count));
    }

    if remaining_words.first().is_some_and(|word| *word == "it")
        && remaining_words
            .iter()
            .skip(1)
            .all(|word| *word == "instead" || *word == "this" || *word == "way")
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["itself"] {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if matches!(
        remaining_words.as_slice(),
        ["them"] | ["him"] | ["her"] | ["that", "player"]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }

    let opponent_or_planeswalker = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["opponent", "or", "planeswalker"]
                | ["opponents", "or", "planeswalkers"]
                | ["planeswalker", "or", "opponent"]
                | ["planeswalkers", "or", "opponents"]
        )
    });
    if opponent_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let player_or_planeswalker = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["player", "or", "planeswalker"]
                | ["players", "or", "planeswalkers"]
                | ["planeswalker", "or", "player"]
                | ["planeswalkers", "or", "players"]
        )
    });
    if player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if matches!(
        remaining_words.as_slice(),
        ["permanent", "or", "player"]
            | ["permanents", "or", "players"]
            | ["player", "or", "permanent"]
            | ["players", "or", "permanents"]
    ) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from(IT_TAG), span),
            target_count,
        ));
    }

    let creature_or_player = remaining_words.windows(3).any(|window| {
        matches!(
            window,
            ["creature", "or", "player"]
                | ["creatures", "or", "players"]
                | ["player", "or", "creature"]
                | ["players", "or", "creatures"]
                | ["creature", "and", "player"]
                | ["creatures", "and", "players"]
                | ["player", "and", "creature"]
                | ["players", "and", "creatures"]
                | ["creature", "and/or", "player"]
                | ["creatures", "and/or", "players"]
                | ["player", "and/or", "creature"]
                | ["players", "and/or", "creatures"]
        )
    }) || remaining_words.windows(4).any(|window| {
        matches!(
            window,
            ["creature", "and", "or", "player"]
                | ["creatures", "and", "or", "players"]
                | ["player", "and", "or", "creature"]
                | ["players", "and", "or", "creatures"]
        )
    });
    if creature_or_player {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let filter = parse_object_filter(remaining, other)?;
    let it_span = if filter
        .tagged_constraints
        .iter()
        .any(|constraint| constraint.tag.as_str() == IT_TAG)
    {
        tokens
            .iter()
            .rev()
            .find(|token| token.is_word("it"))
            .map(Token::span)
    } else {
        None
    };
    Ok(wrap_target_count(
        TargetAst::Object(filter, target_span, it_span),
        target_count,
    ))
}

fn parse_target_count_range_prefix(tokens: &[Token]) -> Option<(ChoiceCount, usize)> {
    let (first, first_used) = parse_number(tokens)?;
    let or_idx = first_used;
    if !tokens.get(or_idx).is_some_and(|token| token.is_word("or")) {
        return None;
    }
    let (second, second_used) = parse_number(&tokens[or_idx + 1..])?;
    if second < first {
        return None;
    }
    Some((
        ChoiceCount {
            min: first as usize,
            max: Some(second as usize),
            dynamic_x: false,
        },
        first_used + 1 + second_used,
    ))
}

fn wrap_target_count(target: TargetAst, target_count: Option<ChoiceCount>) -> TargetAst {
    if let Some(count) = target_count {
        TargetAst::WithCount(Box::new(target), count)
    } else {
        target
    }
}

fn is_source_from_your_graveyard_words(words: &[&str]) -> bool {
    if words.len() < 4 {
        return false;
    }

    let starts_with_this = words[0] == "this" || words[0] == "thiss";
    let references_source_noun =
        words.contains(&"card") || words.contains(&"creature") || words.contains(&"permanent");

    starts_with_this
        && references_source_noun
        && words.contains(&"from")
        && words.contains(&"your")
        && words.contains(&"graveyard")
}

fn is_source_reference_words(words: &[&str]) -> bool {
    if words.is_empty() {
        return false;
    }

    if words[0] != "this" && words[0] != "thiss" {
        return false;
    }

    if words.len() == 1 {
        return true;
    }

    if words.len() != 2 {
        return false;
    }

    match words[1] {
        "source" | "spell" | "permanent" | "card" | "creature" => true,
        other => parse_card_type(other).is_some(),
    }
}

fn contains_source_from_your_graveyard_phrase(words: &[&str]) -> bool {
    words.windows(5).any(|window| {
        (window[0] == "this" || window[0] == "thiss")
            && matches!(window[1], "card" | "creature" | "permanent")
            && window[2] == "from"
            && window[3] == "your"
            && window[4] == "graveyard"
    })
}

fn is_demonstrative_object_head(word: &str) -> bool {
    if matches!(
        word,
        "creature"
            | "creatures"
            | "permanent"
            | "permanents"
            | "card"
            | "cards"
            | "spell"
            | "spells"
            | "source"
            | "sources"
            | "token"
            | "tokens"
    ) {
        return true;
    }
    if parse_card_type(word).is_some() {
        return true;
    }
    if let Some(singular) = word.strip_suffix('s') {
        return parse_card_type(singular).is_some();
    }
    false
}

fn parse_object_filter(tokens: &[Token], other: bool) -> Result<ObjectFilter, CardTextError> {
    let mut filter = ObjectFilter::default();
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut base_tokens: Vec<Token> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token.is_word("targets") || token.is_word("target") {
            if idx > 0 && tokens[idx - 1].is_word("that") {
                targets_idx = Some(idx);
                break;
            }
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let target_tokens = &tokens[targets_idx + 1..];
        let parse_target_fragment = |fragment_tokens: &[Token]| -> Result<
            (Option<PlayerFilter>, Option<ObjectFilter>),
            CardTextError,
        > {
            let target_words = words(fragment_tokens);
            if target_words.starts_with(&["you"]) {
                return Ok((Some(PlayerFilter::You), None));
            }
            if target_words.starts_with(&["opponent"]) || target_words.starts_with(&["opponents"]) {
                return Ok((Some(PlayerFilter::Opponent), None));
            }
            if target_words.starts_with(&["player"]) || target_words.starts_with(&["players"]) {
                return Ok((Some(PlayerFilter::Any), None));
            }

            let mut target_filter_tokens = fragment_tokens;
            if target_filter_tokens
                .first()
                .is_some_and(|token| token.is_word("target"))
            {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if target_filter_tokens.is_empty() {
                return Ok((None, None));
            }
            Ok((None, Some(parse_object_filter(target_filter_tokens, false)?)))
        };

        let target_words = words(target_tokens);
        if let Some(or_word_idx) = target_words.iter().position(|word| *word == "or")
            && let Some(or_token_idx) = token_index_for_word_index(target_tokens, or_word_idx)
        {
            let left_tokens = trim_commas(&target_tokens[..or_token_idx]);
            let right_tokens = trim_commas(&target_tokens[or_token_idx + 1..]);
            let (left_player, left_object) = parse_target_fragment(&left_tokens)?;
            let (right_player, right_object) = parse_target_fragment(&right_tokens)?;
            target_player = left_player.or(right_player);
            target_object = left_object.or(right_object);
            if target_player.is_some() && target_object.is_some() {
                filter.targets_any_of = true;
            }
        } else {
            let (parsed_player, parsed_object) = parse_target_fragment(target_tokens)?;
            target_player = parsed_player;
            target_object = parsed_object;
        }
    }

    // Object filters should not absorb trailing duration clauses such as
    // "... until this enchantment leaves the battlefield".
    if let Some(until_token_idx) = base_tokens.iter().position(|token| token.is_word("until"))
        && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

    let mut all_words: Vec<&str> = words(&base_tokens)
        .into_iter()
        .filter(|word| !is_article(word) && *word != "instead")
        .collect();

    while all_words.len() >= 2 && all_words[0] == "one" && all_words[1] == "of" {
        all_words.drain(0..2);
    }
    while all_words.len() >= 3
        && all_words[0] == "different"
        && all_words[1] == "one"
        && all_words[2] == "of"
    {
        all_words.drain(0..3);
    }
    while all_words
        .first()
        .is_some_and(|word| matches!(*word, "of" | "from"))
    {
        all_words.remove(0);
    }

    if let Some(idx) = all_words
        .windows(4)
        .position(|window| window == ["that", "isnt", "all", "colors"])
    {
        filter.all_colors = Some(false);
        all_words.drain(idx..idx + 4);
    } else if let Some(idx) = all_words
        .windows(3)
        .position(|window| window == ["isnt", "all", "colors"])
    {
        filter.all_colors = Some(false);
        all_words.drain(idx..idx + 3);
    }

    if let Some(idx) = all_words
        .windows(5)
        .position(|window| window == ["that", "isnt", "exactly", "two", "colors"])
    {
        filter.exactly_two_colors = Some(false);
        all_words.drain(idx..idx + 5);
    } else if let Some(idx) = all_words
        .windows(4)
        .position(|window| window == ["isnt", "exactly", "two", "colors"])
    {
        filter.exactly_two_colors = Some(false);
        all_words.drain(idx..idx + 4);
    }

    if all_words.len() >= 2 && matches!(all_words[0], "that" | "those") {
        let noun_idx = if all_words.get(1).is_some_and(|word| *word == "other") {
            2
        } else {
            1
        };
        if all_words
            .get(noun_idx)
            .is_some_and(|word| is_demonstrative_object_head(word))
        {
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: TagKey::from(IT_TAG),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });
            all_words.remove(0);
        }
    }

    if let Some(idx) = all_words
        .windows(7)
        .position(|window| window == ["that", "entered", "since", "your", "last", "turn", "ended"])
    {
        filter.entered_since_your_last_turn_ended = true;
        all_words.drain(idx..idx + 7);
    } else if let Some(idx) = all_words
        .windows(6)
        .position(|window| window == ["entered", "since", "your", "last", "turn", "ended"])
    {
        filter.entered_since_your_last_turn_ended = true;
        all_words.drain(idx..idx + 6);
    }

    if all_words
        .windows(3)
        .any(|window| window == ["entered", "this", "turn"])
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if all_words.windows(4).any(|window| {
        window == ["counter", "on", "it", "or"] || window == ["counter", "on", "them", "or"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-state object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if all_words
        .iter()
        .any(|word| *word == "face-down" || *word == "facedown")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported face-down object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if all_words.as_slice().starts_with(&["single", "graveyard"])
        || all_words.windows(2).any(|window| window == ["single", "graveyard"])
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported single-graveyard object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if let Some(not_named_idx) = all_words
        .windows(2)
        .position(|window| window == ["not", "named"])
    {
        let mut name_end = all_words.len();
        for idx in (not_named_idx + 2)..all_words.len() {
            if idx == not_named_idx + 2 {
                continue;
            }
            if matches!(
                all_words[idx],
                "in"
                    | "from"
                    | "with"
                    | "without"
                    | "that"
                    | "which"
                    | "who"
                    | "whose"
                    | "under"
                    | "among"
                    | "on"
                    | "you"
                    | "your"
                    | "opponent"
                    | "opponents"
                    | "their"
                    | "its"
                    | "controller"
                    | "controllers"
                    | "owner"
                    | "owners"
            ) {
                name_end = idx;
                break;
            }
        }
        let name_words = &all_words[not_named_idx + 2..name_end];
        if name_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in not-named object filter (clause: '{}')",
                all_words.join(" ")
            )));
        }
        filter.excluded_name = Some(name_words.join(" "));
        let mut remaining = Vec::with_capacity(all_words.len());
        remaining.extend_from_slice(&all_words[..not_named_idx]);
        remaining.extend_from_slice(&all_words[name_end..]);
        all_words = remaining;
    }

    if let Some(named_idx) = all_words.iter().position(|word| *word == "named") {
        let mut name_end = all_words.len();
        for idx in (named_idx + 1)..all_words.len() {
            if idx == named_idx + 1 {
                continue;
            }
            if matches!(
                all_words[idx],
                "in"
                    | "from"
                    | "with"
                    | "without"
                    | "that"
                    | "which"
                    | "who"
                    | "whose"
                    | "under"
                    | "among"
                    | "on"
                    | "you"
                    | "your"
                    | "opponent"
                    | "opponents"
                    | "their"
                    | "its"
                    | "controller"
                    | "controllers"
                    | "owner"
                    | "owners"
            ) {
                name_end = idx;
                break;
            }
        }
        let name_words = &all_words[named_idx + 1..name_end];
        if name_words.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing card name in named object filter (clause: '{}')",
                all_words.join(" ")
            )));
        }
        filter.name = Some(name_words.join(" "));
        let mut remaining = Vec::with_capacity(all_words.len());
        remaining.extend_from_slice(&all_words[..named_idx]);
        remaining.extend_from_slice(&all_words[name_end..]);
        all_words = remaining;
    }

    if all_words.windows(4).any(|window| {
        window == ["one", "or", "more", "colors"] || window == ["one", "or", "more", "color"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported color-count object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    if all_words.windows(3).any(|window| {
        window == ["power", "or", "toughness"] || window == ["toughness", "or", "power"]
    }) {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if all_words.first().is_some_and(|word| *word == "equipped") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("equipped"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    } else if all_words.first().is_some_and(|word| *word == "enchanted") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("enchanted"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        all_words.remove(0);
    }

    if is_source_reference_words(&all_words) {
        filter.source = true;
    }

    if let Some(attached_idx) = all_words.iter().position(|word| *word == "attached")
        && all_words.get(attached_idx + 1) == Some(&"to")
    {
        let attached_to_words = &all_words[attached_idx + 2..];
        let references_it = attached_to_words.starts_with(&["it"])
            || attached_to_words.starts_with(&["that", "object"])
            || attached_to_words.starts_with(&["that", "creature"])
            || attached_to_words.starts_with(&["that", "permanent"])
            || attached_to_words.starts_with(&["that", "equipment"])
            || attached_to_words.starts_with(&["that", "aura"]);
        if references_it {
            let trim_start = if attached_idx >= 2
                && all_words[attached_idx - 2] == "that"
                && matches!(all_words[attached_idx - 1], "were" | "was" | "is" | "are")
            {
                attached_idx - 2
            } else {
                attached_idx
            };
            all_words.truncate(trim_start);
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: IT_TAG.into(),
                relation: TaggedOpbjectRelation::AttachedToTaggedObject,
            });
        }
    }

    let starts_with_exiled_card = all_words.starts_with(&["exiled", "card"])
        || all_words.starts_with(&["exiled", "cards"]);
    let has_exiled_with_phrase = all_words
        .windows(2)
        .any(|window| window == ["exiled", "with"]);
    let owner_only_tail_after_exiled_cards = starts_with_exiled_card
        && all_words
            .iter()
            .skip(2)
            .all(|word| matches!(*word, "you" | "your" | "they" | "their" | "own" | "owns"));
    let is_source_linked_exile_reference = has_exiled_with_phrase
        || (starts_with_exiled_card
            && (all_words.len() == 2 || owner_only_tail_after_exiled_cards));
    if is_source_linked_exile_reference {
        filter.zone = Some(Zone::Exile);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    if all_words.len() == 1 && (all_words[0] == "it" || all_words[0] == "them") {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        return Ok(filter);
    }

    let has_share_card_type = all_words.contains(&"shares")
        && all_words.contains(&"card")
        && all_words.contains(&"type")
        && all_words.contains(&"it");
    let has_share_color = all_words.contains(&"shares")
        && all_words.contains(&"color")
        && all_words.contains(&"it");
    let has_same_mana_value = all_words
        .windows(4)
        .any(|window| window == ["same", "mana", "value", "as"]);
    let references_sacrifice_cost_object = all_words.windows(3).any(|window| {
        matches!(
            window,
            ["the", "sacrificed", "creature"]
                | ["the", "sacrificed", "artifact"]
                | ["the", "sacrificed", "permanent"]
                | ["a", "sacrificed", "creature"]
                | ["a", "sacrificed", "artifact"]
                | ["a", "sacrificed", "permanent"]
        )
    }) || all_words.windows(2).any(|window| {
        matches!(
            window,
            ["sacrificed", "creature"]
                | ["sacrificed", "artifact"]
                | ["sacrificed", "permanent"]
        )
    });
    let references_it_for_mana_value = all_words.iter().any(|word| matches!(*word, "it" | "its"))
        || all_words.windows(2).any(|window| {
            matches!(
                window,
                ["that", "object"]
                    | ["that", "creature"]
                    | ["that", "artifact"]
                    | ["that", "permanent"]
                    | ["that", "spell"]
                    | ["that", "card"]
            )
        });
    let has_same_name_as_tagged_object = all_words.windows(5).any(|window| {
        matches!(
            window,
            ["same", "name", "as", "that", "spell"]
                | ["same", "name", "as", "that", "card"]
                | ["same", "name", "as", "that", "object"]
                | ["same", "name", "as", "that", "creature"]
                | ["same", "name", "as", "that", "permanent"]
        )
    });

    if has_share_card_type {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    }
    if has_share_color {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesColorWithTagged,
        });
    }
    if has_same_mana_value && references_sacrifice_cost_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("sacrifice_cost_0"),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    } else if has_same_mana_value && references_it_for_mana_value {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameManaValueAsTagged,
        });
    }
    if has_same_name_as_tagged_object {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
    }

    if all_words
        .windows(4)
        .any(|window| window == ["that", "convoked", "this", "spell"])
    {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("convoked_this_spell"),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }
    if all_words.windows(3).any(|window| {
        matches!(
            window,
            ["exiled", "this", "way"]
                | ["destroyed", "this", "way"]
                | ["sacrificed", "this", "way"]
                | ["discarded", "this", "way"]
                | ["milled", "this", "way"]
        )
    }) {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
    }

    let references_target_player = all_words.windows(2).any(|window| {
        matches!(
            window,
            ["target", "player"] | ["target", "players"]
        )
    });
    let references_target_opponent = all_words.windows(2).any(|window| {
        matches!(window, ["target", "opponent"] | ["target", "opponents"])
    });
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };
    let is_tagged_spell_reference_at = |idx: usize| {
        all_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| matches!(*prev, "that" | "this" | "its" | "their"))
    };
    let contains_unqualified_spell_word = all_words.iter().enumerate().any(|(idx, word)| {
        matches!(*word, "spell" | "spells") && !is_tagged_spell_reference_at(idx)
    });

    if all_words.len() >= 2 {
        for window in all_words.windows(2) {
            match window {
                ["you", "control"] | ["you", "controls"] => {
                    filter.controller = Some(PlayerFilter::You);
                }
                ["you", "own"] | ["you", "owns"] => {
                    filter.owner = Some(PlayerFilter::You);
                }
                ["opponent", "control"]
                | ["opponent", "controls"]
                | ["opponents", "control"]
                | ["opponents", "controls"] => {
                    filter.controller = Some(PlayerFilter::Opponent);
                }
                ["opponent", "own"]
                | ["opponent", "owns"]
                | ["opponents", "own"]
                | ["opponents", "owns"] => {
                    filter.owner = Some(PlayerFilter::Opponent);
                }
                ["they", "control"] | ["they", "controls"] => {
                    filter.controller = Some(pronoun_player_filter.clone());
                }
                ["they", "own"] | ["they", "owns"] => {
                    filter.owner = Some(pronoun_player_filter.clone());
                }
                _ => {}
            }
        }
    }
    if all_words.len() >= 3 {
        for window in all_words.windows(3) {
            match window {
                ["your", "team", "control"] | ["your", "team", "controls"] => {
                    filter.controller = Some(PlayerFilter::You);
                }
                ["your", "team", "own"] | ["your", "team", "owns"] => {
                    filter.owner = Some(PlayerFilter::You);
                }
                ["that", "player", "control"] | ["that", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::IteratedPlayer);
                }
                ["defending", "player", "control"] | ["defending", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::Defending);
                }
                ["attacking", "player", "control"] | ["attacking", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::Attacking);
                }
                ["that", "player", "own"] | ["that", "player", "owns"] => {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                ["target", "player", "control"] | ["target", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::target_player());
                }
                ["target", "opponent", "control"] | ["target", "opponent", "controls"] => {
                    filter.controller = Some(PlayerFilter::target_opponent());
                }
                ["target", "player", "own"] | ["target", "player", "owns"] => {
                    filter.owner = Some(PlayerFilter::target_player());
                }
                ["target", "opponent", "own"] | ["target", "opponent", "owns"] => {
                    filter.owner = Some(PlayerFilter::target_opponent());
                }
                ["you", "dont", "control"] => {
                    filter.controller = Some(PlayerFilter::NotYou);
                }
                ["you", "dont", "own"] => {
                    filter.owner = Some(PlayerFilter::NotYou);
                }
                _ => {}
            }
        }
    }
    if all_words.len() >= 4 {
        for window in all_words.windows(4) {
            if window[1..] == ["your", "team", "control"]
                || window[1..] == ["your", "team", "controls"]
            {
                filter.controller = Some(PlayerFilter::You);
            } else if window[1..] == ["your", "team", "own"]
                || window[1..] == ["your", "team", "owns"]
            {
                filter.owner = Some(PlayerFilter::You);
            } else if window == ["you", "do", "not", "control"] {
                filter.controller = Some(PlayerFilter::NotYou);
            } else if window == ["you", "do", "not", "own"] {
                filter.owner = Some(PlayerFilter::NotYou);
            }
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if all_words[with_idx] != "with" {
            with_idx += 1;
            continue;
        }

        if all_words
            .get(with_idx + 1)
            .is_some_and(|word| *word == "no")
            && all_words
                .get(with_idx + 2)
                .is_some_and(|word| matches!(*word, "ability" | "abilities"))
        {
            filter.no_abilities = true;
            with_idx += 3;
            continue;
        }

        if all_words
            .get(with_idx + 1)
            .is_some_and(|word| *word == "no")
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[with_idx + 2..])
        {
            filter.without_counter = Some(counter_constraint);
            with_idx += 2 + consumed;
            continue;
        }

        if let Some((kind, consumed)) = parse_alternative_cast_words(&all_words[with_idx + 1..]) {
            filter.alternative_cast = Some(kind);
            with_idx += 1 + consumed;
            continue;
        }
        if let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[with_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            with_idx += 1 + consumed;
            continue;
        }

        if let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&all_words[with_idx + 1..])
        {
            apply_filter_keyword_constraint(&mut filter, constraint, false);
            with_idx += 1 + consumed;
            continue;
        }

        with_idx += 1;
    }

    let mut without_idx = 0usize;
    while without_idx + 1 < all_words.len() {
        if all_words[without_idx] != "without" {
            without_idx += 1;
            continue;
        }

        if let Some((constraint, consumed)) =
            parse_filter_keyword_constraint_words(&all_words[without_idx + 1..])
        {
            apply_filter_keyword_constraint(&mut filter, constraint, true);
            without_idx += 1 + consumed;
            continue;
        }
        if let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[without_idx + 1..])
        {
            filter.without_counter = Some(counter_constraint);
            without_idx += 1 + consumed;
            continue;
        }

        without_idx += 1;
    }

    let has_tap_activated_ability = all_words.windows(9).any(|window| {
        window
            == [
                "has",
                "an",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ]
    }) || all_words.windows(8).any(|window| {
        window
            == [
                "has",
                "activated",
                "ability",
                "with",
                "t",
                "in",
                "its",
                "cost",
            ]
    });
    if has_tap_activated_ability {
        filter.has_tap_activated_ability = true;
    }

    for idx in 0..all_words.len() {
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            let is_reference_zone_for_spell = if contains_unqualified_spell_word {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    "their" => {
                        filter.owner = Some(pronoun_player_filter.clone());
                    }
                    _ => {}
                }
            }
        }
    }

    let clause_words = all_words.clone();
    let mut idx = 0usize;
    while idx < all_words.len() {
        let axis = match all_words[idx] {
            "power" => Some("power"),
            "toughness" => Some("toughness"),
            "mana" if idx + 1 < all_words.len() && all_words[idx + 1] == "value" => {
                Some("mana value")
            }
            _ => None,
        };
        let Some(axis) = axis else {
            idx += 1;
            continue;
        };
        let is_base_reference = idx > 0 && all_words[idx - 1] == "base";

        let axis_word_count = usize::from(axis == "mana value") + 1;
        let value_tokens = if idx + axis_word_count < all_words.len() {
            &all_words[idx + axis_word_count..]
        } else {
            &[]
        };
        let Some((cmp, consumed)) =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words)?
        else {
            idx += 1;
            continue;
        };

        match axis {
            "power" => {
                filter.power = Some(cmp);
                filter.power_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "toughness" => {
                filter.toughness = Some(cmp);
                filter.toughness_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        idx += axis_word_count + consumed;
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    let mut negated_word_indices = std::collections::HashSet::new();
    let mut negated_historic_indices = std::collections::HashSet::new();
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] != "non" {
            continue;
        }
        let next = all_words[idx + 1];
        if is_outlaw_word(next) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(card_type) = parse_card_type(next)
            && !filter.excluded_card_types.contains(&card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(color) = parse_color(next) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(subtype) =
            parse_subtype_word(next).or_else(|| next.strip_suffix('s').and_then(parse_subtype_word))
            && !filter.excluded_subtypes.contains(&subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(idx + 1);
        }
    }
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] == "not" && all_words[idx + 1] == "historic" {
            filter.nonhistoric = true;
            negated_historic_indices.insert(idx + 1);
        }
    }

    for (idx, word) in all_words.iter().enumerate() {
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => {
                if !is_tagged_spell_reference_at(idx) {
                    saw_spell = true;
                }
            }
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "other" => filter.other = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "attacking" => filter.attacking = true,
            "blocking" => filter.blocking = true,
            "commander" | "commanders" => filter.is_commander = true,
            "nonbasic" => {
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            "monocolored" => filter.monocolored = true,
            "nonhistoric" => filter.nonhistoric = true,
            "historic" if !negated_historic_indices.contains(&idx) => filter.historic = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if negated_word_indices.contains(&idx) {
            continue;
        }

        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            saw_subtype = true;
            continue;
        }

        if let Some(card_type) = parse_non_type(word) {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(supertype) = parse_non_supertype(word)
            && !filter.excluded_supertypes.contains(&supertype)
        {
            filter.excluded_supertypes.push(supertype);
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
        }
        if let Some(subtype) = parse_non_subtype(word)
            && !filter.excluded_subtypes.contains(&subtype)
        {
            filter.excluded_subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !filter.supertypes.contains(&supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            if !filter.card_types.contains(&card_type) {
                filter.card_types.push(card_type);
            }
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
        {
            if !filter.subtypes.contains(&subtype) {
                filter.subtypes.push(subtype);
            }
            saw_subtype = true;
        }
    }

    let segments = split_on_or(&base_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_words: Vec<String> = words(segment)
            .into_iter()
            .filter(|word| !is_article(word))
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for word in &segment_words {
            if let Some(card_type) = parse_card_type(word)
                && !types.contains(&card_type)
            {
                types.push(card_type);
            }
            if let Some(subtype) = parse_subtype_word(word)
                .or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
                && !subtypes.contains(&subtype)
            {
                subtypes.push(subtype);
            }
        }
        segment_marker_counts.push(types.len() + subtypes.len());
        if !types.is_empty() {
            segment_types.push(types);
        }
        if !subtypes.is_empty() {
            segment_subtypes.push(subtypes);
        }
    }

    if segments.len() > 1 {
        let qualifier_in_all_segments = |qualifier: &str| {
            segment_words_lists
                .iter()
                .all(|segment| segment.iter().any(|word| word == qualifier))
        };

        if filter.tapped && !qualifier_in_all_segments("tapped") {
            filter.tapped = false;
        }
        if filter.untapped && !qualifier_in_all_segments("untapped") {
            filter.untapped = false;
        }
    }

    if segments.len() > 1 {
        let type_list_candidate = !segment_marker_counts.is_empty()
            && segment_marker_counts.iter().all(|count| *count == 1);

        if type_list_candidate {
            let mut any_types = Vec::new();
            let mut any_subtypes = Vec::new();
            for types in segment_types {
                let Some(card_type) = types.first().copied() else {
                    continue;
                };
                if !any_types.contains(&card_type) {
                    any_types.push(card_type);
                }
            }
            for subtypes in segment_subtypes {
                let Some(subtype) = subtypes.first().copied() else {
                    continue;
                };
                if !any_subtypes.contains(&subtype) {
                    any_subtypes.push(subtype);
                }
            }
            if !any_types.is_empty() {
                filter.card_types = any_types;
            }
            if !any_subtypes.is_empty() {
                filter.subtypes = any_subtypes;
            }
            if !filter.card_types.is_empty() && !filter.subtypes.is_empty() {
                filter.type_or_subtype_union = true;
            }
        }
    } else if let Some(types) = segment_types.into_iter().next() {
        let has_and = all_words.contains(&"and");
        let has_or = all_words.contains(&"or");
        if types.len() > 1 {
            if has_and && !has_or {
                filter.card_types = types;
            } else {
                filter.all_card_types = types;
            }
        } else if types.len() == 1 {
            filter.card_types = types;
        }
    }

    if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
        filter.card_types = vec![
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ];
    }

    if saw_spell && saw_permanent {
        return Err(CardTextError::ParseError(format!(
            "cannot mix spell and permanent targets (clause: '{}')",
            all_words.join(" ")
        )));
    }

    if let Some(zone) = filter.zone {
        if saw_spell && zone != Zone::Stack {
            return Err(CardTextError::ParseError(
                "spell targets must be on the stack".to_string(),
            ));
        }
    } else if saw_spell {
        filter.zone = Some(Zone::Stack);
    } else if saw_permanent || saw_permanent_type || saw_subtype {
        filter.zone = Some(Zone::Battlefield);
    }

    if target_player.is_some() || target_object.is_some() {
        filter = filter.targeting(target_player.take(), target_object.take());
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.blocking
        || filter.is_commander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.custom_static_markers.is_empty()
        || !filter.excluded_custom_static_markers.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    let has_object_identity = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || filter.zone.is_some()
        || filter.token
        || filter.nontoken
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.blocking
        || filter.is_commander
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.custom_static_markers.is_empty()
        || !filter.excluded_custom_static_markers.is_empty()
        || filter.colors.is_some()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some();
    if !has_object_identity {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase lacking object selector (clause: '{}')",
            all_words.join(" ")
        )));
    }

    Ok(filter)
}

fn parse_spell_filter(tokens: &[Token]) -> crate::ability::SpellFilter {
    let mut filter = crate::ability::SpellFilter::default();
    let words: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let clause_words = words.clone();

    let mut idx = 0usize;
    while idx < words.len() {
        if let Some((kind, consumed)) = parse_alternative_cast_words(&words[idx..]) {
            filter.alternative_cast = Some(kind);
            idx += consumed;
            continue;
        }
        let word = words[idx];
        if let Some(card_type) = parse_card_type(word)
            && !filter.card_types.contains(&card_type)
        {
            filter.card_types.push(card_type);
        }
        if let Some(card_type) = parse_non_type(word)
            && !filter.excluded_card_types.contains(&card_type)
        {
            filter.excluded_card_types.push(card_type);
        }

        if let Some(subtype) =
            parse_subtype_word(word).or_else(|| word.strip_suffix('s').and_then(parse_subtype_word))
            && !filter.subtypes.contains(&subtype)
        {
            filter.subtypes.push(subtype);
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
        idx += 1;
    }

    let mut cmp_idx = 0usize;
    while cmp_idx < words.len() {
        let axis = match words[cmp_idx] {
            "power" => Some("power"),
            "toughness" => Some("toughness"),
            "mana" if cmp_idx + 1 < words.len() && words[cmp_idx + 1] == "value" => {
                Some("mana value")
            }
            _ => None,
        };
        let Some(axis) = axis else {
            cmp_idx += 1;
            continue;
        };

        let axis_word_count = usize::from(axis == "mana value") + 1;
        let value_tokens = if cmp_idx + axis_word_count < words.len() {
            &words[cmp_idx + axis_word_count..]
        } else {
            &[]
        };
        let parsed =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words).ok().flatten();
        let Some((cmp, consumed)) = parsed else {
            cmp_idx += 1;
            continue;
        };

        match axis {
            "power" => filter.power = Some(cmp),
            "toughness" => filter.toughness = Some(cmp),
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        cmp_idx += axis_word_count + consumed;
    }

    filter
}

fn spell_filter_has_identity(filter: &crate::ability::SpellFilter) -> bool {
    !filter.card_types.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.subtypes.is_empty()
        || filter.colors.is_some()
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.alternative_cast.is_some()
}

fn merge_spell_filters(
    base: &mut crate::ability::SpellFilter,
    extra: crate::ability::SpellFilter,
) {
    for card_type in extra.card_types {
        if !base.card_types.contains(&card_type) {
            base.card_types.push(card_type);
        }
    }
    for card_type in extra.excluded_card_types {
        if !base.excluded_card_types.contains(&card_type) {
            base.excluded_card_types.push(card_type);
        }
    }
    for subtype in extra.subtypes {
        if !base.subtypes.contains(&subtype) {
            base.subtypes.push(subtype);
        }
    }
    if let Some(colors) = extra.colors {
        let existing = base.colors.unwrap_or(ColorSet::new());
        base.colors = Some(existing.union(colors));
    }
    if base.alternative_cast.is_none() {
        base.alternative_cast = extra.alternative_cast;
    }
    if base.power.is_none() {
        base.power = extra.power;
    }
    if base.toughness.is_none() {
        base.toughness = extra.toughness;
    }
    if base.mana_value.is_none() {
        base.mana_value = extra.mana_value;
    }
}

fn split_on_or(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        let is_separator = matches!(token, Token::Comma(_))
            || (token.is_word("or") && !is_comparison_or_delimiter(tokens, idx));
        if is_separator {
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

fn is_comparison_or_delimiter(tokens: &[Token], idx: usize) -> bool {
    if !tokens.get(idx).is_some_and(|token| token.is_word("or")) {
        return false;
    }
    let previous_word = (0..idx).rev().find_map(|i| tokens[i].as_word());
    let next_word = tokens.get(idx + 1).and_then(Token::as_word);
    if matches!(next_word, Some("less" | "greater" | "more" | "fewer")) {
        return true;
    }
    if previous_word == Some("than") && next_word == Some("equal") {
        return true;
    }
    false
}

fn parse_card_type(word: &str) -> Option<CardType> {
    match word {
        "creature" | "creatures" => Some(CardType::Creature),
        "artifact" | "artifacts" => Some(CardType::Artifact),
        "enchantment" | "enchantments" => Some(CardType::Enchantment),
        "land" | "lands" => Some(CardType::Land),
        "planeswalker" | "planeswalkers" => Some(CardType::Planeswalker),
        "instant" | "instants" => Some(CardType::Instant),
        "sorcery" | "sorceries" => Some(CardType::Sorcery),
        "battle" | "battles" => Some(CardType::Battle),
        "kindred" => Some(CardType::Kindred),
        _ => None,
    }
}

fn parse_non_type(word: &str) -> Option<CardType> {
    let rest = word.strip_prefix("non")?;
    parse_card_type(rest)
}

fn parse_non_supertype(word: &str) -> Option<Supertype> {
    let rest = word.strip_prefix("non")?;
    parse_supertype_word(rest)
}

fn parse_non_color(word: &str) -> Option<ColorSet> {
    let rest = word.strip_prefix("non")?;
    match rest {
        "white" => Some(ColorSet::WHITE),
        "blue" => Some(ColorSet::BLUE),
        "black" => Some(ColorSet::BLACK),
        "red" => Some(ColorSet::RED),
        "green" => Some(ColorSet::GREEN),
        _ => None,
    }
}

fn parse_non_subtype(word: &str) -> Option<Subtype> {
    let rest = word.strip_prefix("non")?;
    parse_subtype_word(rest).or_else(|| rest.strip_suffix('s').and_then(parse_subtype_word))
}

fn is_outlaw_word(word: &str) -> bool {
    matches!(word, "outlaw" | "outlaws")
}

fn is_non_outlaw_word(word: &str) -> bool {
    matches!(word, "nonoutlaw" | "nonoutlaws" | "non-outlaw" | "non-outlaws")
}

fn push_outlaw_subtypes(out: &mut Vec<Subtype>) {
    for subtype in [
        Subtype::Assassin,
        Subtype::Mercenary,
        Subtype::Pirate,
        Subtype::Rogue,
        Subtype::Warlock,
    ] {
        if !out.contains(&subtype) {
            out.push(subtype);
        }
    }
}

fn parse_color(word: &str) -> Option<ColorSet> {
    match word {
        "white" => Some(ColorSet::WHITE),
        "blue" => Some(ColorSet::BLUE),
        "black" => Some(ColorSet::BLACK),
        "red" => Some(ColorSet::RED),
        "green" => Some(ColorSet::GREEN),
        _ => None,
    }
}

fn parse_zone_word(word: &str) -> Option<Zone> {
    match word {
        "battlefield" => Some(Zone::Battlefield),
        "graveyard" | "graveyards" => Some(Zone::Graveyard),
        "hand" | "hands" => Some(Zone::Hand),
        "library" | "libraries" => Some(Zone::Library),
        "exile" | "exiled" => Some(Zone::Exile),
        "stack" => Some(Zone::Stack),
        _ => None,
    }
}

fn parse_alternative_cast_words(words: &[&str]) -> Option<(AlternativeCastKind, usize)> {
    match words {
        ["flashback", ..] => Some((AlternativeCastKind::Flashback, 1)),
        ["jump", "start", ..] => Some((AlternativeCastKind::JumpStart, 2)),
        ["jumpstart", ..] => Some((AlternativeCastKind::JumpStart, 1)),
        ["escape", ..] => Some((AlternativeCastKind::Escape, 1)),
        ["madness", ..] => Some((AlternativeCastKind::Madness, 1)),
        ["miracle", ..] => Some((AlternativeCastKind::Miracle, 1)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterKeywordConstraint {
    Static(StaticAbilityId),
    Marker(&'static str),
}

fn keyword_action_to_filter_constraint(action: KeywordAction) -> Option<FilterKeywordConstraint> {
    use FilterKeywordConstraint::{Marker, Static};
    let ability = match action {
        KeywordAction::Flying => Static(StaticAbilityId::Flying),
        KeywordAction::Menace => Static(StaticAbilityId::Menace),
        KeywordAction::Hexproof => Static(StaticAbilityId::Hexproof),
        KeywordAction::Haste => Static(StaticAbilityId::Haste),
        KeywordAction::FirstStrike => Static(StaticAbilityId::FirstStrike),
        KeywordAction::DoubleStrike => Static(StaticAbilityId::DoubleStrike),
        KeywordAction::Deathtouch => Static(StaticAbilityId::Deathtouch),
        KeywordAction::Lifelink => Static(StaticAbilityId::Lifelink),
        KeywordAction::Vigilance => Static(StaticAbilityId::Vigilance),
        KeywordAction::Trample => Static(StaticAbilityId::Trample),
        KeywordAction::Reach => Static(StaticAbilityId::Reach),
        KeywordAction::Defender => Static(StaticAbilityId::Defender),
        KeywordAction::Flash => Static(StaticAbilityId::Flash),
        KeywordAction::Indestructible => Static(StaticAbilityId::Indestructible),
        KeywordAction::Shroud => Static(StaticAbilityId::Shroud),
        KeywordAction::Wither => Static(StaticAbilityId::Wither),
        KeywordAction::Infect => Static(StaticAbilityId::Infect),
        KeywordAction::Fear => Static(StaticAbilityId::Fear),
        KeywordAction::Intimidate => Static(StaticAbilityId::Intimidate),
        KeywordAction::Shadow => Static(StaticAbilityId::Shadow),
        KeywordAction::Horsemanship => Static(StaticAbilityId::Horsemanship),
        KeywordAction::Flanking => Static(StaticAbilityId::Flanking),
        KeywordAction::Landwalk(subtype) => {
            let marker = match subtype {
                Subtype::Island => "islandwalk",
                Subtype::Swamp => "swampwalk",
                Subtype::Mountain => "mountainwalk",
                Subtype::Forest => "forestwalk",
                Subtype::Plains => "plainswalk",
                _ => return Some(Static(StaticAbilityId::Landwalk)),
            };
            Marker(marker)
        }
        KeywordAction::Bloodthirst(_) => return None,
        KeywordAction::Rampage(_) => return None,
        KeywordAction::Changeling => Static(StaticAbilityId::Changeling),
        _ => return None,
    };
    Some(ability)
}

fn parse_filter_keyword_constraint_words(
    words: &[&str],
) -> Option<(FilterKeywordConstraint, usize)> {
    if words.is_empty() {
        return None;
    }

    let max_len = words.len().min(4);
    for len in (1..=max_len).rev() {
        let tokens = words[..len]
            .iter()
            .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
            .collect::<Vec<_>>();
        let Some(action) = parse_ability_phrase(&tokens) else {
            continue;
        };
        if let Some(constraint) = keyword_action_to_filter_constraint(action) {
            return Some((constraint, len));
        }
    }
    None
}

fn parse_filter_counter_constraint_words(
    words: &[&str],
) -> Option<(crate::filter::CounterConstraint, usize)> {
    if words.len() < 3 {
        return None;
    }
    let counter_idx = words
        .iter()
        .position(|word| *word == "counter" || *word == "counters")?;
    if words.get(counter_idx + 1) != Some(&"on") {
        return None;
    }
    if !words
        .get(counter_idx + 2)
        .is_some_and(|word| matches!(*word, "it" | "them"))
    {
        return None;
    }

    let descriptor_words = words[..counter_idx]
        .iter()
        .copied()
        .filter(|word| !matches!(*word, "a" | "an" | "one" | "or" | "more"))
        .collect::<Vec<_>>();
    let consumed = counter_idx + 3;
    if descriptor_words.is_empty() {
        return Some((crate::filter::CounterConstraint::Any, consumed));
    }
    let descriptor_tokens = descriptor_words
        .iter()
        .map(|word| Token::Word((*word).to_string(), TextSpan::synthetic()))
        .collect::<Vec<_>>();
    let counter_type = parse_counter_type_from_tokens(&descriptor_tokens)?;
    Some((
        crate::filter::CounterConstraint::Typed(counter_type),
        consumed,
    ))
}

fn apply_filter_keyword_constraint(
    filter: &mut ObjectFilter,
    constraint: FilterKeywordConstraint,
    excluded: bool,
) {
    match constraint {
        FilterKeywordConstraint::Static(ability_id) => {
            if excluded {
                if !filter.excluded_static_abilities.contains(&ability_id) {
                    filter.excluded_static_abilities.push(ability_id);
                }
            } else if !filter.static_abilities.contains(&ability_id) {
                filter.static_abilities.push(ability_id);
            }
        }
        FilterKeywordConstraint::Marker(marker) => {
            if excluded {
                if !filter
                    .excluded_custom_static_markers
                    .iter()
                    .any(|value| value.eq_ignore_ascii_case(marker))
                {
                    filter
                        .excluded_custom_static_markers
                        .push(marker.to_string());
                }
            } else if !filter
                .custom_static_markers
                .iter()
                .any(|value| value.eq_ignore_ascii_case(marker))
            {
                filter.custom_static_markers.push(marker.to_string());
            }
        }
    }
}

fn is_permanent_type(card_type: CardType) -> bool {
    matches!(
        card_type,
        CardType::Artifact
            | CardType::Creature
            | CardType::Enchantment
            | CardType::Land
            | CardType::Planeswalker
            | CardType::Battle
    )
}

fn is_article(word: &str) -> bool {
    matches!(word, "a" | "an" | "the")
}

fn parse_number(tokens: &[Token]) -> Option<(u32, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?;

    if let Ok(value) = word.parse::<u32>() {
        return Some((value, 1));
    }

    let value = match word {
        "a" | "an" | "one" => 1,
        "two" => 2,
        "three" => 3,
        "four" => 4,
        "five" => 5,
        "six" => 6,
        "seven" => 7,
        "eight" => 8,
        "nine" => 9,
        "ten" => 10,
        _ => return None,
    };

    Some((value, 1))
}

fn parse_value(tokens: &[Token]) -> Option<(Value, usize)> {
    let token = tokens.first()?;
    let word = token.as_word()?;

    if word == "x" {
        return Some((Value::X, 1));
    }

    let (number, used) = parse_number(tokens)?;
    Some((Value::Fixed(number as i32), used))
}

#[derive(Debug, Clone)]
struct CompileContext {
    next_effect_id: u32,
    next_tag_id: u32,
    last_effect_id: Option<EffectId>,
    last_object_tag: Option<String>,
    last_player_filter: Option<PlayerFilter>,
    iterated_player: bool,
    auto_tag_object_targets: bool,
    allow_life_event_value: bool,
}

impl CompileContext {
    fn new() -> Self {
        Self {
            next_effect_id: 0,
            next_tag_id: 0,
            last_effect_id: None,
            last_object_tag: None,
            last_player_filter: None,
            iterated_player: false,
            auto_tag_object_targets: false,
            allow_life_event_value: false,
        }
    }

    fn next_effect_id(&mut self) -> EffectId {
        let id = EffectId(self.next_effect_id);
        self.next_effect_id += 1;
        id
    }

    fn next_tag(&mut self, prefix: &str) -> String {
        let tag = format!("{prefix}_{}", self.next_tag_id);
        self.next_tag_id += 1;
        tag
    }
}
