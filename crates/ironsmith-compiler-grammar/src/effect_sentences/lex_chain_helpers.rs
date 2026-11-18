use super::super::clause_support::parse_ability_line_lexed;
use super::super::grammar::effects::chain_splitting as chain_grammar;
use super::super::lexer::OwnedLexToken;
use super::chain_carry::Verb;

pub fn strip_leading_instead_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    chain_grammar::strip_leading_instead_tokens(tokens).map(<[_]>::to_vec)
}

pub fn strip_leading_instead_prefix_lexed(tokens: &[OwnedLexToken]) -> Option<&[OwnedLexToken]> {
    chain_grammar::strip_leading_instead_tokens(tokens)
}

pub fn starts_with_inline_token_rules_tail(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::starts_with_inline_token_rules_tail_tokens(tokens)
}

pub fn is_token_creation_context(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::is_token_creation_context_tokens(tokens)
}

pub fn find_verb_lexed(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    let found = chain_grammar::find_chain_verb_tokens(tokens)?;
    Some((found.kind, found.word_index))
}

/// "Add two mana in any combination of {U}, {B}, and/or {R}." (Relic of
/// Sauron): the conjunctions after "any combination of" list mana options and
/// never coordinate actions. A later "and <effect verb>" ("add three mana in
/// any combination of its colors and put it into your hand", Omnath, Locus of
/// All) is still a real chain boundary, so the list only shields the clause
/// when no such boundary follows it.
pub fn lists_mana_combination_without_chain_boundary(tokens: &[OwnedLexToken]) -> bool {
    let Some(start) = tokens.windows(3).position(|window| {
        window[0].is_word("any") && window[1].is_word("combination") && window[2].is_word("of")
    }) else {
        return false;
    };
    !tokens
        .iter()
        .enumerate()
        .skip(start + 3)
        .any(|(index, token)| {
            token.is_word("and")
                && find_verb_lexed(&tokens[index + 1..]).is_some_and(|(_, word_index)| word_index == 0)
        })
}

pub fn split_effect_chain_on_and_lexed(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    chain_grammar::split_effect_chain_on_and_tokens(tokens, true)
}

/// "put a +1/+1 counter on that creature and each other creature you control
/// that shares a creature type with it" (Holy Frazzle-Cannon): a verbless
/// "each ..." arm after a counter placement's "on <recipient>" is a second
/// recipient of that placement, not a new quantified clause.
pub fn is_counter_placement_recipient_arm_lexed(
    previous: &[OwnedLexToken],
    segment: &[OwnedLexToken],
) -> bool {
    previous.first().is_some_and(|token| token.is_any_word(&["put", "puts"]))
        && previous
            .iter()
            .position(|token| token.is_any_word(&["counter", "counters"]))
            .is_some_and(|counter| previous[counter + 1..].iter().any(|token| token.is_word("on")))
        && segment.first().is_some_and(|token| token.is_word("each"))
        && !segment.get(1).is_some_and(|token| token.is_word("of"))
        && find_verb_lexed(segment).is_none()
}

pub fn has_effect_head_without_verb_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::has_extended_effect_head_tokens(tokens)
}

pub fn segment_has_effect_head_lexed(tokens: &[OwnedLexToken]) -> bool {
    // "that creature fights up to one target creature" (CR 701.14a).
    let fights = crate::lexer::parser_token_word_refs(tokens)
        .iter()
        .take(4)
        .skip(1)
        .any(|word| matches!(*word, "fights" | "fight"));
    fights
        || find_verb_lexed(tokens).is_some()
        || has_effect_head_without_verb_lexed(tokens)
        || super::super::grammar::effects::chain_carry::parse_carry_duration_prefix_tokens(tokens)
            .is_some_and(|shape| segment_has_effect_head_lexed(shape.rest))
        || chain_grammar::starts_with_player_may_tokens(tokens)
        // `copy` is a real effect head, but it is intentionally kept out of
        // the generic chain-verb vocabulary because copy parsing has several
        // specialized target/retarget shapes. Still recognize it here so a
        // following coordinated clause is not merged into the copy parser's
        // prefix-only result.
        || super::super::grammar::effects::clause_pattern_shapes::parse_copy_clause_shape_tokens(
            tokens,
        )
        .is_some_and(|shape| shape.copy_word == 0)
}

pub fn split_segments_on_comma_then_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    chain_grammar::split_segments_on_comma_then_tokens(segments, |tokens| {
        parse_ability_line_lexed(tokens).is_some()
    })
}

pub fn has_explicit_comma_then_boundary_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::has_explicit_comma_then_boundary_tokens(tokens, |tokens| {
        parse_ability_line_lexed(tokens).is_some()
    })
}

pub fn has_authored_comma_then_surface_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::has_authored_comma_then_surface_tokens(tokens)
}

pub fn split_segments_on_comma_effect_head_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    chain_grammar::split_segments_on_comma_effect_head_tokens(segments)
}
