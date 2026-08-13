use super::super::clause_support::parse_ability_line_lexed;
use super::super::grammar::effects::chain_splitting as chain_grammar;
use super::super::lexer::OwnedLexToken;
use super::chain_carry::Verb;

pub(crate) fn strip_leading_instead_prefix(tokens: &[OwnedLexToken]) -> Option<Vec<OwnedLexToken>> {
    chain_grammar::strip_leading_instead_tokens(tokens).map(<[_]>::to_vec)
}

pub(crate) fn strip_leading_instead_prefix_lexed(
    tokens: &[OwnedLexToken],
) -> Option<&[OwnedLexToken]> {
    chain_grammar::strip_leading_instead_tokens(tokens)
}

pub(crate) fn starts_with_inline_token_rules_tail(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::starts_with_inline_token_rules_tail_tokens(tokens)
}

pub(crate) fn is_token_creation_context(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::is_token_creation_context_tokens(tokens)
}

pub(crate) fn find_verb_lexed(tokens: &[OwnedLexToken]) -> Option<(Verb, usize)> {
    let found = chain_grammar::find_chain_verb_tokens(tokens)?;
    Some((lower_chain_verb(found.kind), found.word_index))
}

pub(crate) fn split_effect_chain_on_and_lexed(tokens: &[OwnedLexToken]) -> Vec<&[OwnedLexToken]> {
    chain_grammar::split_effect_chain_on_and_tokens(tokens, true)
}

pub(crate) fn has_effect_head_without_verb_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::has_extended_effect_head_tokens(tokens)
}

pub(crate) fn segment_has_effect_head_lexed(tokens: &[OwnedLexToken]) -> bool {
    find_verb_lexed(tokens).is_some()
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

pub(crate) fn split_segments_on_comma_then_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    chain_grammar::split_segments_on_comma_then_tokens(segments, |tokens| {
        parse_ability_line_lexed(tokens).is_some()
    })
}

pub(crate) fn has_explicit_comma_then_boundary_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::has_explicit_comma_then_boundary_tokens(tokens, |tokens| {
        parse_ability_line_lexed(tokens).is_some()
    })
}

pub(crate) fn has_authored_comma_then_surface_lexed(tokens: &[OwnedLexToken]) -> bool {
    chain_grammar::has_authored_comma_then_surface_tokens(tokens)
}

pub(crate) fn split_segments_on_comma_effect_head_lexed(
    segments: Vec<&[OwnedLexToken]>,
) -> Vec<&[OwnedLexToken]> {
    chain_grammar::split_segments_on_comma_effect_head_tokens(segments)
}

fn lower_chain_verb(kind: chain_grammar::ChainVerbKind) -> Verb {
    match kind {
        chain_grammar::ChainVerbKind::Add => Verb::Add,
        chain_grammar::ChainVerbKind::Move => Verb::Move,
        chain_grammar::ChainVerbKind::Deal => Verb::Deal,
        chain_grammar::ChainVerbKind::Draw => Verb::Draw,
        chain_grammar::ChainVerbKind::Counter => Verb::Counter,
        chain_grammar::ChainVerbKind::Destroy => Verb::Destroy,
        chain_grammar::ChainVerbKind::Exile => Verb::Exile,
        chain_grammar::ChainVerbKind::Reveal => Verb::Reveal,
        chain_grammar::ChainVerbKind::Look => Verb::Look,
        chain_grammar::ChainVerbKind::Lose => Verb::Lose,
        chain_grammar::ChainVerbKind::Gain => Verb::Gain,
        chain_grammar::ChainVerbKind::Put => Verb::Put,
        chain_grammar::ChainVerbKind::Sacrifice => Verb::Sacrifice,
        chain_grammar::ChainVerbKind::Create => Verb::Create,
        chain_grammar::ChainVerbKind::Investigate => Verb::Investigate,
        chain_grammar::ChainVerbKind::Proliferate => Verb::Proliferate,
        chain_grammar::ChainVerbKind::Tap => Verb::Tap,
        chain_grammar::ChainVerbKind::Unattach => Verb::Unattach,
        chain_grammar::ChainVerbKind::Attach => Verb::Attach,
        chain_grammar::ChainVerbKind::Untap => Verb::Untap,
        chain_grammar::ChainVerbKind::Unlock => Verb::Unlock,
        chain_grammar::ChainVerbKind::Scry => Verb::Scry,
        chain_grammar::ChainVerbKind::Discard => Verb::Discard,
        chain_grammar::ChainVerbKind::Transform => Verb::Transform,
        chain_grammar::ChainVerbKind::Convert => Verb::Convert,
        chain_grammar::ChainVerbKind::Flip => Verb::Flip,
        chain_grammar::ChainVerbKind::Roll => Verb::Roll,
        chain_grammar::ChainVerbKind::Regenerate => Verb::Regenerate,
        chain_grammar::ChainVerbKind::Heal => Verb::Heal,
        chain_grammar::ChainVerbKind::Mill => Verb::Mill,
        chain_grammar::ChainVerbKind::Get => Verb::Get,
        chain_grammar::ChainVerbKind::Remove => Verb::Remove,
        chain_grammar::ChainVerbKind::Return => Verb::Return,
        chain_grammar::ChainVerbKind::Exchange => Verb::Exchange,
        chain_grammar::ChainVerbKind::Become => Verb::Become,
        chain_grammar::ChainVerbKind::Switch => Verb::Switch,
        chain_grammar::ChainVerbKind::Skip => Verb::Skip,
        chain_grammar::ChainVerbKind::Surveil => Verb::Surveil,
        chain_grammar::ChainVerbKind::Incubate => Verb::Incubate,
        chain_grammar::ChainVerbKind::Shuffle => Verb::Shuffle,
        chain_grammar::ChainVerbKind::Reorder => Verb::Reorder,
        chain_grammar::ChainVerbKind::Reverse => Verb::Reverse,
        chain_grammar::ChainVerbKind::Pay => Verb::Pay,
        chain_grammar::ChainVerbKind::Take => Verb::Take,
        chain_grammar::ChainVerbKind::Detain => Verb::Detain,
        chain_grammar::ChainVerbKind::Assign => Verb::Assign,
        chain_grammar::ChainVerbKind::Goad => Verb::Goad,
        chain_grammar::ChainVerbKind::Suspect => Verb::Suspect,
        chain_grammar::ChainVerbKind::End => Verb::End,
    }
}
