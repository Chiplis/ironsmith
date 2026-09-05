use super::*;
use crate::grammar::effects::counter_marker_shapes as counter_shapes;
use crate::grammar::effects::zone_counter_shapes;

fn subject_verb_put_counters_target(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::PutCounters { target, .. } => Some(target.clone()),
            SubjectVerbActionAst::PutCounterChoice { target, .. } => Some(target.clone()),
            _ => None,
        },
        _ => None,
    }
}

fn retarget_it_target_for_counter_followup(target: &mut TargetAst, source_target: &TargetAst) {
    match target {
        TargetAst::Tagged(tag, _)
            if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() =>
        {
            *target = source_target.clone();
        }
        TargetAst::Object(filter, _, _)
            if *filter == ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()) =>
        {
            *target = source_target.clone();
        }
        TargetAst::WithCount(inner, _) => {
            retarget_it_target_for_counter_followup(inner, source_target);
        }
        _ => {}
    }
}

fn retarget_it_filter_for_counter_followup(
    filter: &mut ObjectFilter,
    source_filter: &ObjectFilter,
) {
    if *filter == ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind()) {
        *filter = source_filter.clone();
        return;
    }
    if let Some(targets) = filter.targets_object.as_deref_mut() {
        retarget_it_filter_for_counter_followup(targets, source_filter);
    }
    if let Some(targets) = filter.targets_only_object.as_deref_mut() {
        retarget_it_filter_for_counter_followup(targets, source_filter);
    }
    for branch in &mut filter.any_of {
        retarget_it_filter_for_counter_followup(branch, source_filter);
    }
}

fn retarget_it_restriction_for_counter_followup(
    restriction: &mut crate::effect::Restriction,
    source_filter: &ObjectFilter,
) {
    use crate::effect::Restriction;

    match restriction {
        Restriction::Attack(filter)
        | Restriction::Block(filter)
        | Restriction::MustBeBlocked(filter)
        | Restriction::BlockAlone(filter)
        | Restriction::Untap(filter)
        | Restriction::BeBlocked(filter)
        | Restriction::BeDestroyed(filter)
        | Restriction::BeRegenerated(filter)
        | Restriction::BeSacrificed(filter)
        | Restriction::HaveCountersPlaced(filter)
        | Restriction::BeTargeted(filter)
        | Restriction::BeCountered(filter)
        | Restriction::Transform(filter)
        | Restriction::PhaseOut(filter)
        | Restriction::PhaseIn(filter)
        | Restriction::AttackOrBlock(filter)
        | Restriction::AttackOrBlockAlone(filter)
        | Restriction::ActivateAbilitiesOf(filter)
        | Restriction::ActivateTapAbilitiesOf(filter)
        | Restriction::ActivateNonManaAbilitiesOf(filter) => {
            retarget_it_filter_for_counter_followup(filter, source_filter);
        }
        Restriction::BlockSpecificAttacker { blockers, attacker }
        | Restriction::MustBlockSpecificAttacker { blockers, attacker } => {
            retarget_it_filter_for_counter_followup(blockers, source_filter);
            retarget_it_filter_for_counter_followup(attacker, source_filter);
        }
        Restriction::AttackPlayerOrPlaneswalkersControlledBy { attackers, .. }
        | Restriction::AttackPlayer { attackers, .. }
        | Restriction::CastSpellsMatching(_, attackers)
        | Restriction::CastMoreThanOneSpellEachTurn(_, attackers) => {
            retarget_it_filter_for_counter_followup(attackers, source_filter);
        }
        Restriction::BeTargetedFrom(target, source) => {
            retarget_it_filter_for_counter_followup(target, source_filter);
            retarget_it_filter_for_counter_followup(source, source_filter);
        }
        Restriction::BeTargetedPlayerFrom(_, source) => {
            retarget_it_filter_for_counter_followup(source, source_filter);
        }
        _ => {}
    }
}

fn retarget_it_effect_for_counter_followup(effect: &mut EffectAst, source_target: &TargetAst) {
    let source_filter = target_ast_to_object_filter(source_target.clone());
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) => match action {
            SubjectVerbActionAst::Pump { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesToTarget { target, .. }
            | SubjectVerbActionAst::GrantToTarget { target, .. }
            | SubjectVerbActionAst::GrantAbilitiesChoiceToTarget { target, .. } => {
                retarget_it_target_for_counter_followup(target, source_target);
            }
            SubjectVerbActionAst::Cant { restriction, .. } => {
                if let Some(source_filter) = source_filter.as_ref() {
                    retarget_it_restriction_for_counter_followup(restriction, source_filter);
                }
            }
            _ => {}
        },
        EffectAst::Conditional {
            if_true, if_false, ..
        }
        | EffectAst::SelfReplacement {
            if_true, if_false, ..
        } => {
            for nested in if_true.iter_mut().chain(if_false.iter_mut()) {
                retarget_it_effect_for_counter_followup(nested, source_target);
            }
        }
        _ => {}
    }
}

fn parse_put_counter_choice_sequence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_put_counter_choice_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    let target_phrase = crate::lexer::render_token_slice(shape.target_tokens);
    let mode_texts = shape
        .counter_types
        .iter()
        .map(|counter_type| {
            format!(
                "Put {} on {target_phrase}",
                super::super::zone_counter_helpers::describe_counter_phrase_for_mode(
                    1,
                    *counter_type,
                )
            )
        })
        .collect();

    Ok(Some(vec![EffectAst::subject_verb_put_counter_choice(
        shape.counter_types,
        Value::Fixed(1),
        mode_texts,
        target,
        None,
    )]))
}

pub fn parse_sentence_put_fixed_and_counter_choice(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_put_fixed_and_counter_choice_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    let target_phrase = crate::lexer::render_token_slice(shape.target_tokens);
    let mode_texts = shape
        .counter_types
        .iter()
        .map(|counter_type| {
            format!(
                "Put {} on {target_phrase}",
                super::super::zone_counter_helpers::describe_counter_phrase_for_mode(
                    1,
                    *counter_type,
                )
            )
        })
        .collect();

    Ok(Some(vec![
        EffectAst::subject_verb_put_counters(
            shape.fixed.counter_type,
            Value::Fixed(shape.fixed.count as i32),
            target.clone(),
            None,
            false,
        ),
        EffectAst::subject_verb_put_counter_choice(
            shape.counter_types,
            Value::Fixed(1),
            mode_texts,
            target,
            None,
        ),
    ]))
}

pub fn parse_sentence_sacrifice_at_end_of_combat(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_sacrifice_at_end_of_combat_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    let filter = if shape.tagged_object {
        ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.bind())
    } else {
        parse_object_filter(shape.object_tokens, false)?
    };

    Ok(Some(vec![EffectAst::DelayedUntilEndOfCombat {
        effects: vec![EffectAst::subject_verb_sacrifice(
            PlayerAst::Implicit,
            filter,
            1,
            None,
        )],
    }]))
}

pub fn parse_sentence_for_each_counter_kind_put_or_remove(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_for_each_counter_kind_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;

    Ok(Some(vec![
        EffectAst::subject_verb_for_each_counter_kind_put_or_remove(target),
    ]))
}

fn lower_counter_placements(
    placements: Vec<counter_shapes::CounterPlacementShape<'_>>,
) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::with_capacity(placements.len());
    for placement in placements {
        if let Some(filter_tokens) =
            zone_counter_shapes::strip_each_counter_prefix(placement.target_tokens)
            && parse_counter_target_count_prefix(placement.target_tokens)?.is_none()
        {
            let filter = parse_object_filter(filter_tokens, false)?;
            effects.push(EffectAst::subject_verb_put_counters_all(
                placement.descriptor.counter_type,
                Value::Fixed(placement.descriptor.count as i32),
                filter,
            ));
            continue;
        }

        let (target, target_count) = if let Some((target_count, used)) =
            parse_counter_target_count_prefix(placement.target_tokens)?
        {
            let target_tokens = placement.target_tokens.get(used..).unwrap_or_default();
            if target_tokens.is_empty() {
                return Err(CardTextError::ParseError(
                    "missing target after counter-placement count".to_string(),
                ));
            }
            (parse_target_phrase(target_tokens)?, Some(target_count))
        } else {
            (parse_target_phrase(placement.target_tokens)?, None)
        };
        effects.push(EffectAst::subject_verb_put_counters(
            placement.descriptor.counter_type,
            Value::Fixed(placement.descriptor.count as i32),
            target,
            target_count,
            false,
        ));
    }

    Ok(effects)
}

#[path = "counter_marker_family/put_counter_sequence_readings.rs"]
mod put_counter_sequence_readings;

pub fn parse_sentence_put_counter_sequence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_put_counter_sequence_tokens(clause.tokens()) else {
        return Ok(None);
    };
    if let counter_shapes::PutCounterSequenceShape::Then {
        head_tokens,
        tail_tokens,
    } = shape
    {
        let mut effects = parse_effect_chain(head_tokens)?;
        if effects.is_empty() {
            return Ok(None);
        }
        effects.extend(parse_effect_chain(tail_tokens)?);
        return Ok(Some(effects));
    }
    let input = put_counter_sequence_readings::PutCounterSequence {
        tokens: clause.tokens(),
        clause,
    };
    match put_counter_sequence_readings::read(&input) {
        crate::recognition::ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        crate::recognition::ParseOutcome::NoMatch => {}
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
    }

    Ok(None)
}

pub fn is_pump_like_effect(effect: &EffectAst) -> bool {
    matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Pump { .. }
                | SubjectVerbActionAst::PumpByLastEffect { .. }
                | SubjectVerbActionAst::SetBasePowerToughness { .. }
                | SubjectVerbActionAst::SetBasePower { .. },
            ..
        })
    )
}

pub fn parse_gets_then_fights_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_gets_then_fights_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let pump_effect = parse_effect_clause(shape.pump_tokens)?;
    if !is_pump_like_effect(&pump_effect) {
        return Ok(None);
    }

    let creature1 = parse_target_phrase(shape.first_target_tokens)?;
    let creature2 = parse_target_phrase(shape.second_target_tokens)?;
    if matches!(
        creature1,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) || matches!(
        creature2,
        TargetAst::Player(_, _) | TargetAst::PlayerOrPlaneswalker(_, _)
    ) {
        return Err(CardTextError::ParseError(format!(
            "fight target must be a creature (clause: '{}')",
            clause.text()
        )));
    }

    Ok(Some(vec![
        pump_effect,
        EffectAst::subject_verb_fight(creature1, creature2),
    ]))
}

pub fn parse_sentence_gets_then_fights(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_gets_then_fights_sentence(clause)
}

pub fn parse_return_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = counter_shapes::parse_return_with_counters_tokens(clause.tokens()) else {
        return Ok(None);
    };
    let target = parse_target_phrase(shape.target_tokens)?;
    let return_effect = if shape.destination.attacking {
        if shape.destination.transformed {
            return Err(CardTextError::ParseError(format!(
                "unsupported transformed attacking return-with-counter destination (clause: '{}')",
                clause.text()
            )));
        }
        EffectAst::subject_verb_move_to_zone_with_attacking(
            target,
            Zone::Battlefield,
            false,
            shape.destination.controller,
            shape.destination.tapped,
            true,
            false,
            None,
        )
        .with_move_to_zone_verb_surface(ironsmith_core::MoveToZoneVerbSurface::Return)
    } else {
        EffectAst::subject_verb_return_to_battlefield(
            target,
            shape.destination.tapped,
            shape.destination.transformed,
            false,
            shape.destination.controller,
            None,
        )
    };
    let mut effects = vec![return_effect];
    let tagged_target =
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span());
    for descriptor in shape.descriptors {
        let count = Value::Fixed(descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter);
        let count = if descriptor.additional {
            count.with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
        } else {
            count
        };
        effects.push(EffectAst::subject_verb_put_counters(
            descriptor.counter_type,
            count,
            tagged_target.clone(),
            None,
            false,
        ));
    }

    let wrapped = if let Some(timing) = shape.timing {
        match timing {
            counter_shapes::CounterMarkerTimingShape::NextEndStep(player) => {
                vec![EffectAst::DelayedUntilNextEndStep { player, effects }]
            }
            counter_shapes::CounterMarkerTimingShape::NextUpkeep(player) => {
                vec![EffectAst::DelayedUntilNextUpkeep { player, effects }]
            }
            counter_shapes::CounterMarkerTimingShape::EndOfCombat => {
                vec![EffectAst::DelayedUntilEndOfCombat { effects }]
            }
        }
    } else {
        effects
    };

    Ok(Some(wrapped))
}

/// Preserve an X-sized entry-counter clause as part of the return event.
/// The fixed-number return grammar is intentionally separate because its
/// descriptor parser does not accept dynamic values.
pub fn parse_return_with_dynamic_entry_counters_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let view = crate::lexer::TokenWordView::new(clause.tokens());
    let words = view.word_refs();
    let Some(destination) = crate::word_primitives::parse_sequence_start(
        &words,
        &["to", "the", "battlefield", "with", "x", "additional"],
    ) else {
        return Ok(None);
    };
    if !crate::word_primitives::first_is(&words, "return")
        || destination <= 1
        || !crate::word_primitives::parse_sequence_suffix(&words, &["counters", "on", "it"])
    {
        return Ok(None);
    }
    let target_words = &words[1..destination];
    if !crate::word_primitives::parse_sequence_suffix(target_words, &["from", "your", "graveyard"])
    {
        return Ok(None);
    }
    let Some(counter_start) = view.token_index_after_words(destination + 6) else {
        return Ok(None);
    };
    let Some(counter_end) = view.map_word_to_token_start(words.len() - 3) else {
        return Ok(None);
    };
    if counter_start >= counter_end {
        return Ok(None);
    }
    let Some(counter_type) =
        parse_counter_type_from_tokens(&clause.tokens()[counter_start..counter_end])
    else {
        return Ok(None);
    };
    let Some(target_range) = view.token_span_for_words(1, destination) else {
        return Ok(None);
    };
    let mut target = parse_target_phrase(&clause.tokens()[target_range])?;

    fn bind_owned_graveyard(target: &mut TargetAst) -> bool {
        match target {
            TargetAst::Object(filter, ..) => {
                filter.zone = Some(Zone::Graveyard);
                filter.owner = Some(PlayerFilter::You);
                true
            }
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, ..) => {
                bind_owned_graveyard(inner)
            }
            _ => false,
        }
    }
    if !bind_owned_graveyard(&mut target) {
        return Ok(None);
    }

    let return_effect = EffectAst::subject_verb_return_to_battlefield(
        target,
        false,
        false,
        false,
        ReturnControllerAst::Preserve,
        None,
    );
    let counter_amount = Value::X
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter)
        .with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter);
    let counter_effect = EffectAst::subject_verb_put_counters(
        counter_type,
        counter_amount,
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span()),
        None,
        false,
    );
    Ok(Some(vec![return_effect, counter_effect]))
}

pub fn parse_put_onto_battlefield_with_counters_on_it_sentence(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if let Some(effects) = parse_optional_put_from_owned_hand_or_graveyard_with_counters(clause)? {
        return Ok(Some(effects));
    }
    let Some(shape) =
        counter_shapes::parse_put_onto_battlefield_with_counters_tokens(clause.tokens())
    else {
        return Ok(None);
    };
    if shape.destination.tapped || shape.destination.attacking {
        return Ok(None);
    }

    let target = parse_target_phrase(shape.target_tokens)?;
    let move_effect = if shape
        .target_tokens
        .first()
        .and_then(OwnedLexToken::as_word)
        .is_some_and(|word| matches!(word, "all" | "each"))
    {
        EffectAst::subject_verb_move_all_to_zone(
            target,
            Zone::Battlefield,
            false,
            shape.destination.controller,
            false,
            None,
        )
    } else {
        EffectAst::subject_verb_move_to_zone(
            target,
            Zone::Battlefield,
            false,
            shape.destination.controller,
            false,
            None,
        )
    }
    .with_exiled_with_source_surface(
        super::super::verb_handlers::parse_exiled_with_source_move_surface(clause.tokens()),
    );
    let move_effect = if shape.destination.transformed {
        move_effect.with_move_to_zone_transformed()
    } else {
        move_effect
    };
    let mut effects = vec![move_effect];
    let tagged_target =
        TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.bind(), clause.span());
    for descriptor in shape.descriptors {
        let count = Value::Fixed(descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter);
        let count = if descriptor.additional {
            count.with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalEntryCounter)
        } else {
            count
        };
        effects.push(EffectAst::subject_verb_put_counters(
            descriptor.counter_type,
            count,
            tagged_target.clone(),
            None,
            false,
        ));
    }

    Ok(Some(effects))
}

/// Parse the reusable cross-zone form
/// `you may put <card> onto the battlefield from your hand or graveyard with
/// <counters> on it`. The ordinary move-with-counters grammar has a single
/// source zone, so treating the trailing origin phrase as part of the entry
/// counter clause loses the chosen card and leaves only a counter action.
fn parse_optional_put_from_owned_hand_or_graveyard_with_counters(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let words = crate::lexer::token_word_refs(clause.tokens());
    let put_index = if crate::word_primitives::parse_sequence_prefix(&words, &["you", "may", "put"])
    {
        2
    } else if crate::word_primitives::parse_sequence_prefix(&words, &["may", "put"]) {
        1
    } else if crate::word_primitives::parse_sequence_prefix(&words, &["put"]) {
        0
    } else {
        return Ok(None);
    };
    let Some(onto_index) =
        crate::slice_primitives::select_position(&words[put_index + 1..], |word| *word == "onto")
            .map(|offset| put_index + 1 + offset)
    else {
        return Ok(None);
    };
    let origin = [
        "onto",
        "the",
        "battlefield",
        "from",
        "your",
        "hand",
        "or",
        "graveyard",
        "with",
    ];
    if !words
        .get(onto_index..)
        .is_some_and(|tail| crate::word_primitives::parse_sequence_prefix(tail, &origin))
    {
        return Ok(None);
    }
    let target_words = &words[put_index + 1..onto_index];
    if target_words.is_empty() {
        return Ok(None);
    }
    let counter_words = &words[onto_index + origin.len()..];
    if counter_words.len() < 3
        || !crate::word_primitives::parse_sequence_suffix(counter_words, &["on", "it"])
    {
        return Ok(None);
    }

    let target_tokens = crate::lexer::synthetic_word_tokens(target_words);
    let mut filter = parse_object_filter(&target_tokens, false)?;
    filter.zone = None;
    filter.owner = Some(PlayerFilter::You);

    let mut counter_probe_words = vec!["put", "it", "onto", "the", "battlefield", "with"];
    counter_probe_words.extend(counter_words.iter().copied());
    let counter_probe = crate::lexer::synthetic_word_tokens(counter_probe_words);
    let Some(counter_shape) =
        counter_shapes::parse_put_onto_battlefield_with_counters_tokens(&counter_probe)
    else {
        return Ok(None);
    };
    if counter_shape.destination.tapped
        || counter_shape.destination.attacking
        || counter_shape.destination.transformed
        || counter_shape.destination.controller != ReturnControllerAst::Preserve
        || counter_shape.descriptors.is_empty()
    {
        return Ok(None);
    }

    let selected = helper_tag_for_tokens(clause.tokens(), "owned_zone_entry");
    let mut effects = vec![
        EffectAst::ChooseObjectsAcrossZones {
            filter,
            count: ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: selected.clone(),
            zones: vec![Zone::Hand, Zone::Graveyard],
            search_mode: None,
        },
        EffectAst::subject_verb_move_to_zone(
            TargetAst::Tagged(selected.clone(), clause.span()),
            Zone::Battlefield,
            false,
            ReturnControllerAst::Preserve,
            false,
            None,
        ),
    ];
    for descriptor in counter_shape.descriptors {
        let count = Value::Fixed(descriptor.count as i32)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::InlineBattlefieldEntryCounter);
        effects.push(EffectAst::subject_verb_put_counters(
            descriptor.counter_type,
            count,
            TargetAst::Tagged(selected.clone(), clause.span()),
            None,
            false,
        ));
    }
    // The leading-may dispatcher owns optionality. Returning another May here
    // would produce a nested decision (`You may may put ...`) and hide this
    // correlated choose/move/entry-counter program from structural lowering.
    Ok(Some(effects))
}

pub fn parse_sentence_return_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_return_with_counters_on_it_sentence(clause)
}

pub fn parse_sentence_put_onto_battlefield_with_counters_on_it(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    parse_put_onto_battlefield_with_counters_on_it_sentence(clause)
}

pub fn replace_target_subtype(target: &mut TargetAst, subtype: Subtype) -> bool {
    match target {
        TargetAst::Object(filter, _, _) => {
            filter.subtypes = vec![subtype];
            true
        }
        TargetAst::WithCount(inner, _) => replace_target_subtype(inner, subtype),
        _ => false,
    }
}

#[cfg(test)]
#[path = "counter_marker_family_inline_dynamic_entry_counter_tests.rs"]
mod dynamic_entry_counter_tests;

#[path = "counter_marker_family/counter.rs"]
mod counter_programs;
use counter_programs::lower_put_with_additional_counter;
pub use counter_programs::{
    parse_each_player_return_with_additional_counter_sentence,
    parse_if_enters_with_additional_counter_sentence,
    parse_if_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence,
    parse_put_onto_battlefield_with_additional_counters_sentence,
    parse_sacrifice_then_put_onto_battlefield_with_additional_counters_sentence,
    parse_tagged_conditional_entry_counters_sentence,
    parse_tagged_enters_with_additional_counter_sentence,
};
#[path = "counter_marker_family/resource.rs"]
mod resource_programs;
use resource_programs::lower_sacrifice_then_put_additional;
pub use resource_programs::{parse_draw_then_connive_sentence, parse_sentence_draw_then_connive};
#[path = "counter_marker_family/zone.rs"]
mod zone_programs;
pub use zone_programs::clone_return_effect_with_subtype;
