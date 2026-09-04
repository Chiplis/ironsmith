use super::super::SentenceInput;
use crate::activation_and_restrictions::parse_may_cast_it_sentence;
use crate::cards::builders::{
    CardTextError, EffectAst, LibraryBottomOrderAst, ObjectFilter, PlayerAst, ReturnControllerAst,
    SubjectVerbActionAst, SubjectVerbEffectAst, TagKey, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::effect_sentences;
use crate::grammar::effects::{clause_dispatch_shapes, control_copy_attach_shapes};
use crate::grammar::permission_facts::subject_filters as permission_subject_filters;
use crate::grammar::sentence_markers::{self, LeadingMayActor};
use crate::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::model::visit::{for_each_nested_effects, for_each_nested_effects_mut};
use crate::object_filters::parse_object_filter_lexed;
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::CardType;
use crate::util::{helper_tag_for_tokens, strip_leading_token_words_any, trim_commas};
use crate::zone::Zone;
use winnow::Parser;

fn exiled_top_collection_tag(effect: &EffectAst) -> Option<TagKey> {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::ExileTopOfLibrary {
                tags,
                accumulated_tags,
                ..
            },
        ..
    }) = effect
    {
        return tags.first().or_else(|| accumulated_tags.first()).cloned();
    }

    let mut found = None;
    for_each_nested_effects(effect, true, |nested| {
        if found.is_none() {
            found = nested.iter().find_map(exiled_top_collection_tag);
        }
    });
    found
}

pub(crate) fn find_exiled_top_collection_tag(effects: &[EffectAst]) -> Option<TagKey> {
    effects.iter().find_map(exiled_top_collection_tag)
}

fn explicit_battlefield_controller(tokens: &[OwnedLexToken]) -> Option<ReturnControllerAst> {
    tokens.iter().enumerate().find_map(|(index, token)| {
        token
            .is_word("under")
            .then(|| {
                control_copy_attach_shapes::parse_battlefield_controller_prefix(&tokens[index..])
            })
            .flatten()
            .map(|shape| match shape.controller {
                control_copy_attach_shapes::BattlefieldControllerShape::You => {
                    ReturnControllerAst::You
                }
                control_copy_attach_shapes::BattlefieldControllerShape::Owner => {
                    ReturnControllerAst::Owner
                }
            })
    })
}

fn tag_first_exile(effect: &mut EffectAst, tag: &TagKey) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Exile { .. } | SubjectVerbActionAst::ExileAll { .. },
            ..
        })
    ) {
        let exile = effect.clone();
        *effect = EffectAst::TagAffected {
            effect: Box::new(exile),
            tag: tag.clone(),
        };
        return true;
    }

    let mut tagged = false;
    for_each_nested_effects_mut(effect, true, |nested| {
        if tagged {
            return;
        }
        for nested_effect in nested {
            if tag_first_exile(nested_effect, tag) {
                tagged = true;
                break;
            }
        }
    });
    tagged
}

pub(crate) fn tag_first_exile_in_effects(effects: &mut [EffectAst], tag: &TagKey) -> bool {
    effects
        .iter_mut()
        .any(|effect| tag_first_exile(effect, tag))
}

fn leading_actor_player(actor: LeadingMayActor) -> PlayerAst {
    match actor {
        LeadingMayActor::ThatPlayer => PlayerAst::That,
        LeadingMayActor::You | LeadingMayActor::Default => PlayerAst::You,
    }
}

pub(crate) fn contains_word_phrase(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let words = parser_token_word_refs(tokens);
    crate::word_primitives::sequence_occurs(&words, phrase)
}

pub(crate) fn has_owner_hands_destination(tokens: &[OwnedLexToken]) -> bool {
    let words = parser_token_word_refs(tokens);
    crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["their", "owners", "hands"],
            &["their", "owners'", "hands"],
        ],
    )
}

/// Composes "exile the top N ...; put <count/filter> from among them onto the
/// battlefield" while keeping the selection scoped to the exact exiled set.
pub fn parse_exile_top_then_put_from_among_tokens(
    first: &[OwnedLexToken],
    second: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut effects) = effect_sentences::parse_effect_sentence_lexed(first) else {
        return Ok(None);
    };
    let Some(exiled_tag) = find_exiled_top_collection_tag(&effects) else {
        return Ok(None);
    };

    let second = trim_commas(second);
    let second = strip_leading_token_words_any(&second, &["then", "and"]);
    let Some(action) = sentence_markers::parse_leading_may_action_tokens(second, &["put"], true)
    else {
        return Ok(None);
    };
    let Some((
        mut count,
        mut filter,
        aggregate_constraint,
        destination,
        mut controller,
        tapped,
        attacking,
        _attack_target_player,
        all_matching,
    )) = super::ordered_control_flow_programs::parse_counted_from_looked_cards_action(
        action.tail_tokens,
    )
    else {
        return Ok(None);
    };
    if destination != Zone::Battlefield || aggregate_constraint.is_some() || attacking {
        return Ok(None);
    }
    // The leading-action marker parser may stop its payload before an authored
    // destination-controller suffix. Recover that suffix from the complete
    // clause so the tagged collection enters under the explicitly named
    // controller instead of silently falling back to Preserve.
    if controller == ReturnControllerAst::Preserve
        && let Some(explicit_controller) = explicit_battlefield_controller(second)
    {
        controller = explicit_controller;
    }
    if action.actor != LeadingMayActor::Default && count == ChoiceCount::exactly(1) {
        count = ChoiceCount::up_to(1);
    }

    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: exiled_tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let chosen_tag = helper_tag_for_tokens(second, "chosen_exiled");
    let chooser = leading_actor_player(action.actor);
    if all_matching {
        filter.zone = None;
        effects.push(EffectAst::subject_verb_tag_matching_objects(
            filter,
            vec![Zone::Exile],
            chosen_tag.clone(),
        ));
    } else {
        effects.push(EffectAst::ChooseTaggedObjectsInZone {
            filter,
            count,
            player: chooser,
            tag: chosen_tag.clone(),
            zone: Zone::Exile,
        });
    }

    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag,
        effects: vec![EffectAst::subject_verb_put_onto_battlefield(
            chooser,
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), None),
            tapped,
            controller,
        )],
    });
    Ok(Some(effects))
}

fn exclude_lands_from_spell_filter(filter: &mut ObjectFilter) {
    if !filter.excluded_card_types.contains(&CardType::Land) {
        filter.excluded_card_types.push(CardType::Land);
    }
}

pub(crate) fn parse_collection_cast_filter(
    shape: &clause_dispatch_shapes::CastTaggedCollectionShape<'_>,
) -> Result<Option<ObjectFilter>, CardTextError> {
    let Some(mut filter) =
        permission_subject_filters::parse_cast_permission_filter_tokens(shape.subject_tokens)?
    else {
        return Ok(None);
    };
    if permission_subject_filters::generic_spell_subject_requires_nonland(shape.subject_tokens) {
        exclude_lands_from_spell_filter(&mut filter);
    }
    filter.mana_value = shape.mana_value.clone();
    Ok(Some(filter))
}

fn remaining_exiled_filter(
    mut filter: ObjectFilter,
    exiled_tag: &TagKey,
    chosen_tag: &TagKey,
) -> ObjectFilter {
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: exiled_tag.clone(),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: chosen_tag.clone(),
        relation: TaggedOpbjectRelation::IsNotTaggedObject,
    });
    filter
}

fn move_all_remaining_exiled(
    filter: ObjectFilter,
    zone: Zone,
    order: Option<LibraryBottomOrderAst>,
) -> EffectAst {
    EffectAst::subject_verb_move_all_to_zone(
        TargetAst::Object(filter, None, None),
        zone,
        false,
        ReturnControllerAst::Preserve,
        false,
        None,
    )
    .with_destination_player_surface(Some(PlayerAst::You))
    .with_library_order(order, PlayerAst::You)
}

pub(crate) fn parse_remaining_exiled_partition(
    tokens: &[OwnedLexToken],
    exiled_tag: &TagKey,
    chosen_tag: &TagKey,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mentions_remaining = contains_word_phrase(tokens, &["not", "cast", "this", "way"])
        || contains_word_phrase(tokens, &["weren't", "cast"])
        // `parser_token_word_refs` intentionally removes apostrophes from
        // token pieces, so authored "weren't" reaches this word-level guard
        // as "werent" even though token parsers retain the source spelling.
        || contains_word_phrase(tokens, &["werent", "cast"]);
    let direct_rest = contains_word_phrase(tokens, &["put", "the", "rest"]);
    if (!mentions_remaining || !contains_word_phrase(tokens, &["exiled"])) && !direct_rest {
        return Ok(None);
    }

    // Partition the remaining filtered cards into hand, then move the rest to
    // the library bottom. Zone scoping makes the second move the exact
    // complement of the first after it resolves.
    if contains_word_phrase(tokens, &["into", "your", "hand"])
        && contains_word_phrase(
            tokens,
            &[
                "the", "rest", "on", "the", "bottom", "of", "your", "library",
            ],
        )
    {
        let Some((_, after_exiled)) = crate::grammar::primitives::parse_prefix(
            tokens,
            (
                winnow::combinator::opt(crate::grammar::primitives::kw("then")),
                crate::grammar::primitives::phrase(&["put", "the", "exiled"]),
            )
                .void(),
        ) else {
            return Ok(None);
        };
        let Some((filter_end, _, _)) =
            crate::grammar::primitives::find_prefix(after_exiled, || {
                crate::grammar::primitives::any_phrase(&[
                    &["that", "weren't", "cast"],
                    &["not", "cast", "this", "way"],
                ])
            })
        else {
            return Ok(None);
        };
        let filter_tokens = after_exiled.get(..filter_end).unwrap_or_default();
        let Ok(filter) = parse_object_filter_lexed(filter_tokens, false) else {
            return Ok(None);
        };
        let hand_filter = remaining_exiled_filter(filter, exiled_tag, chosen_tag);
        let rest_filter = remaining_exiled_filter(ObjectFilter::default(), exiled_tag, chosen_tag);
        return Ok(Some(vec![
            move_all_remaining_exiled(hand_filter, Zone::Hand, None),
            move_all_remaining_exiled(
                rest_filter,
                Zone::Library,
                Some(LibraryBottomOrderAst::Random),
            ),
        ]));
    }

    let destination = if contains_word_phrase(tokens, &["into", "your", "graveyard"]) {
        (Zone::Graveyard, None)
    } else if contains_word_phrase(tokens, &["on", "the", "bottom", "of", "your", "library"])
        && contains_word_phrase(tokens, &["random", "order"])
    {
        (Zone::Library, Some(LibraryBottomOrderAst::Random))
    } else {
        return Ok(None);
    };
    let filter = remaining_exiled_filter(ObjectFilter::default(), exiled_tag, chosen_tag);
    Ok(Some(vec![move_all_remaining_exiled(
        filter,
        destination.0,
        destination.1,
    )]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_support::compile_effects;
    use crate::lexer::{lex_line, split_lexed_sentences};
    use crate::model::facts::EffectLoweringContext;

    fn sentence_inputs(text: &str) -> Vec<SentenceInput> {
        let tokens = lex_line(text, 0).expect("collection-cast fixture should lex");
        split_lexed_sentences(&tokens)
            .into_iter()
            .map(SentenceInput::from_lexed)
            .collect()
    }

    fn registry_match(text: &str) -> super::super::super::DocumentProgramMatch {
        let sentences = sentence_inputs(text);
        super::super::super::try_parse_document_program(&sentences, 0)
            .expect("collection-cast registry lookup should not error")
            .expect("collection-cast registry should match")
    }

    fn chosen_filter_and_count(effects: &[EffectAst]) -> (&ObjectFilter, ChoiceCount) {
        effects
            .iter()
            .find_map(|effect| match effect {
                EffectAst::ChooseTaggedObjectsInZone { filter, count, .. } => {
                    Some((filter, *count))
                }
                _ => None,
            })
            .expect("collection cast should choose from the exact tagged exile pool")
    }

    #[test]
    fn collection_cast_registry_preserves_cardinality_and_global_filter_cap() {
        let cases = [
            (
                "Exile the top X cards of your library. You may cast instant and sorcery spells with mana value X or less from among them without paying their mana costs. Then put all cards exiled this way that weren't cast into your graveyard.",
                ChoiceCount::any_number(),
                3,
                "exiled-top-procedure",
                true,
            ),
            (
                "Exile the top six cards of your library. You may cast up to two sorcery spells with mana value 3 or less from among them without paying their mana costs. Put the exiled cards not cast this way on the bottom of your library in a random order.",
                ChoiceCount::up_to(2),
                3,
                "exiled-top-procedure",
                true,
            ),
            (
                "Exile the top X cards of your library. You may cast an instant or sorcery spell with mana value X or less from among them without paying its mana cost. Then put the exiled instant and sorcery cards that weren't cast this way into your hand and the rest on the bottom of your library in a random order.",
                ChoiceCount::up_to(1),
                3,
                "exiled-top-procedure",
                true,
            ),
            (
                "Target opponent exiles the top X cards of their library. You may cast any number of spells with mana value X or less from among them without paying their mana costs.",
                ChoiceCount::any_number(),
                2,
                "exiled-top-procedure",
                true,
            ),
            (
                "Exile the top eight cards of your library. You may cast an Aura spell from among them without paying its mana cost. Then put the rest on the bottom of your library in a random order.",
                ChoiceCount::up_to(1),
                3,
                "exiled-top-procedure",
                false,
            ),
        ];

        for (text, expected_count, consumed, expected_rule, has_mana_cap) in cases {
            let matched = registry_match(text);
            assert_eq!(matched.name, expected_rule, "{text}");
            assert_eq!(matched.consumed_sentences, consumed, "{text}");
            let (filter, count) = chosen_filter_and_count(&matched.effects);
            assert_eq!(count, expected_count, "{text}");
            assert_eq!(filter.zone, Some(Zone::Exile), "{text}");
            assert_eq!(filter.mana_value.is_some(), has_mana_cap, "{text}");
            assert!(
                filter.tagged_constraints.iter().any(|constraint| {
                    constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                }),
                "cast choice must stay scoped to the exact exiled set: {text}"
            );
        }
    }

    #[test]
    fn collection_cast_partition_uses_actual_selected_subset_and_remaining_exile() {
        let matched = registry_match(
            "Exile the top X cards of your library. You may cast an instant or sorcery spell with mana value X or less from among them without paying its mana cost. Then put the exiled instant and sorcery cards that weren't cast this way into your hand and the rest on the bottom of your library in a random order.",
        );
        let debug = format!("{:#?}", matched.effects);
        assert_eq!(debug.matches("ForEachTagged").count(), 1, "{debug}");
        assert_eq!(debug.matches("MoveToZone").count(), 2, "{debug}");
        assert!(debug.contains("IsNotTaggedObject"), "{debug}");
        assert!(debug.contains("zone: Hand"), "{debug}");
        assert!(debug.contains("zone: Library"), "{debug}");
        assert!(
            debug.contains("order: Some(\n                    Random"),
            "{debug}"
        );
    }

    #[test]
    fn collection_cast_partition_keeps_both_provenance_constraints_after_lowering() {
        let cases = [
            (
                "Exile the top X cards of your library. You may cast instant and sorcery spells with mana value X or less from among them without paying their mana costs. Then put all cards exiled this way that weren't cast into your graveyard.",
                1,
            ),
            (
                "Exile the top X cards of your library. You may cast an instant or sorcery spell with mana value X or less from among them without paying its mana cost. Then put the exiled instant and sorcery cards that weren't cast this way into your hand and the rest on the bottom of your library in a random order.",
                2,
            ),
        ];

        for (text, expected_moves) in cases {
            let matched = registry_match(text);
            let (lowered, _) = compile_effects(&matched.effects, &mut EffectLoweringContext::new())
                .expect("collection cast partition should lower");
            let debug = format!("{lowered:#?}");
            assert_eq!(
                debug.matches("MoveToZoneEffect").count(),
                expected_moves,
                "{debug}"
            );
            assert_eq!(
                debug.matches("relation: IsNotTaggedObject").count(),
                expected_moves,
                "each remainder move must exclude the exact selected cast set: {debug}"
            );
            assert_eq!(
                debug.matches("relation: IsTaggedObject").count(),
                expected_moves + 1,
                "the cast choice and every remainder move must retain exile-set membership: {debug}"
            );
        }
    }
}
