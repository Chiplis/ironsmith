//! "You may play a land from the top of your library or cast a spell with
//! mana value less than or equal to the damage dealt from the top of your
//! library without paying its mana cost." (Ziatora's Envoy): after looking at
//! the top card, one optional play of that card, admitted when it is a land
//! (a land play, CR 305.1) or a nonland card matching the spell qualifier
//! (a cast, CR 601.2; the free price belongs to the spell). It is the same
//! instruction as "If it's a land card or a nonland card <qualifier>, you may
//! play it without paying its mana cost", so it is read through that complete
//! sentence grammar (keeping the authored spans for result tags).
use crate::cards::builders::{CardTextError, EffectAst};
use crate::lexer::OwnedLexToken;

const TOP: &[&str] = &["from", "the", "top", "of", "your", "library"];

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let words = crate::lexer::token_word_refs(tokens);
    let Some(rest) = words.strip_prefix(&["you", "may", "play", "a", "land"][..]) else {
        return Ok(None);
    };
    let Some(rest) = rest.strip_prefix(TOP) else { return Ok(None) };
    let Some(rest) = rest.strip_prefix(&["or", "cast", "a", "spell"][..]) else {
        return Ok(None);
    };
    let (qualifier, free) = if let Some(head) = rest.strip_suffix(&["without", "paying", "its", "mana", "cost"][..]) {
        (head, true)
    } else {
        (rest, false)
    };
    let Some(qualifier) = qualifier.strip_suffix(TOP) else { return Ok(None) };
    // The qualifier is an ordinary card-filter tail ("with mana value ...").
    if qualifier.first().is_some_and(|word| *word != "with") || qualifier.contains(&"from") {
        return Ok(None);
    }
    let mut text = String::from("if it's a land card or a nonland card");
    for word in qualifier {
        text.push(' ');
        text.push_str(word);
    }
    text.push_str(", you may play it");
    if free {
        text.push_str(" without paying its mana cost");
    }
    text.push('.');
    // Keep the authored offsets so helper tags stay unique within the line.
    let (line, start) = tokens
        .first()
        .map(|token| (token.span.line, token.span.start))
        .unwrap_or((0, 0));
    let padded = format!("{}{}", " ".repeat(start), text);
    let Ok(rewritten) = crate::lexer::lex_line(&padded, line) else {
        return Ok(None);
    };
    let effects = crate::clause_support::parse_effect_sentences_lexed(&rewritten)?;
    Ok((!effects.is_empty()).then_some(effects))
}
