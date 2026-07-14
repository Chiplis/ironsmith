use super::super::SentenceInput;
use crate::cards::builders::{
    CardTextError, EffectAst, ObjectFilter, PlayerAst, ReturnControllerAst, SubjectVerbActionAst,
    SubjectVerbEffectAst, TagKey, TargetAst,
};
use crate::effect::ChoiceCount;
use crate::runtime_backend::effect_ast_traversal::{
    for_each_nested_effects, for_each_nested_effects_mut,
};
use crate::runtime_backend::effect_sentences;
use crate::runtime_backend::families::activation_and_restrictions::parse_may_cast_it_sentence;
use crate::runtime_backend::front_end::grammar::sentence_markers::{self, LeadingMayActor};
use crate::runtime_backend::front_end::lexer::{OwnedLexToken, parser_token_word_refs};
use crate::runtime_backend::grammar::effects::{
    clause_dispatch_shapes, control_copy_attach_shapes,
};
use crate::runtime_backend::util::{
    helper_tag_for_tokens, strip_leading_token_words_any, trim_commas,
};
use crate::target::{TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::zone::Zone;

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

fn find_exiled_top_collection_tag(effects: &[EffectAst]) -> Option<TagKey> {
    effects.iter().find_map(exiled_top_collection_tag)
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

fn tag_first_exile_in_effects(effects: &mut [EffectAst], tag: &TagKey) -> bool {
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

fn contains_word_phrase(tokens: &[OwnedLexToken], phrase: &[&str]) -> bool {
    let words = parser_token_word_refs(tokens);
    words.windows(phrase.len()).any(|window| window == phrase)
}

/// Composes "exile the top N ...; put <count/filter> from among them onto the
/// battlefield" while keeping the selection scoped to the exact exiled set.
pub(crate) fn parse_exile_top_then_put_from_among_onto_battlefield(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(exiled_tag) = find_exiled_top_collection_tag(&effects) else {
        return Ok(None);
    };

    let second = trim_commas(sentences[sentence_idx + 1].lowered());
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
        tapped,
        attacking,
        _attack_target_player,
        all_matching,
    )) = super::triples::parse_counted_from_looked_cards_action(action.tail_tokens)
    else {
        return Ok(None);
    };
    if destination != Zone::Battlefield || aggregate_constraint.is_some() || attacking {
        return Ok(None);
    }
    if action.actor != LeadingMayActor::Default && count == ChoiceCount::exactly(1) {
        count = ChoiceCount::up_to(1);
    }

    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: exiled_tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let chosen_tag = helper_tag_for_tokens(sentences[sentence_idx + 1].lowered(), "chosen_exiled");
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

    let controller = if contains_word_phrase(second, &["under", "your", "control"]) {
        ReturnControllerAst::You
    } else {
        ReturnControllerAst::Preserve
    };
    effects.push(EffectAst::ForEachTagged {
        tag: chosen_tag,
        effects: vec![EffectAst::subject_verb_put_onto_battlefield(
            chooser,
            TargetAst::Tagged(TagKey::from(crate::cards::builders::IT_TAG), None),
            tapped,
            controller,
        )],
    });
    Ok(Some(effects))
}

/// Composes "exile the top N ...; you may cast any number ... from among
/// them" using the moved-object tag minted by the exile effect.
pub(crate) fn parse_exile_top_then_cast_any_number_free(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Ok(mut effects) =
        effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered())
    else {
        return Ok(None);
    };
    let Some(exiled_tag) = find_exiled_top_collection_tag(&effects) else {
        return Ok(None);
    };
    let Some(shape) =
        clause_dispatch_shapes::parse_cast_any_tagged_shape(sentences[sentence_idx + 1].lowered())
    else {
        return Ok(None);
    };

    let mut filter = ObjectFilter::nonland().in_zone(Zone::Exile);
    filter.mana_value = shape.mana_value;
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: exiled_tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });
    effects.push(EffectAst::May {
        effects: vec![EffectAst::ForEachObject {
            filter,
            effects: vec![EffectAst::May {
                effects: vec![EffectAst::subject_verb_cast_tagged(
                    TagKey::from(crate::cards::builders::IT_TAG),
                    PlayerAst::You,
                    false,
                    false,
                    true,
                    None,
                )],
            }],
        }],
    });
    Ok(Some(effects))
}

/// Composes "exile cards at random; choose a card from among them and copy
/// it; you may cast the copy". The copied spell permission remains bound to
/// the chosen member of the tagged exile collection.
pub(crate) fn parse_random_graveyard_exile_choose_copy_then_cast_copy(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let mut effects =
        match effect_sentences::parse_effect_sentence_lexed(sentences[sentence_idx].lowered()) {
            Ok(effects) => effects,
            Err(_) => return Ok(None),
        };
    let exiled_tag = helper_tag_for_tokens(sentences[sentence_idx].lowered(), "exiled");
    if !tag_first_exile_in_effects(&mut effects, &exiled_tag) {
        return Ok(None);
    }

    let second = sentences[sentence_idx + 1].lowered();
    if !contains_word_phrase(second, &["and", "copy", "it"]) {
        return Ok(None);
    }
    let Some(shape) = control_copy_attach_shapes::parse_from_among_them_shape(second) else {
        return Ok(None);
    };
    let filter_tokens = strip_leading_token_words_any(shape.filter_tokens, &["choose"]);
    let Some(mut filter) = effect_sentences::parse_looked_card_choice_filter(filter_tokens) else {
        return Ok(None);
    };
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: exiled_tag,
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    let Some(cast) = parse_may_cast_it_sentence(sentences[sentence_idx + 2].lowered()) else {
        return Ok(None);
    };
    if !cast.as_copy || !cast.without_paying_mana_cost {
        return Ok(None);
    }
    let chosen_tag = helper_tag_for_tokens(second, "chosen_exiled");
    effects.push(EffectAst::ChooseTaggedObjectsInZone {
        filter,
        count: ChoiceCount::exactly(1),
        player: PlayerAst::You,
        tag: chosen_tag.clone(),
        zone: Zone::Exile,
    });
    effects.push(EffectAst::May {
        effects: vec![EffectAst::subject_verb_cast_tagged(
            chosen_tag,
            cast.player,
            false,
            true,
            true,
            cast.cost_reduction,
        )],
    });
    Ok(Some(effects))
}
