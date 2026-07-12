use super::super::grammar::effects as search_grammar;
use super::super::grammar::primitives as grammar;
use super::super::grammar::values::parse_value_comparison_tokens;
use super::super::lexer::{
    OwnedLexToken, find_token_word_sequence_span, token_word_refs, trim_lexed_commas,
};
use super::super::object_filters::{parse_object_filter, parse_object_filter_lexed};
use super::super::util::{
    helper_tag_for_tokens, parse_number, parse_number_word_u32, parse_subject, parse_target_phrase,
    span_from_tokens, strip_leading_token_words_any, trim_commas,
};
use super::sentence_helpers::*;
use super::{
    find_verb, parse_effect_chain, parse_effect_chain_with_subject_verb_primitives,
    parse_effect_clause,
};
use crate::cards::builders::{
    CardTextError, CarryContext, ChoiceCount, EffectAst, IT_TAG, LibraryBottomOrderAst,
    LibraryConsultModeAst, LibraryConsultStopRuleAst, PlayerAst, ReturnControllerAst, SubjectAst,
    SubjectVerbActionAst, SubjectVerbRoleAst, TagKey, TargetAst, TextSpan,
};
use crate::effect::SearchSelectionMode;
use crate::target::{ObjectFilter, PlayerFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

#[allow(dead_code)]
#[derive(Clone)]
pub(crate) enum SearchLibraryManaConstraint {
    Equal(u32),
    LessThanOrEqual(u32),
    GreaterThanOrEqual(u32),
    OneOf(Vec<u32>),
}

const SEARCH_ENCHANT_WORD: &str = "enchant";
const SEARCH_EARTHBEND_WORD: &str = "earthbend";

#[derive(Clone)]
struct SearchZonePairClause {
    owner: PlayerFilter,
    first_zone: Zone,
    second_zone: Zone,
}

fn segment_starts_effect_lexed(tokens: &[OwnedLexToken]) -> bool {
    super::lex_chain_helpers::segment_has_effect_head_lexed(tokens)
}

pub(crate) fn parse_search_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::super::grammar::effects::parse_search_library_sentence_with_grammar_entrypoint_lexed(
        tokens,
        segment_starts_effect_lexed,
        super::chain_carry::parse_effect_chain_with_subject_verb_primitives_lexed,
        super::clause_dispatch::parse_effect_clause_lexed,
    )
}

#[allow(dead_code)]
pub(crate) fn word_slice_mentions_nth_from_top(words: &[&str]) -> bool {
    search_grammar::search_library_has_unsupported_top_position_probe(words)
}

pub(crate) fn parse_restriction_duration_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    Ok(
        search_grammar::parse_search_restriction_duration_shape_lexed(tokens)?
            .map(|shape| (shape.duration, shape.remainder)),
    )
}

#[allow(dead_code)]
pub(crate) fn extract_search_library_mana_constraint(
    filter_tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, SearchLibraryManaConstraint)> {
    let (clause_token_start, clause_token_end) =
        find_token_word_sequence_span(filter_tokens, &["with", "mana", "cost"])
            .or_else(|| find_token_word_sequence_span(filter_tokens, &["with", "mana", "value"]))?;
    let base_filter_tokens = trim_commas(&filter_tokens[..clause_token_start]);
    if base_filter_tokens.is_empty() {
        return None;
    }

    let clause_tokens = trim_lexed_commas(&filter_tokens[clause_token_end..]);
    if clause_tokens.is_empty() {
        return None;
    }

    let parse_single_u32_clause = |tokens: &[OwnedLexToken]| -> Option<u32> {
        let (value, used) = parse_number(tokens)?;
        (used == tokens.len()).then_some(value)
    };
    let constraint = if let Some(value) = parse_single_u32_clause(clause_tokens) {
        SearchLibraryManaConstraint::Equal(value)
    } else if let Some((operator, value_tokens)) = parse_value_comparison_tokens(clause_tokens) {
        let value = parse_single_u32_clause(value_tokens)?;
        match operator {
            crate::effect::ValueComparisonOperator::LessThanOrEqual => {
                SearchLibraryManaConstraint::LessThanOrEqual(value)
            }
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual => {
                SearchLibraryManaConstraint::GreaterThanOrEqual(value)
            }
            _ => return None,
        }
    } else {
        let [left, middle, right] = clause_tokens else {
            return None;
        };
        if !search_grammar::search_library_token_is_or(middle) {
            return None;
        }
        SearchLibraryManaConstraint::OneOf(vec![
            parse_single_u32_clause(std::slice::from_ref(left))?,
            parse_single_u32_clause(std::slice::from_ref(right))?,
        ])
    };

    Some((base_filter_tokens, constraint))
}

#[allow(dead_code)]
pub(crate) fn apply_search_library_mana_constraint(
    filter: &mut ObjectFilter,
    constraint: SearchLibraryManaConstraint,
) {
    if !filter.any_of.is_empty() {
        for nested in &mut filter.any_of {
            apply_search_library_mana_constraint(nested, constraint.clone());
        }
        return;
    }

    let build_branch = |base: &ObjectFilter, mana_value: crate::filter::Comparison| {
        let mut branch = base.clone();
        branch.has_mana_cost = true;
        branch.no_x_in_cost = true;
        branch.mana_value = Some(mana_value);
        branch
    };

    match constraint {
        SearchLibraryManaConstraint::Equal(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::Equal(value as i32));
        }
        SearchLibraryManaConstraint::LessThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::GreaterThanOrEqual(value) => {
            filter.has_mana_cost = true;
            filter.no_x_in_cost = true;
            filter.mana_value = Some(crate::filter::Comparison::GreaterThanOrEqual(value as i32));
        }
        SearchLibraryManaConstraint::OneOf(values) => {
            let base = filter.clone();
            *filter = ObjectFilter::default();
            filter.any_of = values
                .into_iter()
                .map(|value| build_branch(&base, crate::filter::Comparison::Equal(value as i32)))
                .collect();
        }
    }
}

#[allow(dead_code)]
pub(crate) fn split_search_same_name_reference_filter(
    tokens: &[OwnedLexToken],
) -> Option<(Vec<OwnedLexToken>, Vec<OwnedLexToken>)> {
    let (start_token_idx, end_token_idx) =
        find_token_word_sequence_span(tokens, &["with", "the", "same", "name", "as"])
            .or_else(|| find_token_word_sequence_span(tokens, &["with", "same", "name", "as"]))?;
    let base_filter_tokens = trim_commas(&tokens[..start_token_idx]);
    let reference_tokens = trim_commas(&tokens[end_token_idx..]);
    Some((base_filter_tokens, reference_tokens))
}

#[allow(dead_code)]
pub(crate) fn is_same_name_that_reference_words(words: &[&str]) -> bool {
    search_grammar::is_same_name_that_reference_words(words)
}

pub(crate) fn normalize_search_library_filter(filter: &mut ObjectFilter) {
    filter.zone = None;
    if filter.subtypes.iter().any(|subtype| {
        matches!(
            subtype,
            Subtype::Plains
                | Subtype::Island
                | Subtype::Swamp
                | Subtype::Mountain
                | Subtype::Forest
                | Subtype::Desert
        )
    }) && !filter
        .card_types
        .iter()
        .any(|card_type| *card_type == CardType::Land)
    {
        filter.card_types.push(CardType::Land);
    }

    for nested in &mut filter.any_of {
        normalize_search_library_filter(nested);
    }
}

pub(crate) fn parse_shuffle_graveyard_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_shuffle_graveyard_shape_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = shape.subject_tokens;
    let optional_shuffle = shape.optional_shuffle;
    let each_player_subject = shape.each_player_subject;
    let subject = if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else if each_player_subject {
        SubjectAst::Player(PlayerAst::Implicit)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let owner_library_destination = shape.owner_library_destination;
    let trailing_tokens = shape.trailing_tokens.to_vec();
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
            if each_player_subject {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player(effect, CarryContext::ForEachPlayer);
                }
            } else {
                for effect in &mut trailing_effects {
                    maybe_apply_carried_player_with_clause(
                        effect,
                        CarryContext::Player(player),
                        &trailing_tokens,
                    );
                }
            }
            effects.extend(trailing_effects);
            Ok(Some(effects))
        };
    let wrap_optional = |effects: Vec<EffectAst>| -> Vec<EffectAst> {
        if optional_shuffle {
            vec![EffectAst::MayByPlayer { player, effects }]
        } else {
            effects
        }
    };

    let target_tokens = shape.target_tokens;
    let has_target_selector = shape.has_target_selector;
    if !has_target_selector {
        let mut effects = Vec::new();
        let has_source_and_graveyard_clause = shape.has_source_and_graveyard_clause;
        let has_hand_clause = shape.has_hand_clause;
        if has_source_and_graveyard_clause {
            effects.push(EffectAst::subject_verb_move_to_zone(
                TargetAst::Source(None),
                Zone::Library,
                false,
                ReturnControllerAst::Preserve,
                false,
                None,
            ));
            if owner_library_destination {
                effects.push(EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ));
            }
            effects.push(EffectAst::subject_verb_shuffle_graveyard_into_library(
                player,
            ));
        } else if has_hand_clause {
            effects.push(EffectAst::subject_verb_shuffle_hand_and_graveyard_into_library(player));
        } else {
            effects.push(EffectAst::subject_verb_shuffle_graveyard_into_library(
                player,
            ));
        }
        if each_player_subject {
            return append_trailing(vec![EffectAst::ForEachPlayer {
                effects: wrap_optional(effects),
            }]);
        }
        return append_trailing(wrap_optional(effects));
    }

    let mut target = parse_target_phrase(&target_tokens)?;
    apply_shuffle_subject_graveyard_owner_context(&mut target, subject);

    append_trailing(vec![EffectAst::subject_verb_shuffle_objects_into_library(
        player, target,
    )])
}

pub(crate) fn parse_shuffle_object_into_library_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_shuffle_object_shape_lexed(tokens) else {
        return Ok(None);
    };
    let subject_tokens = shape.subject_tokens;
    let owner_of_subject_target = shape
        .owner_subject_target_tokens
        .map(parse_target_phrase)
        .transpose()?;

    let subject = if owner_of_subject_target.is_some() {
        SubjectAst::Player(PlayerAst::ItsOwner)
    } else if subject_tokens.is_empty() {
        SubjectAst::Player(PlayerAst::You)
    } else {
        parse_subject(&subject_tokens)
    };
    let player = match subject {
        SubjectAst::Player(player) => player,
        SubjectAst::This => return Ok(None),
    };

    let trailing_tokens = shape.trailing_tokens.to_vec();
    let append_trailing =
        |mut effects: Vec<EffectAst>| -> Result<Option<Vec<EffectAst>>, CardTextError> {
            if trailing_tokens.is_empty() {
                return Ok(Some(effects));
            }
            let mut trailing_effects = parse_effect_chain(&trailing_tokens)?;
            for effect in &mut trailing_effects {
                maybe_apply_carried_player_with_clause(
                    effect,
                    CarryContext::Player(player),
                    &trailing_tokens,
                );
            }
            effects.extend(trailing_effects);
            Ok(Some(effects))
        };

    let target_tokens = shape.target_tokens;
    if let Some(target) = owner_of_subject_target {
        if shape.reference == search_grammar::SearchShuffleObjectReference::SingularBackReference {
            if !trailing_tokens.is_empty() {
                return append_trailing(vec![
                    EffectAst::subject_verb_move_to_zone(
                        target,
                        Zone::Library,
                        false,
                        ReturnControllerAst::Preserve,
                        false,
                        None,
                    ),
                    EffectAst::subject_verb(
                        SubjectVerbRoleAst::LibraryOwner,
                        PlayerAst::ItsOwner,
                        SubjectVerbActionAst::ShuffleLibrary,
                    ),
                ]);
            }
            return append_trailing(vec![EffectAst::subject_verb_shuffle_objects_into_library(
                PlayerAst::ItsOwner,
                target,
            )]);
        }
        return Ok(None);
    }
    if matches!(subject, SubjectAst::Player(PlayerAst::ItsOwner))
        && shape.reference == search_grammar::SearchShuffleObjectReference::PluralTaggedReference
    {
        return append_trailing(vec![EffectAst::ForEachTagged {
            tag: TagKey::from(IT_TAG),
            effects: vec![
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(TagKey::from(IT_TAG), span_from_tokens(&target_tokens)),
                    Zone::Library,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                EffectAst::subject_verb(
                    SubjectVerbRoleAst::LibraryOwner,
                    PlayerAst::ItsOwner,
                    SubjectVerbActionAst::ShuffleLibrary,
                ),
            ],
        }]);
    }
    let target = parse_target_phrase(&target_tokens)?;

    append_trailing(vec![EffectAst::subject_verb_shuffle_objects_into_library(
        player, target,
    )])
}

pub(crate) fn parse_exile_hand_and_graveyard_bundle_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    let trimmed_tokens = trim_commas(tokens);
    let clause_tokens = strip_leading_token_words_any(&trimmed_tokens, &["then", "and"]).to_vec();
    if clause_tokens.is_empty() {
        return Ok(None);
    }

    if grammar::match_word_prefix(&clause_tokens, &["exile", "all", "cards", "from"]).is_none() {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "hand")
        && !grammar::contains_word(&clause_tokens, "hands")
    {
        return Ok(None);
    }
    if !grammar::contains_word(&clause_tokens, "graveyard")
        && !grammar::contains_word(&clause_tokens, "graveyards")
    {
        return Ok(None);
    }
    let Some(zone_pair) = parse_search_exile_all_cards_zone_pair(&clause_tokens) else {
        return Ok(None);
    };

    let mut first_filter = ObjectFilter::default().in_zone(zone_pair.first_zone);
    first_filter.owner = Some(zone_pair.owner.clone());
    let mut second_filter = ObjectFilter::default().in_zone(zone_pair.second_zone);
    second_filter.owner = Some(zone_pair.owner);

    Ok(Some(vec![
        EffectAst::subject_verb_exile_all(first_filter, false),
        EffectAst::subject_verb_exile_all(second_filter, false),
    ]))
}

fn parse_search_exile_all_cards_zone_pair(
    tokens: &[OwnedLexToken],
) -> Option<SearchZonePairClause> {
    search_grammar::parse_search_exile_zone_pair_shape_lexed(tokens).map(|shape| {
        SearchZonePairClause {
            owner: shape.owner,
            first_zone: shape.first_zone,
            second_zone: shape.second_zone,
        }
    })
}

pub(crate) fn parse_target_player_exiles_creature_and_graveyard_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_target_exile_bundle_shape_lexed(tokens) else {
        return Ok(None);
    };
    let subject_player = shape.player;
    let subject_filter = shape.filter;

    let mut creature_filter = ObjectFilter::creature();
    creature_filter.controller = Some(subject_filter.clone());

    let mut graveyard_filter = ObjectFilter::default().in_zone(Zone::Graveyard);
    graveyard_filter.owner = Some(subject_filter);

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter: creature_filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: subject_player,
            tag: TagKey::from(IT_TAG),
        },
        EffectAst::subject_verb_exile(TargetAst::Tagged(TagKey::from(IT_TAG), None), false),
        EffectAst::subject_verb_exile_all(graveyard_filter, false),
    ]))
}

pub(crate) fn parse_for_each_exiled_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::Exiled {
        return Ok(None);
    }
    let words_all = token_word_refs(tokens);
    if shape.permanent_card_type_consult {
        let filter = ObjectFilter::permanent().shares_card_type_with_tagged(IT_TAG);
        let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
        let matched_tag = helper_tag_for_tokens(tokens, "chosen");

        return Ok(Some(vec![EffectAst::ForEachTagged {
            tag: IT_TAG.into(),
            effects: vec![
                EffectAst::subject_verb_consult_top_of_library(
                    PlayerAst::Implicit,
                    LibraryConsultModeAst::Reveal,
                    filter,
                    LibraryConsultStopRuleAst::FirstMatch,
                    revealed_tag.clone(),
                    matched_tag.clone(),
                ),
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(matched_tag.clone(), None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    revealed_tag,
                    Some(matched_tag),
                    LibraryBottomOrderAst::Random,
                    PlayerAst::Implicit,
                ),
            ],
        }]));
    }

    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    if let Some(consult) = search_grammar::parse_search_exiled_consult_shape_lexed(effect_tokens) {
        let filter = parse_object_filter_lexed(consult.filter_tokens, false)?;
        let revealed_tag = helper_tag_for_tokens(tokens, "revealed");
        let matched_tag = helper_tag_for_tokens(tokens, "chosen");
        let finish = match consult.finish {
            search_grammar::SearchExiledConsultFinish::Shuffle => EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::ItsController,
                SubjectVerbActionAst::ShuffleLibrary,
            ),
            search_grammar::SearchExiledConsultFinish::PutRestOnBottom => {
                EffectAst::subject_verb_put_tagged_remainder_on_bottom_of_library(
                    revealed_tag.clone(),
                    Some(matched_tag.clone()),
                    LibraryBottomOrderAst::Random,
                    PlayerAst::ItsController,
                )
            }
        };
        return Ok(Some(vec![EffectAst::ForEachTagged {
            tag: crate::tag::SOURCE_EXILED_TAG.into(),
            effects: vec![
                EffectAst::subject_verb_consult_top_of_library(
                    PlayerAst::ItsController,
                    LibraryConsultModeAst::Reveal,
                    filter,
                    LibraryConsultStopRuleAst::FirstMatch,
                    revealed_tag,
                    matched_tag.clone(),
                ),
                EffectAst::subject_verb_move_to_zone(
                    TargetAst::Tagged(matched_tag, None),
                    Zone::Battlefield,
                    false,
                    ReturnControllerAst::Preserve,
                    false,
                    None,
                ),
                finish,
            ],
        }]));
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... exiled this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_each_player_put_permanent_cards_exiled_with_source_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !search_grammar::search_each_player_exiled_permanents_shape_lexed(tokens) {
        return Ok(None);
    }

    let mut filter = ObjectFilter::default().in_zone(Zone::Exile);
    filter.owner = Some(PlayerFilter::IteratedPlayer);
    filter.card_types = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Ok(Some(vec![EffectAst::ForEachPlayer {
        effects: vec![EffectAst::subject_verb_return_all_to_battlefield(
            filter,
            false,
            false,
            ReturnControllerAst::Owner,
        )],
    }]))
}

pub(crate) fn parse_for_each_destroyed_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::DestroyedOrDied {
        return Ok(None);
    }
    let words_all = token_word_refs(tokens);
    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            words_all.join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_for_each_put_into_graveyard_this_way_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = search_grammar::parse_search_for_each_way_shape_lexed(tokens) else {
        return Ok(None);
    };
    if shape.kind != search_grammar::SearchForEachWayKind::PutIntoGraveyard {
        return Ok(None);
    }
    let effect_tokens = shape.effect_tokens.ok_or_else(|| {
        CardTextError::ParseError(format!(
            "missing comma after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        ))
    })?;
    if effect_tokens.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }
    let effects = parse_effect_chain(effect_tokens)?;
    if effects.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "empty effect after 'for each ... this way' clause (clause: '{}')",
            token_word_refs(tokens).join(" ")
        )));
    }

    Ok(Some(vec![EffectAst::ForEachTagged {
        tag: IT_TAG.into(),
        effects,
    }]))
}

pub(crate) fn parse_earthbend_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if !words
        .first()
        .is_some_and(|word| *word == SEARCH_EARTHBEND_WORD)
    {
        return Ok(None);
    }

    let count = parse_number(tokens.get(1..).unwrap_or_default())
        .map(|(value, _)| value)
        .or_else(|| words.get(1).and_then(|word| parse_number_word_u32(word)))
        .ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing earthbend count (clause: '{}')",
                words.join(" ")
            ))
        })?;

    Ok(Some(EffectAst::subject_verb_earthbend(count)))
}

pub(crate) fn parse_enchant_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let words = token_word_refs(tokens);
    if words.is_empty() || words[0] != SEARCH_ENCHANT_WORD {
        return Ok(None);
    }

    let remaining = if tokens.len() > 1 { &tokens[1..] } else { &[] };
    let filter = match words.get(1..) {
        Some(["player"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::Any),
        Some(["opponent"]) | Some(["an", "opponent"]) => {
            crate::object::AuraAttachmentFilter::Player(PlayerFilter::Opponent)
        }
        Some(["you"]) => crate::object::AuraAttachmentFilter::Player(PlayerFilter::You),
        _ => crate::object::AuraAttachmentFilter::Object(parse_object_filter(remaining, false)?),
    };
    Ok(Some(EffectAst::subject_verb_enchant(filter)))
}

pub(crate) fn parse_restriction_duration(
    tokens: &[OwnedLexToken],
) -> Result<Option<(crate::effect::Until, Vec<OwnedLexToken>)>, CardTextError> {
    parse_restriction_duration_lexed(tokens)
}
