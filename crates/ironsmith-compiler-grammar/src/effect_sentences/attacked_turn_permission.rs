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
    let attacker_filter = if crate::util::is_source_reference_words(&attacker_words) {
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
    // The permission body is the shared tagged play/cast reader (the same
    // one every exile permission uses: target pools, "without paying its mana
    // cost", owner narrowing). The turn condition replaces its lifetime, so
    // the body must not author one of its own.
    let Some(crate::permission_helpers::PermissionClauseSpec::Tagged {
        tag,
        player,
        allow_land,
        as_copy: false,
        max_plays: None,
        without_paying_mana_cost,
        lifetime:
            crate::permission_helpers::PermissionLifetime::Immediate
            | crate::permission_helpers::PermissionLifetime::ForAsLongAsExiled,
        filter,
        surface,
    }) = crate::permission_helpers::parse_permission_clause_spec(permission)?
    else {
        return Ok(None);
    };
    if !matches!(player, PlayerAst::Implicit | PlayerAst::You) {
        return Ok(None);
    }
    let mut effect = EffectAst::subject_verb_grant_play_tagged_during_turns_attacked_with(
        crate::tag::TagRef::of(tag),
        PlayerAst::You,
        allow_land,
        mana_spend_mode,
        ironsmith_core::effect::AttackedWithTurnCondition { filter: attacker_filter, minimum },
    );
    if let EffectAst::SubjectVerb(subject) = &mut effect
        && let crate::cards::builders::SubjectVerbActionAst::Grants(
            crate::cards::builders::GrantActionAst::GrantPlayTaggedForAsLongAsExiled {
                without_paying_mana_cost: free,
                filter: pool_filter,
                surface: grant_surface,
                ..
            },
        ) = &mut subject.action
    {
        *free = without_paying_mana_cost;
        *pool_filter = filter;
        *grant_surface = surface;
    }
    Ok(Some(effect))
}
