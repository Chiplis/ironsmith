use super::*;

/// Return whether the source lexically contains an authored `, then`
/// connective outside quoted rules text.
///
/// This is intentionally broader than [`has_explicit_comma_then_boundary_tokens`].
/// The latter answers whether the generic chain splitter can safely separate
/// the clauses before parsing, so it rejects some pronoun-bearing tails. Once
/// a specialist has already produced multiple typed effects, those safety
/// heuristics must not erase the connective's presentation provenance.
pub fn has_authored_comma_then_surface_tokens(tokens: &[OwnedLexToken]) -> bool {
    let mut inside_quotes = false;
    for (idx, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Quote {
            inside_quotes = !inside_quotes;
            continue;
        }
        if !inside_quotes
            && token.kind == TokenKind::Comma
            && tokens
                .get(idx + 1)
                .is_some_and(|next| is_word(next, "then"))
        {
            return true;
        }
    }
    false
}

pub fn split_segments_on_comma_effect_head_tokens(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    let mut result = Vec::new();
    for segment in segments {
        let mut start = 0usize;
        let mut split_any = false;
        let mut input = LexStream::new(segment);
        let mut inside_quotes = false;
        while !input.is_empty() {
            let idx = segment.len().saturating_sub(input.len());
            let parsed: WResult<&OwnedLexToken> = any.parse_next(&mut input);
            let Ok(token) = parsed else {
                break;
            };
            if token.kind == TokenKind::Quote {
                inside_quotes = !inside_quotes;
                continue;
            }
            if inside_quotes || token.kind != TokenKind::Comma {
                continue;
            }
            let before = trim_lexed_commas(segment.get(start..idx).unwrap_or_default());
            let after = trim_lexed_commas(segment.get(idx + 1..).unwrap_or_default());
            if before.is_empty() || after.is_empty() {
                continue;
            }
            let facts = comma_boundary_facts(before, after);
            if facts.preserve_boundary {
                continue;
            }
            if facts.before_has_verb && facts.after_starts_effect {
                if std::env::var_os("IRONSMITH_CHOICE_TRACE").is_some() {
                    eprintln!(
                        "comma-effect-head split: before='{}' after='{}'",
                        crate::lexer::token_word_refs(before).join(" "),
                        crate::lexer::token_word_refs(after).join(" ")
                    );
                }
                result.push(before);
                start = idx + 1;
                split_any = true;
            }
        }
        if split_any {
            let tail = trim_lexed_commas(segment.get(start..).unwrap_or_default());
            if !tail.is_empty() {
                result.push(tail);
            }
        } else {
            result.push(segment);
        }
    }
    result
}
