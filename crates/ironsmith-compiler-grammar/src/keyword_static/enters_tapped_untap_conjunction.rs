//! "This creature enters tapped and doesn't untap during your untap step."
//! (Leviathan): two independent source statics sharing one subject — an
//! entry replacement (CR 614.1c) and an untap-step restriction (CR 502.3).
//! Each half is read by its established single-clause owner; the line only
//! succeeds when both halves are claimed.
use super::*;

pub(super) fn parse_enters_tapped_and_doesnt_untap_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<StaticAbilityAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    if tokens.iter().any(OwnedLexToken::is_quote) {
        return Ok(None);
    }
    let view = TokenWordView::new(tokens);
    let words = view.word_refs();
    let starts = view.token_start_indices();
    let Some(enters) = words.iter().position(|word| *word == "enters") else {
        return Ok(None);
    };
    // Only a source subject: "Lands you control enter tapped and ..." is a
    // different (filtered) family.
    if enters == 0 || !crate::util::is_source_reference_words(&words[..enters]) {
        return Ok(None);
    }
    let negated_untap = |word: &str| matches!(word, "doesn't" | "doesnt" | "doesn’t");
    if words.get(enters + 1) != Some(&"tapped")
        || words.get(enters + 2) != Some(&"and")
        || !words.get(enters + 3).is_some_and(|word| negated_untap(word))
        || words.get(enters + 4) != Some(&"untap")
    {
        return Ok(None);
    }
    let (Some(&enters_token), Some(&and_token)) = (starts.get(enters), starts.get(enters + 2))
    else {
        return Ok(None);
    };
    let subject = &tokens[..enters_token];

    let entry_line = tokens[..and_token].to_vec();
    let mut untap_line = subject.to_vec();
    untap_line.extend_from_slice(&tokens[and_token + 1..]);

    let Some(mut abilities) = parse_static_ability_ast_line_lexed(&entry_line)? else {
        return Ok(None);
    };
    let Some(untap) = parse_static_ability_ast_line_lexed(&untap_line)? else {
        return Ok(None);
    };
    abilities.extend(untap);
    Ok(Some(abilities))
}
