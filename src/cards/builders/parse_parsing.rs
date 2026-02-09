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

        if ch == '\'' {
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
    let lower = line.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let full_bytes = full_name.as_bytes();
    let short_bytes = short_name.as_bytes();

    let mut out = String::new();
    let mut map = Vec::new();
    let mut idx = 0;

    while idx < bytes.len() {
        if !full_bytes.is_empty() && bytes[idx..].starts_with(full_bytes) {
            let name_len = full_bytes.len().max(1);
            for j in 0..4 {
                out.push("this".chars().nth(j).unwrap());
                let mapped = base_offset + idx + (j * name_len / 4);
                map.push(mapped);
            }
            idx += full_bytes.len();
            continue;
        }
        if !short_bytes.is_empty() && bytes[idx..].starts_with(short_bytes) {
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
            | ["will", "of", "the", "council"]
            | ["guardian", "protocols"]
            | ["jolly", "gutpipes"]
            | ["protection", "fighting", "style"]
            | ["relentless", "march"]
            | ["secret", "of", "the", "soul"]
            | ["secrets", "of", "the", "soul"]
            | ["flurry", "of", "blows"]
    ) {
        return true;
    }

    matches!(
        words[0],
        "adamant"
            | "addendum"
            | "ascend"
            | "battalion"
            | "bloodrush"
            | "channel"
            | "chroma"
            | "cohort"
            | "constellation"
            | "converge"
            | "coven"
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
            | "protector"
            | "radiance"
            | "raid"
            | "revolt"
            | "spectacle"
            | "strive"
            | "surge"
            | "threshold"
            | "undergrowth"
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
        let mut split_point = None;
        for i in 0..segment.len().saturating_sub(1) {
            if matches!(segment[i], Token::Comma(_))
                && segment.get(i + 1).is_some_and(|t| t.is_word("then"))
            {
                let after_then = &segment[i + 2..];
                let after_words = words(after_then);
                let has_back_ref = after_words.iter().any(|w| back_ref_words.contains(w));
                if !has_back_ref && find_verb(after_then).is_some() {
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

fn parse_line(line: &str, line_index: usize) -> Result<LineAst, CardTextError> {
    parser_trace_line("parse_line:entry", line);
    let normalized = line
        .trim()
        .trim_start_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_ascii_lowercase();
    if normalized.contains("creature token") {
        let starts_with_pt = normalized.split_whitespace().next().is_some_and(|token| {
            let mut chars = token.chars();
            let first = chars.next();
            first.is_some_and(|ch| ch.is_ascii_digit() || ch == '*') && token.contains('/')
        });
        if starts_with_pt {
            return Err(CardTextError::ParseError(format!(
                "unsupported token characteristic line (line: '{line}')"
            )));
        }
    }
    if normalized.starts_with("as this saga enters") {
        return Err(CardTextError::ParseError(format!(
            "unsupported replacement-style saga line (line: '{line}')"
        )));
    }
    if normalized.starts_with("choose one")
        || normalized.starts_with("choose one or more")
        || normalized.starts_with("choose two")
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone modal header line (line: '{line}')"
        )));
    }
    if normalized.starts_with("the ring tempts you") {
        return Err(CardTextError::ParseError(format!(
            "unsupported ring tempts line (line: '{line}')"
        )));
    }
    if normalized.starts_with("play x random fast effects") {
        return Err(CardTextError::ParseError(format!(
            "unsupported debug-only line (line: '{line}')"
        )));
    }
    if normalized.starts_with("activate only as a sorcery") {
        return Err(CardTextError::ParseError(format!(
            "unsupported activation timing restriction line (line: '{line}')"
        )));
    }
    if normalized.starts_with("activate only once each turn") {
        return Err(CardTextError::ParseError(format!(
            "unsupported activation frequency restriction line (line: '{line}')"
        )));
    }
    if normalized.starts_with("this ability triggers only once each turn") {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger frequency restriction line (line: '{line}')"
        )));
    }
    if let Some((chapters, rest)) = parse_saga_chapter_prefix(&normalized) {
        let tokens = tokenize_line(rest, line_index);
        parser_trace("parse_line:branch=saga", &tokens);
        let effects = parse_effect_sentences(&tokens)?;
        return Ok(LineAst::Triggered {
            trigger: TriggerSpec::SagaChapter(chapters),
            effects,
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
        let effects = parse_effect_sentences(effect_tokens)?;
        return Ok(LineAst::AdditionalCost { effects });
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

    if let Some((trigger_idx, _)) = tokens.iter().enumerate().find(|(idx, token)| {
        token.is_word("whenever")
            || token.is_word("when")
            || (token.is_word("at") && tokens.get(*idx + 1).is_some_and(|next| next.is_word("the")))
    }) && trigger_idx <= 2
    {
        parser_trace("parse_line:branch=triggered", &tokens[trigger_idx..]);
        return parse_triggered_line(&tokens[trigger_idx..]);
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

    if find_verb(&tokens).is_some_and(|(_, idx)| idx == 0)
        && let Ok(effects) = parse_effect_sentences(&tokens)
        && !effects.is_empty()
    {
        parser_trace("parse_line:branch=statement-verb-leading", &tokens);
        return Ok(LineAst::Statement { effects });
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

fn starts_with_activation_cost(tokens: &[Token]) -> bool {
    let Some(word) = tokens.first().and_then(Token::as_word) else {
        return false;
    };
    if matches!(
        word,
        "tap" | "t" | "pay" | "discard" | "sacrifice" | "put" | "remove" | "exile" | "e"
    ) {
        return true;
    }
    if word.contains('/') {
        return parse_mana_symbol_group(word).is_ok();
    }
    parse_mana_symbol(word).is_ok()
}

fn parse_ability_line(tokens: &[Token]) -> Option<Vec<KeywordAction>> {
    let segments = split_on_comma(tokens);
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
        KeywordAction::Ward(_) => None,
        KeywordAction::Wither => Some(StaticAbility::wither()),
        KeywordAction::Infect => Some(StaticAbility::infect()),
        KeywordAction::Undying => None,
        KeywordAction::Persist => None,
        KeywordAction::Prowess => None,
        KeywordAction::Exalted => None,
        KeywordAction::Storm => None,
        KeywordAction::Toxic(_) => None,
        KeywordAction::Fear => Some(StaticAbility::fear()),
        KeywordAction::Intimidate => Some(StaticAbility::intimidate()),
        KeywordAction::Shadow => Some(StaticAbility::shadow()),
        KeywordAction::Horsemanship => Some(StaticAbility::horsemanship()),
        KeywordAction::Flanking => Some(StaticAbility::flanking()),
        KeywordAction::Bushido(_) => None,
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
        KeywordAction::ProtectionFromSubtype(_subtype) => None,
        KeywordAction::Unblockable => Some(StaticAbility::unblockable()),
        KeywordAction::Marker(name) => Some(StaticAbility::custom(name, name.to_string())),
    }
}

fn parse_static_ability_line(
    tokens: &[Token],
) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
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
    if let Some(ability) = parse_doesnt_untap_during_untap_step_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_equipped_creature_has_line(tokens)? {
        return Ok(Some(vec![ability]));
    }
    if let Some(ability) = parse_attacks_each_combat_if_able_line(tokens)? {
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
    if let Some(abilities) = parse_cant_clauses(tokens)? {
        return Ok(Some(abilities));
    }
    Ok(None)
}

fn parse_characteristic_defining_pt_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    let has_this_pt = words.windows(4).any(|window| {
        window == ["this", "power", "and", "toughness"]
            || window == ["thiss", "power", "and", "toughness"]
    });
    if !has_this_pt {
        return Ok(None);
    }
    if !(words.contains(&"equal") && words.contains(&"number") && words.contains(&"of")) {
        return Ok(None);
    }

    let Some(number_idx) = tokens.iter().position(|token| token.is_word("number")) else {
        return Ok(None);
    };
    if !tokens
        .get(number_idx + 1)
        .is_some_and(|token| token.is_word("of"))
    {
        return Ok(None);
    }

    let mut filter_tokens = &tokens[number_idx + 2..];
    while filter_tokens
        .last()
        .is_some_and(|token| token.is_word("respectively") || matches!(token, Token::Period(_)))
    {
        filter_tokens = &filter_tokens[..filter_tokens.len().saturating_sub(1)];
    }
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let filter = parse_object_filter(filter_tokens, false)?;
    let value = Value::Count(filter);
    Ok(Some(StaticAbility::characteristic_defining_pt(
        value.clone(),
        value,
    )))
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

    let between_words = &clause_words[spells_idx + 1..cost_idx];
    if between_words
        .windows(2)
        .any(|window| window == ["you", "cast"])
    {
        filter.controller = Some(PlayerFilter::You);
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
    if actions.len() != 1 || !matches!(actions[0], KeywordAction::Indestructible) {
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
    score += usize::from(filter.colorless || filter.multicolored) * 2;
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
        let candidate = &tokens[start..];
        let Ok(filter) = parse_object_filter(candidate, false) else {
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
    if clause_words == ["it", "is", "your", "turn"] || clause_words == ["its", "your", "turn"] {
        return Ok(StaticCondition::YourTurn);
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
    let words = words(tokens);
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
    let have_idx = words
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

    let ability_tokens = &tokens[have_token_idx + 1..];
    let Some(actions) = parse_ability_line(ability_tokens) else {
        return Ok(None);
    };
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
    let clause = parse_anthem_clause(tokens, get_idx, clause_tail_end)?;
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
    let words = words(tokens);
    if is_negated_untap_clause(&words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
            words.join(" ")
        )));
    }
    if words.contains(&"unless") {
        return Ok(None);
    }
    let enter_word_idx = words
        .iter()
        .position(|word| *word == "enter" || *word == "enters");
    let Some(enter_word_idx) = enter_word_idx else {
        return Ok(None);
    };
    let Some(enter_token_idx) = token_index_for_word_index(tokens, enter_word_idx) else {
        return Ok(None);
    };
    if !words
        .iter()
        .skip(enter_word_idx + 1)
        .any(|word| *word == "tapped")
    {
        return Ok(None);
    }
    if words.first().copied() == Some("this") {
        return Ok(None);
    }
    let filter = parse_object_filter(&tokens[..enter_token_idx], false)?;
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
        if let Some(ability) = keyword_action_to_static_ability(action) {
            abilities.push(ability);
        }
    }

    if abilities.is_empty() {
        return Ok(None);
    }

    Ok(Some(StaticAbility::equipment_grant(abilities)))
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

fn parse_doesnt_untap_during_untap_step_line(
    tokens: &[Token],
) -> Result<Option<StaticAbility>, CardTextError> {
    let words = words(tokens);
    if words.len() < 4 {
        return Ok(None);
    }

    let matches = words
        .starts_with(&["this", "doesnt", "untap", "during", "your", "untap", "step"])
        || words.starts_with(&[
            "this", "doesn't", "untap", "during", "your", "untap", "step",
        ])
        || words.starts_with(&[
            "this", "does", "not", "untap", "during", "your", "untap", "step",
        ]);

    if matches {
        return Ok(Some(StaticAbility::doesnt_untap()));
    }

    Ok(None)
}

fn parse_flying_restriction_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let normalized = words(tokens)
        .into_iter()
        .map(|word| if word == "cannot" { "cant" } else { word })
        .collect::<Vec<_>>();

    let matches = normalized.as_slice()
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

    if matches {
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
    if words.as_slice() == ["this", "attacks", "each", "combat", "if", "able"]
        || words.as_slice() == ["attacks", "each", "combat", "if", "able"]
        || words.len() >= 5
            && words[words.len() - 5..] == ["attacks", "each", "combat", "if", "able"]
    {
        return Ok(Some(StaticAbility::must_attack()));
    }
    Ok(None)
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
    if subject_words.contains(&"may") {
        return Ok(None);
    }
    if subject_words
        .first()
        .is_some_and(|word| *word == "enchanted" || *word == "equipped")
        || is_source_reference_words(&subject_words)
    {
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
    } else if looks_like_trigger {
        match parse_triggered_line(ability_tokens)? {
            LineAst::Triggered { trigger, effects } => {
                let (compiled_effects, choices) =
                    compile_trigger_effects(Some(&trigger), &effects)?;
                Some(Ability {
                    kind: AbilityKind::Triggered(TriggeredAbility {
                        trigger: compile_trigger_spec(trigger),
                        effects: compiled_effects,
                        choices,
                        intervening_if: None,
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
    let AnthemSubjectAst::Filter(filter) = subject else {
        return Ok(None);
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
    effect_sentences.retain(|sentence| {
        if is_activate_only_as_sorcery(sentence) {
            timing = ActivationTiming::SorcerySpeed;
            false
        } else if is_activate_only_once_each_turn(sentence) {
            timing = ActivationTiming::OncePerTurn;
            false
        } else {
            true
        }
    });
    if !effect_sentences.is_empty() {
        let primary_sentence = &effect_sentences[0];
        let effect_words = words(primary_sentence);
        if effect_words.contains(&"add") {
            let (mana_cost, cost_effects) = parse_activation_cost(cost_tokens)?;
            let mana_cost = crate::ability::merge_cost_effects(mana_cost, cost_effects);

            let mut extra_effects = Vec::new();
            let mut extra_effects_ast = Vec::new();
            let mut activation_condition = None;
            if effect_sentences.len() > 1 {
                for sentence in &effect_sentences[1..] {
                    if sentence.is_empty() {
                        continue;
                    }
                    if let Some(condition) = parse_activation_condition(sentence) {
                        activation_condition = Some(condition);
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
            let uses_commander_identity = mana_words
                .iter()
                .any(|word| *word == "commander" || *word == "commanders")
                && mana_words.contains(&"identity");
            if has_imprinted_colors || has_any_choice_mana || uses_commander_identity {
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
                if let Some(condition) = activation_condition
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
                    if let Some(condition) = activation_condition
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
                    if let Some(condition) = activation_condition
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
                if let Some(condition) = activation_condition
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
            }),
            functional_zones,
            text: None,
        },
        effects_ast: Some(effects_ast),
    }))
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

fn is_activate_only_as_sorcery(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["activate", "only", "as", "a", "sorcery"]
}

fn is_activate_only_once_each_turn(tokens: &[Token]) -> bool {
    let words = words(tokens);
    words.as_slice() == ["activate", "only", "once", "each", "turn"]
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

    let (mana_cost, cost_effects) = parse_activation_cost(&tokens[cost_start..cost_end])?;
    let mut full_cost_effects = cost_effects;
    full_cost_effects.push(Effect::discard(1));
    let mana_cost = crate::ability::merge_cost_effects(mana_cost, full_cost_effects);

    let cycling_tokens = &tokens[..cost_start];
    let search_filter = parse_cycling_search_filter(cycling_tokens)?;
    let effect = if let Some(filter) = search_filter {
        Effect::search_library_to_hand(filter, true)
    } else {
        Effect::draw(1)
    };

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects: vec![effect],
                choices: Vec::new(),
                timing: ActivationTiming::AnyTime,
            }),
            functional_zones: vec![Zone::Hand],
            text: Some(words_all.join(" ")),
        },
        effects_ast: None,
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
    for word in words.iter().skip(1) {
        if let Ok(symbol) = parse_mana_symbol(word) {
            if matches!(symbol, ManaSymbol::Generic(0)) {
                saw_zero = true;
            } else {
                symbols.push(symbol);
            }
        }
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
    let target = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));

    Ok(Some(ParsedAbility {
        ability: Ability {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: total_cost,
                effects: vec![Effect::attach_to(target.clone())],
                choices: vec![target.clone()],
                timing: ActivationTiming::SorcerySpeed,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some("Equip".to_string()),
        },
        effects_ast: None,
    }))
}

fn parse_activation_cost(tokens: &[Token]) -> Result<(TotalCost, Vec<Effect>), CardTextError> {
    let mut mana_pips: Vec<Vec<ManaSymbol>> = Vec::new();
    let mut cost_effects = Vec::new();
    let mut explicit_costs = Vec::new();
    let mut energy_count: u32 = 0;
    let mut sac_tag_id = 0u32;
    let mut tap_tag_id = 0u32;
    let mut exile_tag_id = 0u32;

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
                cost_effects.push(Effect::tap_source());
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
            cost_effects.push(Effect::choose_objects(
                filter,
                count as usize,
                PlayerFilter::You,
                tag.clone(),
            ));
            cost_effects.push(Effect::tap(ChooseSpec::tagged(tag)));
            continue;
        }

        if segment_words[0] == "pay" {
            if segment_words.contains(&"life") {
                let amount = parse_number(&segment[1..]).ok_or_else(|| {
                    CardTextError::ParseError("unable to parse pay life cost".to_string())
                })?;
                cost_effects.push(Effect::pay_life(amount.0));
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
            let count = parse_number(&segment[1..])
                .map(|value| value.0)
                .unwrap_or(1);
            cost_effects.push(Effect::discard(count));
            continue;
        }

        if segment_words[0] == "sacrifice" {
            if segment_words.get(1).copied() == Some("this") {
                cost_effects.push(Effect::sacrifice_source());
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
            cost_effects.push(Effect::choose_objects(
                filter,
                count as usize,
                PlayerFilter::You,
                tag.clone(),
            ));
            cost_effects.push(Effect::sacrifice(ObjectFilter::tagged(tag), count));
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
                cost_effects.push(Effect::exile_from_hand_as_cost(count, color_filter));
                continue;
            }

            let mut filter_tokens = &segment[1..];
            let mut generic_count = 1usize;
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
                // "Exile the top ... card of your graveyard" still maps through object filters.
                filter_tokens = &filter_tokens[1..];
            }
            while filter_tokens.first().is_some_and(|token| {
                token.is_word("the") || token.is_word("a") || token.is_word("an")
            }) {
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
                cost_effects.push(Effect::exile(ChooseSpec::Source));
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
            cost_effects.push(Effect::new(
                crate::effects::ChooseObjectsEffect::new(
                    filter,
                    generic_count,
                    PlayerFilter::You,
                    tag.clone(),
                )
                .in_zone(choice_zone),
            ));
            cost_effects.push(Effect::exile(ChooseSpec::tagged(tag)));
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
            let (count, used) = parse_number(&segment[1..]).ok_or_else(|| {
                CardTextError::ParseError("unable to parse remove counter cost amount".to_string())
            })?;
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
                explicit_costs.push(crate::costs::Cost::new(
                    crate::costs::RemoveAnyCountersAmongCost::new(count, filter),
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
                explicit_costs.push(crate::costs::Cost::remove_counters(counter_type, count));
            } else {
                let mut filter = parse_object_filter(remaining, false)?;
                if filter.controller.is_none() {
                    filter.controller = Some(PlayerFilter::You);
                }
                if filter.zone.is_none() {
                    filter.zone = Some(Zone::Battlefield);
                }
                explicit_costs.push(crate::costs::Cost::new(
                    crate::costs::RemoveAnyCountersAmongCost::new(count, filter),
                ));
            }
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
    let words = words(tokens);
    if words.len() < 5 {
        return None;
    }
    if !words.starts_with(&["activate", "only", "if", "you", "control"]) {
        return None;
    }

    let mut subtypes = Vec::new();
    for word in words {
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

fn parse_enters_tapped_line(tokens: &[Token]) -> Result<Option<StaticAbility>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.is_empty() {
        return Ok(None);
    }
    if is_negated_untap_clause(&clause_words) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mixed enters-tapped and negated-untap clause (clause: '{}')",
            clause_words.join(" ")
        )));
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

fn parse_cant_clauses(tokens: &[Token]) -> Result<Option<Vec<StaticAbility>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }

        let mut abilities = Vec::new();
        for segment in segments {
            let Some(ability) = parse_cant_clause(&segment)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant clause segment (clause: '{}')",
                    words(&segment).join(" ")
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
) -> Result<Option<Vec<crate::effect::Restriction>>, CardTextError> {
    if find_negation_span(tokens).is_none() {
        return Ok(None);
    }

    if tokens.iter().any(|token| token.is_word("and")) {
        let segments = split_on_and(tokens);
        if segments.is_empty() {
            return Ok(None);
        }

        let mut restrictions = Vec::new();
        for segment in segments {
            let Some(restriction) = parse_cant_restriction_clause(&segment)? else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported cant restriction segment (clause: '{}')",
                    words(&segment).join(" ")
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
) -> Result<Option<crate::effect::Restriction>, CardTextError> {
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

    Ok(Some(restriction))
}

fn parse_negated_object_restriction_clause(
    tokens: &[Token],
) -> Result<Option<crate::effect::Restriction>, CardTextError> {
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

    let Some(filter) = parse_subject_object_filter(&subject_tokens)? else {
        return Err(CardTextError::ParseError(format!(
            "unsupported subject in negated restriction clause (clause: '{}')",
            words(tokens).join(" ")
        )));
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

    Ok(Some(restriction))
}

fn find_negation_span(tokens: &[Token]) -> Option<(usize, usize)> {
    for (idx, token) in tokens.iter().enumerate() {
        let Some(word) = token.as_word() else {
            continue;
        };
        if matches!(word, "cant" | "cannot" | "doesnt") {
            return Some((idx, idx + 1));
        }
        if (word == "does" || word == "can")
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

fn parse_ability_phrase(tokens: &[Token]) -> Option<KeywordAction> {
    let mut words = words(tokens);
    if words.is_empty() {
        return None;
    }

    if words.first().copied() == Some("and") {
        words.remove(0);
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

    if let Some(first) = words.first().copied()
        && matches!(
            first,
            "banding" | "morph" | "megamorph" | "fabricate" | "bestow" | "rampage"
        )
    {
        return Some(KeywordAction::Marker(match first {
            "banding" => "banding",
            "morph" => "morph",
            "megamorph" => "megamorph",
            "fabricate" => "fabricate",
            "bestow" => "bestow",
            "rampage" => "rampage",
            _ => unreachable!(),
        }));
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
        ["devoid"] => KeywordAction::Marker("devoid"),
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
        ["islandwalk"] => KeywordAction::Marker("islandwalk"),
        ["swampwalk"] => KeywordAction::Marker("swampwalk"),
        ["mountainwalk"] => KeywordAction::Marker("mountainwalk"),
        ["forestwalk"] => KeywordAction::Marker("forestwalk"),
        ["plainswalk"] => KeywordAction::Marker("plainswalk"),
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
                    "devoid" => return Some(KeywordAction::Marker("devoid")),
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
                    "islandwalk" => return Some(KeywordAction::Marker("islandwalk")),
                    "swampwalk" => return Some(KeywordAction::Marker("swampwalk")),
                    "mountainwalk" => return Some(KeywordAction::Marker("mountainwalk")),
                    "forestwalk" => return Some(KeywordAction::Marker("forestwalk")),
                    "plainswalk" => return Some(KeywordAction::Marker("plainswalk")),
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
                    return Some(KeywordAction::FirstStrike);
                }
                if words.starts_with(&["double", "strike"]) {
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

    if let Some(comma_idx) = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .or_else(|| tokens.iter().position(|token| token.is_word("then")))
    {
        let trigger_tokens = &tokens[start_idx..comma_idx];
        let trigger = parse_trigger_clause(trigger_tokens)?;
        let effects_tokens = &tokens[comma_idx + 1..];
        let effects = parse_effect_sentences(effects_tokens)?;
        return Ok(LineAst::Triggered { trigger, effects });
    }

    // Some oracle lines omit the comma after the trigger clause.
    for split_idx in ((start_idx + 1)..tokens.len()).rev() {
        let trigger_tokens = &tokens[start_idx..split_idx];
        let effects_tokens = &tokens[split_idx..];
        if effects_tokens.is_empty() {
            continue;
        }
        if let Ok(trigger) = parse_trigger_clause(trigger_tokens)
            && let Ok(effects) = parse_effect_sentences(effects_tokens)
        {
            return Ok(LineAst::Triggered { trigger, effects });
        }
    }

    Err(CardTextError::ParseError(format!(
        "missing comma in triggered line (clause: '{}')",
        words(tokens).join(" ")
    )))
}

fn parse_trigger_clause(tokens: &[Token]) -> Result<TriggerSpec, CardTextError> {
    let words = words(tokens);

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

    if let Some(or_idx) = tokens.iter().position(|token| token.is_word("or")) {
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
    if words.contains(&"cast")
        && words.contains(&"this")
        && words.contains(&"spell")
        && words.contains(&"you")
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
        if let Ok(mut filter) = parse_object_filter(subject_tokens, false) {
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
        let player = if subject == ["you"] {
            Some(PlayerFilter::You)
        } else if subject == ["a", "player"]
            || subject == ["any", "player"]
            || subject == ["player"]
        {
            Some(PlayerFilter::Any)
        } else if subject == ["an", "opponent"] || subject == ["opponent"] {
            Some(PlayerFilter::Opponent)
        } else {
            None
        };
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

    if words.as_slice() == ["you", "draw", "a", "card"] {
        return Ok(TriggerSpec::YouDrawCard);
    }

    let last = words
        .last()
        .ok_or_else(|| CardTextError::ParseError("empty trigger clause".to_string()))?;

    match *last {
        "attacks" => {
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
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfEndStep(player))
        }
        _ if words.contains(&"beginning") && words.contains(&"upkeep") => {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfUpkeep(player))
        }
        _ if words.contains(&"beginning") && words.contains(&"draw") && words.contains(&"step") => {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfDrawStep(player))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"combat")
            && words.contains(&"turn") =>
        {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfCombat(player))
        }
        _ if words.contains(&"beginning")
            && words.contains(&"precombat")
            && words.contains(&"main") =>
        {
            let player = if words.contains(&"your") {
                PlayerFilter::You
            } else if words.contains(&"opponent") || words.contains(&"opponents") {
                PlayerFilter::Opponent
            } else {
                PlayerFilter::Any
            };
            Ok(TriggerSpec::BeginningOfPrecombatMain(player))
        }
        _ => Err(CardTextError::ParseError(format!(
            "unsupported trigger clause (clause: '{}')",
            words.join(" ")
        ))),
    }
}

fn parse_trigger_subject_filter(
    subject_tokens: &[Token],
) -> Result<Option<ObjectFilter>, CardTextError> {
    if subject_tokens.is_empty() {
        return Ok(None);
    }

    let mut subject_tokens = subject_tokens;
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

    let actor = if clause_words.contains(&"you") {
        PlayerFilter::You
    } else if clause_words.contains(&"opponent") || clause_words.contains(&"opponents") {
        PlayerFilter::Opponent
    } else {
        PlayerFilter::Any
    };

    let parse_filter = |filter_tokens: &[Token]| -> Result<Option<ObjectFilter>, CardTextError> {
        let filter_words: Vec<&str> = filter_tokens.iter().filter_map(Token::as_word).collect();
        let is_unqualified_spell = filter_words.as_slice() == ["a", "spell"]
            || filter_words.as_slice() == ["spells"]
            || filter_words.as_slice() == ["spell"];
        if filter_tokens.is_empty() || is_unqualified_spell {
            Ok(None)
        } else {
            Ok(Some(parse_object_filter(filter_tokens, false)?))
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

fn parse_target_player_choose_objects_clause(
    tokens: &[Token],
) -> Result<Option<(PlayerAst, ObjectFilter)>, CardTextError> {
    let clause_words = words(tokens);
    if clause_words.len() < 4 || clause_words.first().copied() != Some("target") {
        return Ok(None);
    }

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

    let choose_object_tokens = trim_commas(&tokens[3..]);
    if choose_object_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing chosen object after target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let mut choose_filter = parse_object_filter(&choose_object_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported chosen object filter in target-player choose clause (clause: '{}')",
            clause_words.join(" ")
        ))
    })?;
    if choose_filter.controller.is_none() {
        choose_filter.controller = Some(match chooser {
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            _ => PlayerFilter::target_player(),
        });
    }

    Ok(Some((chooser, choose_filter)))
}

fn parse_target_player_chooses_then_other_cant_block(
    first: &[Token],
    second: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some((chooser, mut choose_filter)) = parse_target_player_choose_objects_clause(first)?
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
            count: ChoiceCount::exactly(1),
            player: chooser,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::Cant {
            restriction: crate::effect::Restriction::block(restriction_filter),
            duration: Until::EndOfTurn,
        },
    ]))
}

fn parse_effect_sentences(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::new();
    let sentences = split_on_period(tokens);
    let mut sentence_idx = 0usize;

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
        parser_trace("parse_effect_sentences:sentence", &sentence);

        if is_spawn_scion_token_mana_reminder(&sentence) {
            if effects
                .last()
                .is_some_and(effect_creates_eldrazi_spawn_or_scion)
            {
                parser_trace("parse_effect_sentences:spawn-scion-reminder", &sentence);
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token mana reminder clause (clause: '{}')",
                words(&sentence).join(" ")
            )));
        }

        let mut sentence_effects = parse_effect_sentence(&sentence)?;
        if try_apply_token_copy_followup(&mut effects, &sentence_effects)? {
            parser_trace("parse_effect_sentences:token-copy-followup", &sentence);
            sentence_idx += 1;
            continue;
        }
        if sentence_effects.is_empty() && !is_round_up_each_time_sentence(&sentence) {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                words(&sentence).join(" ")
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
            }
        }
        let has_instead = sentence.iter().any(|token| token.is_word("instead"));
        if has_instead && sentence_effects.len() == 1 && effects.len() >= 1 {
            if matches!(
                sentence_effects.first(),
                Some(EffectAst::Conditional { .. })
            ) {
                let previous = effects.pop().expect("effects length checked above");
                if let Some(EffectAst::Conditional {
                    predicate,
                    if_true,
                    mut if_false,
                }) = sentence_effects.pop()
                {
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

    let Some((haste, sacrifice)) = (match sentence_effects.first() {
        Some(EffectAst::TokenCopyGainHasteUntilEot) => Some((true, false)),
        Some(EffectAst::TokenCopySacrificeAtNextEndStep) => Some((false, true)),
        _ => None,
    }) else {
        return Ok(false);
    };

    match last {
        EffectAst::CreateTokenCopy {
            has_haste,
            sacrifice_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            sacrifice_at_next_end_step,
            ..
        } => {
            if haste {
                *has_haste = true;
            }
            if sacrifice {
                *sacrifice_at_next_end_step = true;
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
    Ok(parse_earthbend_sentence(tokens)?.map(|effect| vec![effect]))
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

fn parse_sentence_play_from_graveyard(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(parse_play_from_graveyard_sentence(tokens)?.map(|effect| vec![effect]))
}

fn parse_sentence_look_at_hand(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_look_at_hand_sentence(tokens)
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

fn parse_sentence_same_name_gets_fanout(
    tokens: &[Token],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_same_name_gets_fanout_sentence(tokens)
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

fn parse_sentence_unless_pays(tokens: &[Token]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // Find "unless" in the token stream
    let unless_idx = match tokens.iter().position(|t| t.is_word("unless")) {
        Some(idx) => idx,
        None => return Ok(None),
    };

    // Need at least something before "unless" and something after
    if unless_idx == 0 {
        return Ok(None);
    }

    let before_words: Vec<&str> = tokens[..unless_idx]
        .iter()
        .filter_map(Token::as_word)
        .collect();

    // Skip "counter ... unless" - already handled by parse_counter via CounterUnlessPays
    if before_words.first() == Some(&"counter") {
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
    let effects = parse_effect_chain(effect_tokens)?;
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
    } else if after_words.starts_with(&["they"]) {
        (PlayerAst::That, 1)
    } else if after_words.starts_with(&["that", "player"]) {
        (PlayerAst::That, 2)
    } else if after_words.starts_with(&["its", "controller"]) {
        (PlayerAst::ItsController, 2)
    } else if after_words.starts_with(&["their", "controller"]) {
        (PlayerAst::ItsController, 2)
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
        }
    }

    // Try action-based alternative: "unless you [verb] [object]"
    // Both the main effects and the alternative must parse successfully.
    if let Ok(alternative) = parse_effect_chain(action_tokens) {
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
        name: "token-copy-modifier",
        parser: parse_sentence_token_copy_modifier,
    },
    SentencePrimitive {
        name: "each-player-choose-keep-rest-sacrifice",
        parser: parse_sentence_each_player_choose_and_sacrifice_rest,
    },
    SentencePrimitive {
        name: "exile-instead-of-graveyard",
        parser: parse_sentence_exile_instead_of_graveyard,
    },
];

const POST_CONDITIONAL_SENTENCE_PRIMITIVES: &[SentencePrimitive] = &[
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
        name: "exile-then-return-same-object",
        parser: parse_sentence_exile_then_return_same_object,
    },
    SentencePrimitive {
        name: "search-library",
        parser: parse_sentence_search_library,
    },
    SentencePrimitive {
        name: "play-from-graveyard",
        parser: parse_sentence_play_from_graveyard,
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
        name: "same-name-gets-fanout",
        parser: parse_sentence_same_name_gets_fanout,
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
        name: "unless-pays",
        parser: parse_sentence_unless_pays,
    },
];

fn parse_effect_sentence(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    parser_trace("parse_effect_sentence:entry", tokens);
    let sentence_words = words(tokens);
    if sentence_words.starts_with(&["activate", "only"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported activation restriction clause (clause: '{}')",
            sentence_words.join(" ")
        )));
    }
    if sentence_words.starts_with(&["this", "ability", "triggers", "only"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported trigger restriction clause (clause: '{}')",
            sentence_words.join(" ")
        )));
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

    parse_effect_chain(tokens)
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
        || words.windows(2).any(|pair| pair == ["does", "not"])
        || words.contains(&"cant")
        || words.windows(2).any(|pair| pair == ["can", "not"]);
    has_untap && has_negation
}

fn parse_token_copy_modifier_sentence(tokens: &[Token]) -> Option<EffectAst> {
    let filtered: Vec<&str> = words(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if filtered.starts_with(&["it", "gains", "haste"])
        || filtered.starts_with(&["they", "gain", "haste"])
    {
        let has_until_eot = filtered
            .windows(3)
            .any(|window| window == ["until", "end", "of"])
            && filtered.contains(&"turn");
        if has_until_eot {
            return Some(EffectAst::TokenCopyGainHasteUntilEot);
        }
    }

    if filtered.starts_with(&["sacrifice", "it"]) || filtered.starts_with(&["sacrifice", "them"]) {
        let has_next_end_step = filtered
            .windows(6)
            .any(|window| window == ["at", "beginning", "of", "next", "end", "step"]);
        if has_next_end_step {
            return Some(EffectAst::TokenCopySacrificeAtNextEndStep);
        }
    }

    None
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

    if let Some(mv) = &specific.mana_value {
        merged.mana_value = Some(mv.clone());
    }
    if specific.has_mana_cost {
        merged.has_mana_cost = true;
    }
    if specific.no_x_in_cost {
        merged.no_x_in_cost = true;
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
    let words_all = words(tokens);
    let Some(verb) = words_all.first().copied() else {
        return Ok(None);
    };
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

    let first_target = parse_target_phrase(&first_target_tokens)?;
    let first_effect = match verb {
        "destroy" => EffectAst::Destroy {
            target: first_target,
        },
        "exile" => EffectAst::Exile {
            target: first_target,
        },
        "return" => EffectAst::ReturnToHand {
            target: first_target,
        },
        _ => unreachable!("verb already filtered"),
    };
    let second_effect = match verb {
        "destroy" => EffectAst::DestroyAll { filter },
        "exile" => EffectAst::ExileAll { filter },
        "return" => EffectAst::ReturnAllToHand { filter },
        _ => unreachable!("verb already filtered"),
    };

    Ok(Some(vec![first_effect, second_effect]))
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

    let mut effects = Vec::new();
    for segment in split_on_and(&tokens[2..]) {
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
    let words_all = words(tokens);
    if words_all.first().copied() != Some("exile")
        || !words_all.contains(&"then")
        || !words_all.contains(&"return")
    {
        return Ok(None);
    }

    let split_idx = (0..tokens.len().saturating_sub(2)).find(|idx| {
        matches!(tokens[*idx], Token::Comma(_))
            && tokens[*idx + 1].is_word("then")
            && tokens[*idx + 2].is_word("return")
    });
    let Some(split_idx) = split_idx else {
        return Ok(None);
    };

    let first_clause = &tokens[..split_idx];
    let second_clause = &tokens[split_idx + 2..];
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
            target: TargetAst::Tagged(tag, _),
            tapped,
        } if tag.as_str() == IT_TAG => EffectAst::ReturnToBattlefield {
            target: TargetAst::Tagged(TagKey::from("exiled_0"), None),
            tapped,
        },
        EffectAst::ReturnToHand {
            target: TargetAst::Tagged(tag, _),
        } if tag.as_str() == IT_TAG => EffectAst::ReturnToHand {
            target: TargetAst::Tagged(TagKey::from("exiled_0"), None),
        },
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
        let target = TargetAst::Player(PlayerFilter::Any, None);
        return Ok(Some(vec![EffectAst::LookAtHand { target }]));
    }
    Ok(None)
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
    let gain_idx = words
        .iter()
        .position(|word| *word == "gain" || *word == "gains");
    let Some(gain_idx) = gain_idx else {
        return Ok(None);
    };

    let after_gain = &words[gain_idx + 1..];
    if after_gain.contains(&"life") || after_gain.contains(&"control") {
        return Ok(None);
    }

    let duration = if after_gain.contains(&"until")
        && after_gain.contains(&"end")
        && after_gain.contains(&"turn")
    {
        Until::EndOfTurn
    } else {
        Until::EndOfTurn
    };

    let ability_tokens = if let Some(until_idx) = tokens.iter().position(|t| t.is_word("until")) {
        if until_idx > gain_idx + 1 {
            &tokens[gain_idx + 1..until_idx]
        } else {
            &tokens[gain_idx + 1..]
        }
    } else {
        &tokens[gain_idx + 1..]
    };

    let Some(actions) = parse_ability_line(ability_tokens) else {
        return Ok(None);
    };

    let abilities: Vec<StaticAbility> = actions
        .into_iter()
        .filter_map(keyword_action_to_static_ability)
        .collect();
    if abilities.is_empty() {
        return Ok(None);
    }

    // Check for "gets +X/+Y and gains KEYWORD" pattern - if there's a pump modifier
    // before the "gain"/"gains", extract it as a separate Pump/PumpAll effect.
    let before_gain = &words[..gain_idx];
    let get_idx = before_gain.iter().position(|w| *w == "get" || *w == "gets");
    let pump_effect = if let Some(gi) = get_idx {
        let mod_word = before_gain.get(gi + 1).copied().unwrap_or("");
        if let Ok((power, toughness)) = parse_pt_modifier(mod_word) {
            Some((power, toughness, gi))
        } else {
            None
        }
    } else {
        None
    };

    // Determine the real subject (before "get"/"gets" if pump is present)
    let real_subject_end = pump_effect
        .as_ref()
        .map(|(_, _, gi)| *gi)
        .unwrap_or(gain_idx);
    let real_subject_tokens = &tokens[..real_subject_end];

    let mut effects = Vec::new();

    // Check for pronoun subjects ("it", "they") that reference a prior tagged object.
    let real_subject_words: Vec<&str> = real_subject_tokens
        .iter()
        .filter_map(Token::as_word)
        .collect();
    let is_pronoun_subject =
        real_subject_words.as_slice() == ["it"] || real_subject_words.as_slice() == ["they"];
    if is_pronoun_subject {
        let span = span_from_tokens(real_subject_tokens);
        let target = TargetAst::Tagged(TagKey::from(IT_TAG), span);
        if let Some((power, toughness, _)) = pump_effect {
            effects.push(EffectAst::Pump {
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                target: target.clone(),
                duration: duration.clone(),
            });
        }
        effects.push(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        });
        return Ok(Some(effects));
    }

    if words[..gain_idx].contains(&"target") {
        if let Some((power, toughness, _)) = pump_effect {
            let target = parse_target_phrase(real_subject_tokens)?;
            effects.push(EffectAst::Pump {
                power: Value::Fixed(power),
                toughness: Value::Fixed(toughness),
                target,
                duration: duration.clone(),
            });
        }
        let target = parse_target_phrase(real_subject_tokens)?;
        effects.push(EffectAst::GrantAbilitiesToTarget {
            target,
            abilities,
            duration,
        });
        return Ok(Some(effects));
    }

    let filter = parse_object_filter(real_subject_tokens, false).map_err(|_| {
        CardTextError::ParseError(format!(
            "unsupported subject in gain-ability clause (clause: '{}')",
            words.join(" ")
        ))
    })?;

    if let Some((power, toughness, _)) = pump_effect {
        effects.push(EffectAst::PumpAll {
            filter: filter.clone(),
            power: Value::Fixed(power),
            toughness: Value::Fixed(toughness),
            duration: duration.clone(),
        });
    }
    effects.push(EffectAst::GrantAbilitiesAll {
        filter,
        abilities,
        duration,
    });

    Ok(Some(effects))
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

    let filter_end = search_tokens[for_idx + 1..]
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .map(|idx| for_idx + 1 + idx)
        .or_else(|| {
            search_tokens
                .iter()
                .position(|token| token.is_word("reveal") || token.is_word("then"))
        })
        .unwrap_or(put_idx);

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
        if let Some((value, used)) = parse_number(&count_tokens[2..]) {
            count = ChoiceCount::up_to(value as usize);
            count_used = 2 + used;
        }
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
    let effects = if split_battlefield_and_hand {
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

    Ok(Some(effects))
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

    let effects = restrictions
        .into_iter()
        .map(|restriction| EffectAst::Cant {
            restriction,
            duration: duration.clone(),
        })
        .collect();

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
        "gate" | "gates" => Some(Subtype::Gate),
        "advisor" => Some(Subtype::Advisor),
        "ally" | "allies" => Some(Subtype::Ally),
        "angel" => Some(Subtype::Angel),
        "ape" => Some(Subtype::Ape),
        "archer" => Some(Subtype::Archer),
        "artificer" => Some(Subtype::Artificer),
        "assassin" => Some(Subtype::Assassin),
        "astartes" => Some(Subtype::Astartes),
        "avatar" => Some(Subtype::Avatar),
        "barbarian" => Some(Subtype::Barbarian),
        "bard" => Some(Subtype::Bard),
        "bear" => Some(Subtype::Bear),
        "beast" => Some(Subtype::Beast),
        "berserker" => Some(Subtype::Berserker),
        "bird" => Some(Subtype::Bird),
        "boar" => Some(Subtype::Boar),
        "cat" => Some(Subtype::Cat),
        "centaur" => Some(Subtype::Centaur),
        "changeling" => Some(Subtype::Changeling),
        "cleric" => Some(Subtype::Cleric),
        "construct" => Some(Subtype::Construct),
        "crab" => Some(Subtype::Crab),
        "crocodile" => Some(Subtype::Crocodile),
        "dauthi" => Some(Subtype::Dauthi),
        "demon" => Some(Subtype::Demon),
        "devil" => Some(Subtype::Devil),
        "dinosaur" => Some(Subtype::Dinosaur),
        "djinn" => Some(Subtype::Djinn),
        "efreet" | "efreets" => Some(Subtype::Efreet),
        "dog" => Some(Subtype::Dog),
        "dragon" => Some(Subtype::Dragon),
        "drake" => Some(Subtype::Drake),
        "druid" => Some(Subtype::Druid),
        "dwarf" => Some(Subtype::Dwarf),
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
        "goat" => Some(Subtype::Goat),
        "goblin" => Some(Subtype::Goblin),
        "god" => Some(Subtype::God),
        "golem" => Some(Subtype::Golem),
        "gorgon" => Some(Subtype::Gorgon),
        "griffin" => Some(Subtype::Griffin),
        "hag" => Some(Subtype::Hag),
        "halfling" => Some(Subtype::Halfling),
        "harpy" => Some(Subtype::Harpy),
        "hippo" => Some(Subtype::Hippo),
        "horror" => Some(Subtype::Horror),
        "horse" => Some(Subtype::Horse),
        "hound" => Some(Subtype::Hound),
        "human" => Some(Subtype::Human),
        "hydra" => Some(Subtype::Hydra),
        "illusion" => Some(Subtype::Illusion),
        "imp" => Some(Subtype::Imp),
        "insect" => Some(Subtype::Insect),
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
        "moonfolk" => Some(Subtype::Moonfolk),
        "mouse" => Some(Subtype::Mouse),
        "mutant" => Some(Subtype::Mutant),
        "myr" => Some(Subtype::Myr),
        "naga" => Some(Subtype::Naga),
        "nightmare" => Some(Subtype::Nightmare),
        "ninja" => Some(Subtype::Ninja),
        "noble" => Some(Subtype::Noble),
        "octopus" => Some(Subtype::Octopus),
        "ogre" => Some(Subtype::Ogre),
        "ooze" => Some(Subtype::Ooze),
        "orc" => Some(Subtype::Orc),
        "otter" => Some(Subtype::Otter),
        "ox" => Some(Subtype::Ox),
        "oyster" => Some(Subtype::Oyster),
        "peasant" => Some(Subtype::Peasant),
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
        "rebel" => Some(Subtype::Rebel),
        "rhino" => Some(Subtype::Rhino),
        "rogue" => Some(Subtype::Rogue),
        "robot" => Some(Subtype::Robot),
        "salamander" => Some(Subtype::Salamander),
        "samurai" => Some(Subtype::Samurai),
        "satyr" => Some(Subtype::Satyr),
        "scarecrow" => Some(Subtype::Scarecrow),
        "scout" => Some(Subtype::Scout),
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

    let has_doesnt =
        words.contains(&"doesnt") || words.windows(2).any(|pair| pair == ["do", "not"]);
    if !has_doesnt {
        return Ok(None);
    }

    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| {
            CardTextError::ParseError("missing comma in for each opponent clause".to_string())
        })?;

    let effect_tokens = &tokens[comma_idx + 1..];
    let effects = parse_effect_chain(effect_tokens)?;
    Ok(Some(EffectAst::ForEachOpponentDoesNot { effects }))
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
    let comma_idx = tokens
        .iter()
        .position(|token| matches!(token, Token::Comma(_)))
        .ok_or_else(|| CardTextError::ParseError("missing comma in if clause".to_string()))?;

    let predicate_tokens = &tokens[1..comma_idx];
    let effect_tokens = &tokens[comma_idx + 1..];
    let effects = parse_effect_chain(effect_tokens)?;

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
    if words.len() >= 2 && words[0] == "they" && (words[1] == "dont" || words[1] == "do") {
        if words.len() >= 3 && words[2] == "not" {
            return Some(IfResultPredicate::DidNot);
        }
        if words[1] == "dont" {
            return Some(IfResultPredicate::DidNot);
        }
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

    if filtered.as_slice() == ["no", "spells", "were", "cast", "last", "turn"]
        || filtered.as_slice() == ["no", "spell", "was", "cast", "last", "turn"]
    {
        return Ok(PredicateAst::NoSpellsWereCastLastTurn);
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
    let has_land = filtered.contains(&"land");
    let has_card = filtered.contains(&"card");

    if is_it && has_land && has_card {
        return Ok(PredicateAst::ItIsLandCard);
    }

    if is_it {
        let mut card_types = Vec::new();
        for word in &filtered {
            if let Some(card_type) = parse_card_type(word)
                && !card_types.contains(&card_type)
            {
                card_types.push(card_type);
            }
        }
        if !card_types.is_empty() {
            return Ok(PredicateAst::ItMatches(ObjectFilter {
                card_types,
                ..Default::default()
            }));
        }
    }

    Err(CardTextError::ParseError(format!(
        "unsupported predicate (predicate: '{}')",
        filtered.join(" ")
    )))
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

    if tokens.first().is_some_and(|token| token.is_word("they"))
        && tokens.get(1).is_some_and(|token| token.is_word("may"))
    {
        let inner_tokens = &tokens[2..];
        let effects = parse_effect_chain_inner(inner_tokens)?;
        return Ok(vec![EffectAst::MayByTaggedController {
            tag: TagKey::from("triggering"),
            effects,
        }]);
    }

    if let Some(player) = parse_leading_player_may(tokens) {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_inner(&stripped)?;
        return Ok(vec![EffectAst::MayByPlayer { player, effects }]);
    }

    if tokens.iter().any(|token| token.is_word("may")) && !starts_with_each_opponent {
        let stripped = remove_first_word(tokens, "may");
        let effects = parse_effect_chain_inner(&stripped)?;
        return Ok(vec![EffectAst::May { effects }]);
    }

    parse_effect_chain_inner(tokens)
}

fn parse_effect_chain_inner(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    if let Some(effects) = parse_search_library_sentence(tokens)? {
        return Ok(effects);
    }

    let mut effects = Vec::new();
    let raw_segments = split_on_and(tokens);
    let mut segments: Vec<Vec<Token>> = Vec::new();
    for segment in raw_segments {
        if segment.is_empty() {
            continue;
        }
        if segments.is_empty() {
            segments.push(segment);
            continue;
        }
        if find_verb(&segment).is_none() {
            let last = segments.last_mut().expect("non-empty segments");
            last.push(Token::Word("and".to_string(), TextSpan::synthetic()));
            last.extend(segment);
            continue;
        }
        segments.push(segment);
    }
    while segments.len() > 1 && find_verb(&segments[0]).is_none() {
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
    let mut carried_player: Option<PlayerAst> = None;
    for segment in segments {
        let mut effect = parse_effect_clause(&segment)?;
        if let Some(player) = carried_player {
            maybe_apply_carried_player(&mut effect, player);
        }
        if let Some(player) = explicit_player_for_carry(&effect) {
            carried_player = Some(player);
        }
        effects.push(effect);
    }
    Ok(effects)
}

fn explicit_player_for_carry(effect: &EffectAst) -> Option<PlayerAst> {
    let player = match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. } => *player,
        _ => return None,
    };

    if matches!(player, PlayerAst::Implicit) {
        None
    } else {
        Some(player)
    }
}

fn maybe_apply_carried_player(effect: &mut EffectAst, carried_player: PlayerAst) {
    match effect {
        EffectAst::Draw { player, .. }
        | EffectAst::Discard { player, .. }
        | EffectAst::GainLife { player, .. }
        | EffectAst::LoseLife { player, .. }
        | EffectAst::Scry { player, .. }
        | EffectAst::Surveil { player, .. }
        | EffectAst::Mill { player, .. } => {
            if matches!(*player, PlayerAst::Implicit) {
                *player = carried_player;
            }
        }
        _ => {}
    }
}

fn parse_leading_player_may(tokens: &[Token]) -> Option<PlayerAst> {
    let words = words(tokens);
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
    if words.starts_with(&["defending", "player", "may"]) {
        return Some(PlayerAst::Defending);
    }
    if words.starts_with(&["its", "controller", "may"])
        || words.starts_with(&["their", "controller", "may"])
    {
        return Some(PlayerAst::ItsController);
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
    Remove,
    Return,
    Exchange,
    Become,
    Skip,
    Surveil,
    Pay,
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
            parser: parse_prevent_next_damage_clause,
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
        parse_target_phrase(&left_tokens)?
    };
    let creature2 = parse_target_phrase(&right_tokens)?;

    for target in [&creature1, &creature2] {
        if matches!(target, TargetAst::Player(_, _)) {
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

    let clause_words = words(tokens);
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

    if let Some(effect) = parse_has_base_power_toughness_clause(tokens)? {
        return Ok(effect);
    }

    if let Some((chooser, mut choose_filter)) = parse_target_player_choose_objects_clause(tokens)? {
        if choose_filter.card_types.is_empty() {
            choose_filter.card_types.push(CardType::Creature);
        }
        return Ok(EffectAst::ChooseObjects {
            filter: choose_filter,
            count: ChoiceCount::exactly(1),
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
            "remove",
            "return",
            "exchange",
            "become",
            "skip",
            "surveil",
            "pay",
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

                if subject_words.contains(&"target") {
                    let target = parse_target_phrase(subject_tokens)?;
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
    let subject = parse_subject(subject_tokens);
    let rest = &tokens[verb_idx + 1..];

    parse_effect_with_verb(verb, Some(subject), rest)
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
    if !tail.is_empty() && tail != ["until", "end", "of", "turn"] {
        return Err(CardTextError::ParseError(format!(
            "unsupported trailing base power/toughness clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    let target = parse_target_phrase(subject_tokens)?;
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

fn parse_for_each_opponent_clause(tokens: &[Token]) -> Result<Option<EffectAst>, CardTextError> {
    let words = words(tokens);
    if words.len() < 2 {
        return Ok(None);
    }

    let start = if words.starts_with(&["for", "each", "opponent"])
        || words.starts_with(&["for", "each", "opponents"])
    {
        3
    } else if words.starts_with(&["each", "opponent"]) || words.starts_with(&["each", "opponents"])
    {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&tokens[start..]);
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
    let words = words(tokens);
    if words.len() < 2 {
        return Ok(None);
    }

    let start = if words.starts_with(&["for", "each", "player"])
        || words.starts_with(&["for", "each", "players"])
    {
        3
    } else if words.starts_with(&["each", "player"]) || words.starts_with(&["each", "players"]) {
        2
    } else {
        return Ok(None);
    };

    let inner_tokens = trim_commas(&tokens[start..]);
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
        "pay" => Verb::Pay,
        _ => return Ok(None),
    };

    let effect = parse_effect_with_verb(verb, None, &tokens[1..])?;
    Ok(Some(effect))
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
            "pays" | "pay" => Verb::Pay,
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

    if words.starts_with(&["you"]) {
        return SubjectAst::Player(PlayerAst::You);
    }

    if words.starts_with(&["target", "opponent"]) || words.starts_with(&["target", "opponents"]) {
        return SubjectAst::Player(PlayerAst::TargetOpponent);
    }

    if words.starts_with(&["target", "player"]) || words.starts_with(&["target", "players"]) {
        return SubjectAst::Player(PlayerAst::Target);
    }

    if words.starts_with(&["opponent"])
        || words.starts_with(&["opponents"])
        || words.starts_with(&["an", "opponent"])
    {
        return SubjectAst::Player(PlayerAst::Opponent);
    }

    if words.starts_with(&["defending", "player"]) {
        return SubjectAst::Player(PlayerAst::Defending);
    }

    if words.starts_with(&["that", "player"]) {
        return SubjectAst::Player(PlayerAst::That);
    }

    if words.starts_with(&["its", "controller"]) {
        return SubjectAst::Player(PlayerAst::ItsController);
    }

    if words.starts_with(&["this"]) || words.starts_with(&["thiss"]) {
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
        Verb::Pay => parse_pay(tokens, subject),
    }
}

fn parse_deal_damage(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    if let Some((value, used)) = parse_value(tokens) {
        return parse_deal_damage_with_amount(tokens, value, used);
    }

    let clause_words = words(tokens);
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

    let target_words = words(target_tokens);
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
    let (mut count, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing draw count (clause: '{}')",
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
    let tail = trim_commas(&rest[1..]);
    if !tail.is_empty() {
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
                words(tokens).join(" ")
            )));
        }
    }

    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    Ok(EffectAst::Draw { count, player })
}

fn parse_counter(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
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

        let mut mana = Vec::new();
        for token in &unless_tokens[pays_idx + 1..] {
            if let Some(word) = token.as_word()
                && let Ok(symbol) = parse_mana_symbol(word)
            {
                mana.push(symbol);
            }
        }

        if mana.is_empty() {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                words(tokens).join(" ")
            )));
        }

        return Ok(EffectAst::CounterUnlessPays { target, mana });
    }

    let mut target_tokens = tokens;
    if let Some(that_idx) = tokens.iter().position(|token| token.is_word("that")) {
        target_tokens = &tokens[..that_idx];
    }
    let target = parse_target_phrase(target_tokens)?;
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

fn parse_lose_life(
    tokens: &[Token],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

    let words = words(tokens);
    if words.as_slice() == ["the", "game"] {
        return Ok(EffectAst::LoseGame { player });
    }

    let (amount, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life loss amount (clause: '{}')",
            words.join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "life")
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
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

    let (amount, used) = parse_value(tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing life gain amount (clause: '{}')",
            words(tokens).join(" ")
        ))
    })?;

    let rest = &tokens[used..];
    if rest
        .first()
        .and_then(Token::as_word)
        .is_some_and(|word| word != "life")
    {
        return Err(CardTextError::ParseError(
            "missing life keyword".to_string(),
        ));
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
        let target = parse_target_phrase(&target_tokens)?;
        return Ok(EffectAst::MoveToZone {
            target,
            zone: Zone::Library,
            to_top: true,
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
        "vigilance" => Some(CounterType::Vigilance),
        "loyalty" => Some(CounterType::Loyalty),
        "charge" => Some(CounterType::Charge),
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
            _ => {}
        }
    }
    None
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
    let target = parse_target_phrase(target_tokens)?;
    Ok(EffectAst::PutCounters {
        counter_type,
        count: Value::Fixed(count as i32),
        target,
        target_count: None,
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
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };

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

    let filter_tokens = &tokens[idx..];
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
    let random = rest_words.ends_with(&["at", "random"]);

    Ok(EffectAst::Discard {
        count,
        player,
        random,
    })
}

fn parse_return(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let to_idx = tokens
        .iter()
        .rposition(|token| token.is_word("to"))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing return destination (clause: '{}')",
                words(tokens).join(" ")
            ))
        })?;

    let target_tokens = &tokens[..to_idx];
    let destination_words = words(&tokens[to_idx + 1..]);
    let is_hand = destination_words.contains(&"hand") || destination_words.contains(&"hands");
    let is_battlefield = destination_words.contains(&"battlefield");
    let tapped = destination_words.contains(&"tapped");
    if destination_words.contains(&"transformed") {
        return Err(CardTextError::ParseError(format!(
            "unsupported transformed return clause (clause: '{}')",
            words(tokens).join(" ")
        )));
    }
    let has_delayed_timing = destination_words.contains(&"beginning")
        || destination_words.contains(&"upkeep")
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
    if target_words
        .first()
        .is_some_and(|word| *word == "all" || *word == "each")
    {
        if target_tokens.len() < 2 {
            return Err(CardTextError::ParseError(
                "missing return-all filter".to_string(),
            ));
        }
        let filter = parse_object_filter(&target_tokens[1..], false)?;
        return Ok(EffectAst::ReturnAllToHand { filter });
    }

    let target = parse_target_phrase(target_tokens)?;
    if is_battlefield {
        Ok(EffectAst::ReturnToBattlefield { target, tapped })
    } else {
        Ok(EffectAst::ReturnToHand { target })
    }
}

fn parse_exchange(tokens: &[Token]) -> Result<EffectAst, CardTextError> {
    let words = words(tokens);
    if !words.starts_with(&["control", "of"]) {
        return Err(CardTextError::ParseError(format!(
            "unsupported exchange clause (clause: '{}')",
            words.join(" ")
        )));
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
        return Ok(EffectAst::EnergyCounters {
            count: Value::Fixed(energy_count as i32),
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

fn parse_create(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let player = match subject {
        Some(SubjectAst::Player(player)) => player,
        _ => PlayerAst::Implicit,
    };
    let clause_words = words(tokens);
    let has_dynamic_count = clause_words.first().is_some_and(|word| *word == "x")
        || clause_words.starts_with(&["a", "number", "of"])
        || clause_words.starts_with(&["the", "number", "of"]);
    if has_dynamic_count {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic token count in create clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }
    let mut idx = 0;
    let mut count = 1;
    if let Some((value, used)) = parse_number(tokens) {
        count = value;
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

    let name_words: Vec<&str> = remaining_words[..token_idx]
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    let tail_tokens = &tokens[idx + token_idx + 1..];
    let tail_words = remaining_words[token_idx + 1..].to_vec();
    if name_words.is_empty() {
        if tail_words
            .iter()
            .any(|word| *word == "copy" || *word == "copies")
        {
            let half_pt = tail_words.contains(&"half")
                && tail_words.contains(&"power")
                && tail_words.contains(&"toughness");
            if let Some(of_idx) = tail_tokens.iter().position(|token| token.is_word("of")) {
                let source_tokens = &tail_tokens[of_idx + 1..];
                let source_end = source_tokens
                    .iter()
                    .position(|token| matches!(token, Token::Comma(_)) || token.is_word("except"))
                    .unwrap_or(source_tokens.len());
                let source_tokens = &source_tokens[..source_end];
                if !source_tokens.is_empty() {
                    let source = parse_target_phrase(source_tokens)?;
                    return Ok(EffectAst::CreateTokenCopyFromSource {
                        source,
                        count,
                        player,
                        half_power_toughness_round_up: half_pt,
                        has_haste: false,
                        sacrifice_at_next_end_step: false,
                    });
                }
            }
            return Ok(EffectAst::CreateTokenCopy {
                object: ObjectRefAst::It,
                count,
                player,
                half_power_toughness_round_up: half_pt,
                has_haste: false,
                sacrifice_at_next_end_step: false,
            });
        }
        return Err(CardTextError::ParseError(
            "create clause missing token name".to_string(),
        ));
    }
    let mut name = remaining_words.join(" ");
    if name.is_empty() {
        name = normalize_token_name(&name_words);
        if tail_words.contains(&"lifelink") {
            name.push_str(" lifelink");
        }
    }

    let tapped = tail_words.contains(&"tapped");
    let attacking = tail_words.contains(&"attacking");

    Ok(EffectAst::CreateTokenWithMods {
        name,
        count,
        player,
        tapped,
        attacking,
        exile_at_end_of_combat: false,
    })
}

fn normalize_token_name(words: &[&str]) -> String {
    let mut filtered = Vec::new();
    for word in words {
        if word.contains('/')
            && word
                .chars()
                .all(|c| c.is_ascii_digit() || c == '/' || c == '+' || c == '-')
        {
            continue;
        }
        if matches!(
            *word,
            "white" | "blue" | "black" | "red" | "green" | "colorless" | "creature"
        ) {
            continue;
        }
        filtered.push(*word);
    }
    if filtered.is_empty() {
        words.join(" ")
    } else {
        filtered.join(" ")
    }
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
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
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
    if clause_words.contains(&"blocked") || clause_words.contains(&"blocking") {
        return Err(CardTextError::ParseError(format!(
            "unsupported combat-history destroy clause (clause: '{}')",
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

    let target = parse_target_phrase(tokens)?;
    Ok(EffectAst::Destroy { target })
}

fn parse_exile(tokens: &[Token], subject: Option<SubjectAst>) -> Result<EffectAst, CardTextError> {
    let clause_words = words(tokens);
    if matches!(clause_words.first().copied(), Some("all" | "each")) {
        let filter_tokens = &tokens[1..];
        let filter = parse_object_filter(filter_tokens, false)?;
        return Ok(EffectAst::ExileAll { filter });
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

    let mut target = parse_target_phrase(tokens)?;
    apply_exile_subject_hand_owner_context(&mut target, subject);
    Ok(EffectAst::Exile { target })
}

fn apply_exile_subject_hand_owner_context(target: &mut TargetAst, subject: Option<SubjectAst>) {
    let Some(owner_filter) = exile_subject_owner_filter(subject) else {
        return;
    };
    let Some(filter) = target_object_filter_mut(target) else {
        return;
    };
    if filter.zone != Some(Zone::Hand) {
        return;
    }
    match filter.owner {
        Some(PlayerFilter::Target(_)) | None => {
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
    let mut idx = 0;
    let mut other = false;
    let span = span_from_tokens(tokens);
    let mut target_count: Option<ChoiceCount> = None;

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
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("equipped"), span),
            target_count,
        ));
    }
    if remaining_words.as_slice() == ["enchanted", "creature"]
        || remaining_words.as_slice() == ["enchanted", "creatures"]
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(TagKey::from("enchanted"), span),
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
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("x"))
        && tokens
            .get(idx + 1)
            .is_some_and(|token| token.is_word("target"))
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported dynamic target count 'X target' (clause: '{}')",
            words(tokens).join(" ")
        )));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("on")) {
        idx += 1;
    }

    if tokens
        .get(idx)
        .is_some_and(|token| token.is_word("another"))
    {
        other = true;
        idx += 1;
    }

    let words_all = words(&tokens[idx..]);
    if words_all.as_slice() == ["any", "target"] {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    if tokens.get(idx).is_some_and(|token| token.is_word("target")) {
        idx += 1;
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = words(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();

    if remaining_words.as_slice() == ["player"] || remaining_words.as_slice() == ["players"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Any, span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["you"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["opponent"] || remaining_words.as_slice() == ["opponents"] {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Opponent, span),
            target_count,
        ));
    }

    if remaining_words.as_slice() == ["spell"] || remaining_words.as_slice() == ["spells"] {
        return Ok(wrap_target_count(TargetAst::Spell(span), target_count));
    }

    if is_source_reference_words(&remaining_words) {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if is_source_from_your_graveyard_words(&remaining_words) {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if remaining_words.starts_with(&["thiss", "power", "and", "toughness"]) {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
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

    let has_creature =
        remaining_words.contains(&"creature") || remaining_words.contains(&"creatures");
    let has_player = remaining_words.contains(&"player") || remaining_words.contains(&"players");
    if has_creature && has_player {
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
        TargetAst::Object(filter, span, it_span),
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
            let mut target_filter_tokens = target_tokens;
            if target_filter_tokens
                .first()
                .is_some_and(|token| token.is_word("target"))
            {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if !target_filter_tokens.is_empty() {
                target_object = Some(parse_object_filter(target_filter_tokens, false)?);
            }
        }
    }

    let mut all_words: Vec<&str> = words(&base_tokens)
        .into_iter()
        .filter(|word| !is_article(word) && *word != "instead")
        .collect();

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

    if has_share_card_type {
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: IT_TAG.into(),
            relation: TaggedOpbjectRelation::SharesCardType,
        });
    }

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
                    filter.controller = Some(PlayerFilter::target_player());
                }
                ["they", "own"] | ["they", "owns"] => {
                    filter.owner = Some(PlayerFilter::target_player());
                }
                _ => {}
            }
        }
    }
    if all_words.len() >= 3 {
        for window in all_words.windows(3) {
            match window {
                ["that", "player", "control"] | ["that", "player", "controls"] => {
                    filter.controller = Some(PlayerFilter::IteratedPlayer);
                }
                ["that", "player", "own"] | ["that", "player", "owns"] => {
                    filter.owner = Some(PlayerFilter::IteratedPlayer);
                }
                ["you", "dont", "control"] => {
                    filter.controller = Some(PlayerFilter::Opponent);
                }
                ["you", "dont", "own"] => {
                    filter.owner = Some(PlayerFilter::Opponent);
                }
                _ => {}
            }
        }
    }
    if all_words.len() >= 4 {
        for window in all_words.windows(4) {
            if window == ["you", "do", "not", "control"] {
                filter.controller = Some(PlayerFilter::Opponent);
            } else if window == ["you", "do", "not", "own"] {
                filter.owner = Some(PlayerFilter::Opponent);
            }
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if all_words[with_idx] != "with" {
            with_idx += 1;
            continue;
        }

        if let Some((kind, consumed)) = parse_alternative_cast_words(&all_words[with_idx + 1..]) {
            filter.alternative_cast = Some(kind);
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

        without_idx += 1;
    }

    for idx in 0..all_words.len() {
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            let is_reference_zone_for_spell = if all_words.contains(&"spell") {
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
                        filter.owner = Some(PlayerFilter::target_player());
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
            "power" => filter.power = Some(cmp),
            "toughness" => filter.toughness = Some(cmp),
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
    for idx in 0..all_words.len().saturating_sub(1) {
        if all_words[idx] != "non" {
            continue;
        }
        let next = all_words[idx + 1];
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

    for (idx, word) in all_words.iter().enumerate() {
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => saw_spell = true,
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
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
            _ => {}
        }

        if negated_word_indices.contains(&idx) {
            continue;
        }

        if let Some(card_type) = parse_non_type(word) {
            filter.excluded_card_types.push(card_type);
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

        if let Some(card_type) = parse_card_type(word)
            && is_permanent_type(card_type)
        {
            saw_permanent_type = true;
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

    for segment in &segments {
        let segment_words: Vec<&str> = words(segment)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        let mut types = Vec::new();
        for word in segment_words {
            if let Some(card_type) = parse_card_type(word)
                && !types.contains(&card_type)
            {
                types.push(card_type);
            }
        }
        if !types.is_empty() {
            segment_types.push(types);
        }
    }

    if segments.len() > 1 {
        let mut any_types = Vec::new();
        for types in segment_types {
            if types.len() != 1 {
                return Err(CardTextError::ParseError(
                    "unsupported target type list".to_string(),
                ));
            }
            let card_type = types[0];
            if !any_types.contains(&card_type) {
                any_types.push(card_type);
            }
        }
        if !any_types.is_empty() {
            filter.card_types = any_types;
        }
    } else if let Some(types) = segment_types.into_iter().next() {
        if types.len() > 1 {
            filter.all_card_types = types;
        } else if types.len() == 1 {
            filter.card_types = types;
        }
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
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.name.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.custom_static_markers.is_empty()
        || !filter.excluded_custom_static_markers.is_empty()
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
        || filter.power.is_some()
        || filter.toughness.is_some()
        || filter.mana_value.is_some()
        || filter.name.is_some()
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

    for word in words {
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
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }
    }

    filter
}

fn split_on_or(tokens: &[Token]) -> Vec<Vec<Token>> {
    let mut segments = Vec::new();
    let mut current = Vec::new();

    for token in tokens {
        if matches!(token, Token::Comma(_)) || token.is_word("or") {
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
        "exile" => Some(Zone::Exile),
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
        KeywordAction::Changeling => Static(StaticAbilityId::Changeling),
        KeywordAction::Marker(marker)
            if matches!(
                marker,
                "islandwalk" | "swampwalk" | "mountainwalk" | "forestwalk" | "plainswalk"
            ) =>
        {
            Marker(marker)
        }
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
