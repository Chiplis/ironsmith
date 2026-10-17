//! "That player exiles it. If the player does, they may cast a spell from
//! among other cards exiled with this artifact without paying its mana
//! cost." (Knowledge Pool): the exile adds the object to this source's linked
//! pool (CR 607.2a), and the follow-up cast draws from that pool excluding
//! the object just exiled (CR 608.2g; the cast follows CR 601.2). The exiled
//! object is tagged from the exile's own outcome, so "other" names exactly
//! the card that instruction put into exile.
use crate::cards::builders::{
    CardTextError, ConditionalEffectAst, EffectAst, ForEachEffectAst, IfResultPredicate,
    ObjectChoiceEffectAst, OwnedLexToken, PermissionEffectAst, PlayerAst,
};
use crate::effect::ChoiceCount;
use crate::lexer::parser_token_word_refs;
use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::zone::Zone;

/// "That player exiles it." / "They exile it." / "You exile it." / "Exile
/// it.": the exiling actor.
fn exiling_actor(words: &[&str]) -> Option<PlayerAst> {
    match words {
        ["that", "player", "exiles", "it"] | ["they", "exile", "it"] => Some(PlayerAst::That),
        ["you", "exile", "it"] | ["exile", "it"] => Some(PlayerAst::You),
        _ => None,
    }
}

/// "If the player does, they may cast a spell from among other cards exiled
/// with this artifact without paying its mana cost.": the casting actor and
/// whether the cast is free.
fn other_pool_cast(words: &[&str]) -> Option<(PlayerAst, bool)> {
    let rest = words
        .strip_prefix(&["if", "the", "player", "does"][..])
        .or_else(|| words.strip_prefix(&["if", "that", "player", "does"][..]))
        .or_else(|| words.strip_prefix(&["if", "they", "do"][..]))
        .or_else(|| words.strip_prefix(&["if", "you", "do"][..]))?;
    let (actor, rest) = if let Some(rest) = rest
        .strip_prefix(&["they", "may"][..])
        .or_else(|| rest.strip_prefix(&["that", "player", "may"][..]))
        .or_else(|| rest.strip_prefix(&["the", "player", "may"][..]))
    {
        (PlayerAst::That, rest)
    } else {
        (PlayerAst::You, rest.strip_prefix(&["you", "may"][..])?)
    };
    let rest = rest.strip_prefix(
        &[
            "cast", "a", "spell", "from", "among", "other", "cards", "exiled", "with",
        ][..],
    )?;
    let (source, free) = if let Some(source) =
        rest.strip_suffix(&["without", "paying", "its", "mana", "cost"][..])
    {
        (source, true)
    } else {
        (rest, false)
    };
    crate::util::is_source_reference_words(source).then_some((actor, free))
}

pub(crate) fn read(sentences: &[&[OwnedLexToken]]) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let &[exile, cast] = sentences else {
        return Ok(None);
    };
    let exile = crate::util::trim_edge_punctuation_tokens(exile);
    if exiling_actor(&parser_token_word_refs(exile)).is_none() {
        return Ok(None);
    }
    let Some((actor, free)) =
        other_pool_cast(&parser_token_word_refs(crate::util::trim_edge_punctuation_tokens(cast)))
    else {
        return Ok(None);
    };
    let mut exile_effects = super::parse_effect_chain(exile)?;
    let [exile_effect] = exile_effects.as_mut_slice() else {
        return Ok(None);
    };
    let pooled = crate::util::helper_tag_for_tokens(exile, "pool_exiled");
    let chosen = crate::util::helper_tag_for_tokens(cast, "cast_from_pool");

    // A nonland card in this source's linked pool, other than the one just
    // exiled (lands can't be cast, CR 305.9).
    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.excluded_card_types.push(CardType::Land);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: crate::tag::CompilerReferenceTag::SourceExiled.bind().into(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: crate::tag::TagRef::of(pooled.clone()).into(),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    let cast_one = vec![
        EffectAst::ObjectChoices(ObjectChoiceEffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: actor,
            tag: crate::tag::TagRef::of(chosen.clone()),
        }),
        EffectAst::ForEach(ForEachEffectAst::ForEachTagged {
            tag: crate::tag::TagRef::of(chosen),
            effects: vec![
                EffectAst::subject_verb_cast_tagged_with_additional_cost_and_mana_spend_mode(
                    crate::tag::CompilerReferenceTag::It.bind(),
                    actor,
                    false,
                    false,
                    free,
                    None,
                    None,
                    ironsmith_core::value_model::ManaSpendMode::Normal,
                ),
            ],
        }),
    ];
    Ok(Some(vec![
        EffectAst::TagAffected {
            effect: Box::new(exile_effect.clone()),
            tag: crate::tag::TagRef::of(pooled),
        },
        EffectAst::Conditionals(ConditionalEffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![EffectAst::Permissions(PermissionEffectAst::MayByPlayer {
                player: actor,
                effects: cast_one,
            })],
        }),
    ]))
}
