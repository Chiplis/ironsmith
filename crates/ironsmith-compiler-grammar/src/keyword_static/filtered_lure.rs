//! "All creatures with flying able to block this creature do so." (Talruum
//! Piper) and "All Walls able to block this creature do so." (Marble
//! Priest): the lure requirement (CR 509.1c) restricted to a filtered set of
//! potential blockers. The unfiltered "All creatures ..." line keeps its own
//! rule.

use super::*;

const LURE_TAIL: &[&str] = &["able", "to", "block", "this", "creature", "do", "so"];

pub fn parse_filtered_creatures_able_to_block_source_line(
    tokens: &[OwnedLexToken],
) -> Result<Option<StaticAbility>, CardTextError> {
    let tokens = trim_edge_punctuation(tokens);
    if !tokens.first().is_some_and(|token| token.is_word("all")) {
        return Ok(None);
    }
    let Some(able_idx) = tokens.iter().position(|token| token.is_word("able")) else {
        return Ok(None);
    };
    if able_idx < 2
        || !crate::word_primitives::parse_sequence_complete(
            &parser_token_word_refs(&tokens[able_idx..]),
            LURE_TAIL,
        )
    {
        return Ok(None);
    }
    let blocker_tokens = &tokens[1..able_idx];
    if crate::word_primitives::parse_sequence_complete(
        &parser_token_word_refs(blocker_tokens),
        &["creatures"],
    ) {
        // The unfiltered lure belongs to parse_all_creatures_able_to_block_source_line.
        return Ok(None);
    }
    let mut blockers = parse_object_filter_lexed(blocker_tokens, false)?;
    if blockers.card_types.is_empty() && blockers.subtypes.is_empty() {
        return Ok(None);
    }
    if !blockers.card_types.contains(&CardType::Creature) {
        blockers.card_types.push(CardType::Creature);
    }
    let display = format!(
        "All {} able to block this creature do so",
        crate::lexer::render_token_slice(blocker_tokens).trim()
    );
    Ok(Some(StaticAbility::restriction(
        crate::effect::Restriction::must_block_specific_attacker(
            blockers,
            ObjectFilter::source(),
        ),
        display,
    )))
}
