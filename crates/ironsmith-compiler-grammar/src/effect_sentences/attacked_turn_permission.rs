//! "During any turn you attacked with <creatures>, you may play that card."
//! (Goblin Researcher, Boros Strike-Captain, Robber of the Rich): a lasting
//! permission over the exiled card that is usable only during turns in which
//! you attacked with enough matching creatures (CR 508.1: a creature has
//! attacked once it was declared as an attacker that turn). The duration
//! clause and the permission are one sentence; split at the comma, neither
//! half is an instruction of its own.
use crate::cards::builders::{CardTextError, EffectAst, PlayerAst};
use crate::grammar::{leaf, primitives};
use crate::lexer::{OwnedLexToken, TokenKind, token_word_refs, trim_lexed_commas};
use crate::target::ObjectFilter;
use winnow::Parser;
use winnow::combinator::alt;

/// Split the attacker description into its minimum and filter tokens:
/// "three or more creatures" -> (3, "creatures"); "a Rogue" -> (1, "Rogue").
fn attacker_count(tokens: &[OwnedLexToken]) -> (u32, &[OwnedLexToken]) {
    if let Some((minimum, rest)) = primitives::parse_prefix(
        tokens,
        (
            leaf::parse_leaf_number_token_lexed,
            primitives::phrase(&["or", "more"]),
        )
            .map(|(minimum, ())| minimum),
    ) {
        return (minimum, rest);
    }
    if let Some((_, rest)) =
        primitives::parse_prefix(tokens, alt((primitives::kw("a"), primitives::kw("an"))))
    {
        return (1, rest);
    }
    (1, tokens)
}

pub(super) fn parse(tokens: &[OwnedLexToken]) -> Result<Option<EffectAst>, CardTextError> {
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some(((), rest)) = primitives::parse_prefix(
        tokens,
        primitives::phrase(&["during", "any", "turn", "you", "attacked", "with"]),
    ) else {
        return Ok(None);
    };
    let Some(comma) = rest
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Comma))
    else {
        return Ok(None);
    };
    let (minimum, attacker_tokens) = attacker_count(trim_lexed_commas(&rest[..comma]));
    if attacker_tokens.is_empty() {
        return Ok(None);
    }
    let attacker_words = token_word_refs(attacker_tokens);
    let filter = if crate::util::is_source_reference_words(&attacker_words) {
        // "you attacked with this creature": this exact object (CR 400.7).
        if minimum != 1 {
            return Ok(None);
        }
        ObjectFilter::source()
    } else {
        let mut filter = crate::object_filters::parse_object_filter(attacker_tokens, false)?;
        filter.zone = None;
        filter
    };

    let mut permission = trim_lexed_commas(&rest[comma + 1..]);
    let mut mana_spend_mode = ironsmith_core::value_model::ManaSpendMode::Normal;
    if let Some(fact) =
        crate::grammar::permission_facts::tagged_surface::parse_allow_any_color_for_cast_suffix_tokens(
            permission,
        )
    {
        mana_spend_mode = fact.mana_spend_mode;
        permission = trim_lexed_commas(fact.body_tokens);
    }
    let Some(allow_land) = primitives::probe_all(
        permission,
        (
            primitives::phrase(&["you", "may"]),
            alt((
                primitives::kw("play").value(true),
                primitives::kw("cast").value(false),
            )),
            primitives::any_phrase(&[&["that", "card"], &["those", "cards"], &["it"], &["them"]]),
            primitives::sentence_end(),
        )
            .map(|((), allow_land, _, ())| allow_land),
        "attacked-turn play permission",
    ) else {
        return Ok(None);
    };
    Ok(Some(
        EffectAst::subject_verb_grant_play_tagged_during_turns_attacked_with(
            crate::tag::CompilerReferenceTag::It.bind(),
            PlayerAst::You,
            allow_land,
            mana_spend_mode,
            ironsmith_core::effect::AttackedWithTurnCondition { filter, minimum },
        ),
    ))
}
