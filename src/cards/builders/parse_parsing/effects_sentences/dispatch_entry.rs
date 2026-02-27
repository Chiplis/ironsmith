use super::*;

pub(crate) fn parse_effect_sentences(tokens: &[Token]) -> Result<Vec<EffectAst>, CardTextError> {
    let mut effects = Vec::new();
    let sentences = split_on_period(tokens);
    let mut sentence_idx = 0usize;
    let mut carried_context: Option<CarryContext> = None;

    fn effect_contains_search_library(effect: &EffectAst) -> bool {
        match effect {
            EffectAst::SearchLibrary { .. } => true,
            EffectAst::Conditional {
                if_true, if_false, ..
            } => {
                if_true.iter().any(effect_contains_search_library)
                    || if_false.iter().any(effect_contains_search_library)
            }
            EffectAst::UnlessPays { effects, .. }
            | EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. }
            | EffectAst::MayByTaggedController { effects, .. }
            | EffectAst::IfResult { effects, .. }
            | EffectAst::ForEachOpponent { effects }
            | EffectAst::ForEachPlayer { effects }
            | EffectAst::ForEachTargetPlayers { effects, .. }
            | EffectAst::ForEachObject { effects, .. }
            | EffectAst::ForEachTagged { effects, .. }
            | EffectAst::ForEachOpponentDoesNot { effects }
            | EffectAst::ForEachPlayerDoesNot { effects }
            | EffectAst::ForEachOpponentDid { effects, .. }
            | EffectAst::ForEachPlayerDid { effects, .. }
            | EffectAst::ForEachTaggedPlayer { effects, .. }
            | EffectAst::DelayedUntilNextEndStep { effects, .. }
            | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
            | EffectAst::DelayedUntilEndOfCombat { effects }
            | EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
            | EffectAst::VoteOption { effects, .. } => {
                effects.iter().any(effect_contains_search_library)
            }
            EffectAst::UnlessAction {
                effects,
                alternative,
                ..
            } => {
                effects.iter().any(effect_contains_search_library)
                    || alternative.iter().any(effect_contains_search_library)
            }
            _ => false,
        }
    }

    fn is_if_you_search_library_this_way_shuffle_sentence(tokens: &[Token]) -> bool {
        let words: Vec<&str> = words(tokens)
            .into_iter()
            .filter(|word| !is_article(word))
            .collect();
        // "If you search your library this way, shuffle."
        words.as_slice()
            == [
                "if", "you", "search", "your", "library", "this", "way", "shuffle",
            ]
            || words.as_slice()
                == [
                    "if", "you", "search", "your", "library", "this", "way", "shuffles",
                ]
    }

    while sentence_idx < sentences.len() {
        let sentence = &sentences[sentence_idx];
        if sentence.is_empty() {
            sentence_idx += 1;
            continue;
        }

        if sentence_idx + 1 < sentences.len()
            && let Some(mut combined) = parse_target_player_chooses_then_other_cant_block(
                sentence,
                &sentences[sentence_idx + 1],
            )?
        {
            parser_trace(
                "parse_effect_sentences:sequence-hit:target-chooses-other-cant-block",
                sentence,
            );
            effects.append(&mut combined);
            sentence_idx += 2;
            continue;
        }
        if sentence_idx + 1 < sentences.len()
            && let Some(mut combined) =
                parse_choose_creature_type_then_become_type(sentence, &sentences[sentence_idx + 1])?
        {
            parser_trace(
                "parse_effect_sentences:sequence-hit:choose-creature-type-then-become-type",
                sentence,
            );
            effects.append(&mut combined);
            sentence_idx += 2;
            continue;
        }
        let mut sentence_tokens = strip_embedded_token_rules_text(sentence);
        if sentence_tokens.is_empty() {
            sentence_idx += 1;
            continue;
        }
        sentence_tokens = rewrite_when_you_do_clause_prefix(&sentence_tokens);

        // Oracle frequently splits shuffle followups as a standalone sentence:
        // "If you search your library this way, shuffle." This clause is redundant when the
        // preceding sentence already compiles a library-search effect that shuffles.
        if is_if_you_search_library_this_way_shuffle_sentence(&sentence_tokens)
            && effects.iter().any(effect_contains_search_library)
        {
            parser_trace(
                "parse_effect_sentences:skip:if-you-search-library-this-way-shuffle",
                &sentence_tokens,
            );
            sentence_idx += 1;
            continue;
        }

        let mut wraps_as_if_did_not = false;
        if let Some(without_otherwise) = strip_otherwise_sentence_prefix(&sentence_tokens) {
            sentence_tokens = rewrite_otherwise_referential_subject(without_otherwise);
            wraps_as_if_did_not = true;
        }
        parser_trace("parse_effect_sentences:sentence", &sentence_tokens);

        // "Destroy ... . It/They can't be regenerated." followups.
        if is_cant_be_regenerated_followup_sentence(&sentence_tokens) {
            if apply_cant_be_regenerated_to_last_destroy_effect(&mut effects) {
                parser_trace(
                    "parse_effect_sentences:cant-be-regenerated-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone cant-be-regenerated clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }

        if sentence_idx + 1 < sentences.len() && is_simple_copy_reference_sentence(&sentence_tokens)
        {
            let next_tokens = strip_embedded_token_rules_text(&sentences[sentence_idx + 1]);
            if let Some(spec) = parse_may_cast_it_sentence(&next_tokens)
                && spec.as_copy
            {
                parser_trace(
                    "parse_effect_sentences:copy-reference-next-may-cast-copy",
                    &sentence_tokens,
                );
                effects.push(build_may_cast_tagged_effect(&spec));
                sentence_idx += 2;
                continue;
            }
        }

        if let Some(spec) = parse_may_cast_it_sentence(&sentence_tokens) {
            parser_trace(
                "parse_effect_sentences:may-cast-it-sentence",
                &sentence_tokens,
            );
            effects.push(build_may_cast_tagged_effect(&spec));
            sentence_idx += 1;
            continue;
        }

        if is_spawn_scion_token_mana_reminder(&sentence_tokens) {
            if effects
                .last()
                .is_some_and(effect_creates_eldrazi_spawn_or_scion)
            {
                parser_trace(
                    "parse_effect_sentences:spawn-scion-reminder",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token mana reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        if let Some(effect) =
            parse_sentence_exile_that_token_when_source_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-exile-when-source-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if let Some(effect) =
            parse_sentence_sacrifice_source_when_that_token_leaves(&sentence_tokens, &effects)
        {
            parser_trace(
                "parse_effect_sentences:linked-token-sacrifice-source-when-token-leaves",
                &sentence_tokens,
            );
            effects.push(effect);
            sentence_idx += 1;
            continue;
        }
        if is_generic_token_reminder_sentence(&sentence_tokens)
            && effects.last().is_some_and(effect_creates_any_token)
        {
            if append_token_reminder_to_last_create_effect(&mut effects, &sentence_tokens) {
                parser_trace(
                    "parse_effect_sentences:token-reminder-followup",
                    &sentence_tokens,
                );
                sentence_idx += 1;
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }

        if let Some(effect) = parse_choose_target_prelude_sentence(&sentence_tokens)? {
            effects.push(effect);
            carried_context = None;
            sentence_idx += 1;
            continue;
        }

        let mut sentence_effects = parse_effect_sentence(&sentence_tokens)?;
        if wraps_as_if_did_not {
            sentence_effects = vec![EffectAst::IfResult {
                predicate: IfResultPredicate::DidNot,
                effects: sentence_effects,
            }];
            carried_context = None;
        }
        collapse_token_copy_next_end_step_exile_followup(&mut sentence_effects, &sentence_tokens);
        collapse_token_copy_end_of_combat_exile_followup(&mut sentence_effects, &sentence_tokens);
        if is_that_turn_end_step_sentence(&sentence_tokens)
            && let Some(extra_turn_player) = most_recent_extra_turn_player(&effects)
            && !sentence_effects.is_empty()
        {
            sentence_effects = vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: extra_turn_player,
                effects: sentence_effects,
            }];
        }
        if words(&sentence_tokens).first().copied() == Some("you") {
            carried_context = None;
        }
        if try_apply_token_copy_followup(&mut effects, &sentence_effects)? {
            parser_trace(
                "parse_effect_sentences:token-copy-followup",
                &sentence_tokens,
            );
            sentence_idx += 1;
            continue;
        }
        if sentence_effects.is_empty()
            && !is_round_up_each_time_sentence(&sentence_tokens)
            && !is_nonsemantic_restriction_sentence(&sentence_tokens)
        {
            return Err(CardTextError::ParseError(format!(
                "sentence parsed to no semantic effects (clause: '{}')",
                words(&sentence_tokens).join(" ")
            )));
        }
        // If a token-copy modifier sentinel didn't apply (no preceding CreateTokenCopy),
        // convert it to a proper effect on the tagged "it" object.
        for effect in &mut sentence_effects {
            if matches!(effect, EffectAst::TokenCopyHasHaste) {
                let span = span_from_tokens(&sentence);
                *effect = EffectAst::GrantAbilitiesToTarget {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
                    abilities: vec![StaticAbility::haste()],
                    duration: Until::Forever,
                };
            } else if matches!(effect, EffectAst::TokenCopyGainHasteUntilEot) {
                let span = span_from_tokens(&sentence);
                *effect = EffectAst::GrantAbilitiesToTarget {
                    target: TargetAst::Tagged(TagKey::from(IT_TAG), span),
                    abilities: vec![StaticAbility::haste()],
                    duration: Until::EndOfTurn,
                };
            } else if matches!(effect, EffectAst::TokenCopySacrificeAtNextEndStep) {
                *effect = EffectAst::DelayedUntilNextEndStep {
                    player: PlayerFilter::Any,
                    effects: vec![EffectAst::Sacrifice {
                        filter: ObjectFilter::tagged(TagKey::from(IT_TAG)),
                        player: PlayerAst::Implicit,
                        count: 1,
                    }],
                };
            } else if matches!(effect, EffectAst::TokenCopyExileAtNextEndStep) {
                let span = span_from_tokens(&sentence);
                *effect = EffectAst::DelayedUntilNextEndStep {
                    player: PlayerFilter::Any,
                    effects: vec![EffectAst::Exile {
                        target: TargetAst::Object(
                            ObjectFilter::tagged(TagKey::from(IT_TAG)),
                            span,
                            None,
                        ),
                        face_down: false,
                    }],
                };
            }
        }
        for effect in &mut sentence_effects {
            if let Some(context) = carried_context {
                maybe_apply_carried_player_with_clause(effect, context, &sentence_tokens);
            }
            if let Some(context) = explicit_player_for_carry(effect) {
                carried_context = Some(context);
            }
        }
        if sentence_effects.len() == 1
            && let Some(previous_effect) = effects.last()
            && let Some(EffectAst::IfResult {
                predicate,
                effects: if_result_effects,
            }) = sentence_effects.first_mut()
        {
            if matches!(predicate, IfResultPredicate::Did)
                && matches!(previous_effect, EffectAst::UnlessPays { .. })
            {
                *predicate = IfResultPredicate::DidNot;
            }
            if let Some(previous_target) = primary_damage_target_from_effect(previous_effect) {
                replace_it_damage_target_in_effects(if_result_effects, &previous_target);
            }
        }
        let has_instead = sentence.iter().any(|token| token.is_word("instead"));
        if has_instead && sentence_effects.len() == 1 && effects.len() >= 1 {
            if matches!(
                sentence_effects.first(),
                Some(EffectAst::Conditional { .. })
            ) {
                let previous = effects.pop().expect("effects length checked above");
                let previous_target = primary_target_from_effect(&previous);
                let previous_damage_target = primary_damage_target_from_effect(&previous);
                if let Some(EffectAst::Conditional {
                    predicate,
                    mut if_true,
                    mut if_false,
                }) = sentence_effects.pop()
                {
                    if let Some(target) = previous_target {
                        replace_it_target_in_effects(&mut if_true, &target);
                    }
                    if let Some(target) = previous_damage_target {
                        replace_it_damage_target_in_effects(&mut if_true, &target);
                        replace_placeholder_damage_target_in_effects(&mut if_true, &target);
                    }
                    if_false.insert(0, previous);
                    effects.push(EffectAst::Conditional {
                        predicate,
                        if_true,
                        if_false,
                    });
                    sentence_idx += 1;
                    continue;
                }
            }
        }

        effects.extend(sentence_effects);
        sentence_idx += 1;
    }

    parser_trace("parse_effect_sentences:done", tokens);
    Ok(effects)
}

pub(crate) fn is_cant_be_regenerated_followup_sentence(tokens: &[Token]) -> bool {
    let words = normalize_cant_words(tokens);
    matches!(
        words.as_slice(),
        ["it", "cant", "be", "regenerated"]
            | ["it", "cant", "be", "regenerated", "this", "turn"]
            | ["they", "cant", "be", "regenerated"]
            | ["they", "cant", "be", "regenerated", "this", "turn"]
    )
}

pub(crate) fn apply_cant_be_regenerated_to_last_destroy_effect(effects: &mut Vec<EffectAst>) -> bool {
    let Some(last) = effects.pop() else {
        return false;
    };

    match last {
        EffectAst::Destroy { target } => {
            effects.push(EffectAst::DestroyNoRegeneration { target });
            true
        }
        EffectAst::DestroyAll { filter } => {
            effects.push(EffectAst::DestroyAllNoRegeneration { filter });
            true
        }
        EffectAst::DestroyAllOfChosenColor { filter } => {
            effects.push(EffectAst::DestroyAllOfChosenColorNoRegeneration { filter });
            true
        }
        other => {
            effects.push(other);
            false
        }
    }
}

pub(crate) fn primary_damage_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. } | EffectAst::DealDamageEqualToPower { target, .. } => {
            Some(target.clone())
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => if_true
            .iter()
            .find_map(primary_damage_target_from_effect)
            .or_else(|| if_false.iter().find_map(primary_damage_target_from_effect)),
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. }
        | EffectAst::UnlessAction {
            effects,
            alternative: _,
            ..
        } => effects.iter().find_map(primary_damage_target_from_effect),
        _ => None,
    }
}

pub(crate) fn primary_target_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::DealDamage { target, .. }
        | EffectAst::DealDamageEqualToPower { target, .. }
        | EffectAst::Counter { target }
        | EffectAst::CounterUnlessPays { target, .. }
        | EffectAst::Explore { target }
        | EffectAst::Connive { target }
        | EffectAst::Goad { target }
        | EffectAst::Tap { target }
        | EffectAst::Untap { target }
        | EffectAst::RemoveFromCombat { target }
        | EffectAst::TapOrUntap { target }
        | EffectAst::Destroy { target }
        | EffectAst::DestroyNoRegeneration { target }
        | EffectAst::Exile { target, .. }
        | EffectAst::ExileWhenSourceLeaves { target }
        | EffectAst::SacrificeSourceWhenLeaves { target }
        | EffectAst::ExileUntilSourceLeaves { target, .. }
        | EffectAst::LookAtHand { target }
        | EffectAst::Transform { target }
        | EffectAst::Flip { target }
        | EffectAst::Regenerate { target }
        | EffectAst::TargetOnly { target }
        | EffectAst::ReturnToHand { target, .. }
        | EffectAst::ReturnToBattlefield { target, .. }
        | EffectAst::MoveToZone { target, .. }
        | EffectAst::Pump { target, .. }
        | EffectAst::GrantAbilitiesToTarget { target, .. }
        | EffectAst::GrantAbilitiesChoiceToTarget { target, .. }
        | EffectAst::GrantProtectionChoice { target, .. }
        | EffectAst::PreventDamage { target, .. }
        | EffectAst::PreventAllDamageToTarget { target, .. }
        | EffectAst::PreventAllCombatDamageFromSource { source: target, .. }
        | EffectAst::RedirectNextDamageFromSourceToTarget { target, .. }
        | EffectAst::RedirectNextTimeDamageToSource { target, .. }
        | EffectAst::GainControl { target, .. } => Some(target.clone()),
        EffectAst::Conditional {
            if_true, if_false, ..
        } => if_true
            .iter()
            .find_map(primary_target_from_effect)
            .or_else(|| if_false.iter().find_map(primary_target_from_effect)),
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. }
        | EffectAst::UnlessAction {
            effects,
            alternative: _,
            ..
        } => effects.iter().find_map(primary_target_from_effect),
        _ => None,
    }
}

pub(crate) fn replace_it_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_damage_target(effect, target);
    }
}

pub(crate) fn replace_it_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_it_target(effect, target);
    }
}

pub(crate) fn is_placeholder_damage_target(target: &TargetAst) -> bool {
    matches!(
        target,
        TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, None)
    )
}

pub(crate) fn replace_placeholder_damage_target_in_effects(effects: &mut [EffectAst], target: &TargetAst) {
    for effect in effects {
        replace_placeholder_damage_target(effect, target);
    }
}

pub(crate) fn replace_placeholder_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: damage_target,
            ..
        } => {
            if is_placeholder_damage_target(damage_target) {
                *damage_target = target.clone();
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            replace_placeholder_damage_target_in_effects(if_true, target);
            replace_placeholder_damage_target_in_effects(if_false, target);
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            replace_placeholder_damage_target_in_effects(effects, target);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            replace_placeholder_damage_target_in_effects(effects, target);
            replace_placeholder_damage_target_in_effects(alternative, target);
        }
        _ => {}
    }
}

pub(crate) fn replace_unbound_x_in_damage_effects(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_damage_effect(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_damage_effect(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::LoseLife { amount, .. } => {
            if value_contains_unbound_x(amount) {
                *amount = replace_unbound_x_with_value(amount.clone(), replacement, clause)?;
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            replace_unbound_x_in_damage_effects(if_true, replacement, clause)?;
            replace_unbound_x_in_damage_effects(if_false, replacement, clause)?;
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            replace_unbound_x_in_damage_effects(effects, replacement, clause)?;
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            replace_unbound_x_in_damage_effects(effects, replacement, clause)?;
            replace_unbound_x_in_damage_effects(alternative, replacement, clause)?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effects_anywhere(
    effects: &mut [EffectAst],
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    for effect in effects {
        replace_unbound_x_in_effect_anywhere(effect, replacement, clause)?;
    }
    Ok(())
}

pub(crate) fn replace_unbound_x_in_effect_anywhere(
    effect: &mut EffectAst,
    replacement: &Value,
    clause: &str,
) -> Result<(), CardTextError> {
    fn replace_value(
        value: &mut Value,
        replacement: &Value,
        clause: &str,
    ) -> Result<(), CardTextError> {
        if value_contains_unbound_x(value) {
            *value = replace_unbound_x_with_value(value.clone(), replacement, clause)?;
        }
        Ok(())
    }

    match effect {
        EffectAst::DealDamage { amount, .. }
        | EffectAst::DealDamageEach { amount, .. }
        | EffectAst::Draw { count: amount, .. }
        | EffectAst::LoseLife { amount, .. }
        | EffectAst::GainLife { amount, .. }
        | EffectAst::PreventDamage { amount, .. }
        | EffectAst::PreventDamageEach { amount, .. }
        | EffectAst::PutCounters { count: amount, .. }
        | EffectAst::PutCountersAll { count: amount, .. }
        | EffectAst::Mill { count: amount, .. }
        | EffectAst::Discard { count: amount, .. }
        | EffectAst::Scry { count: amount, .. }
        | EffectAst::Surveil { count: amount, .. }
        | EffectAst::Discover { count: amount, .. }
        | EffectAst::PayEnergy { amount, .. }
        | EffectAst::CopySpell { count: amount, .. }
        | EffectAst::SetLifeTotal { amount, .. }
        | EffectAst::Monstrosity { amount } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::PutOrRemoveCounters {
            put_count,
            remove_count,
            ..
        } => {
            replace_value(put_count, replacement, clause)?;
            replace_value(remove_count, replacement, clause)?;
        }
        EffectAst::RemoveUpToAnyCounters { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::AddManaScaled { amount, .. }
        | EffectAst::AddManaAnyColor { amount, .. }
        | EffectAst::AddManaAnyOneColor { amount, .. }
        | EffectAst::AddManaChosenColor { amount, .. }
        | EffectAst::AddManaFromLandCouldProduce { amount, .. }
        | EffectAst::AddManaCommanderIdentity { amount, .. } => {
            replace_value(amount, replacement, clause)?;
        }
        EffectAst::CreateToken { count, .. }
        | EffectAst::CreateTokenWithMods { count, .. }
        | EffectAst::CreateTokenCopy { count, .. }
        | EffectAst::CreateTokenCopyFromSource { count, .. } => {
            replace_value(count, replacement, clause)?;
        }
        EffectAst::CounterUnlessPays {
            life,
            additional_generic,
            ..
        } => {
            if let Some(life) = life.as_mut() {
                replace_value(life, replacement, clause)?;
            }
            if let Some(generic) = additional_generic.as_mut() {
                replace_value(generic, replacement, clause)?;
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            replace_unbound_x_in_effects_anywhere(if_true, replacement, clause)?;
            replace_unbound_x_in_effects_anywhere(if_false, replacement, clause)?;
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            replace_unbound_x_in_effects_anywhere(effects, replacement, clause)?;
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            replace_unbound_x_in_effects_anywhere(effects, replacement, clause)?;
            replace_unbound_x_in_effects_anywhere(alternative, replacement, clause)?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn apply_where_x_to_damage_amounts(
    tokens: &[Token],
    effects: &mut [EffectAst],
) -> Result<(), CardTextError> {
    let clause_words = words(tokens);
    let has_deal_x = clause_words.windows(3).any(|window| {
        (window[0] == "deal" || window[0] == "deals") && window[1] == "x" && window[2] == "damage"
    });
    let has_x_life = clause_words.windows(3).any(|window| {
        (window[0] == "gain" || window[0] == "gains" || window[0] == "lose" || window[0] == "loses")
            && window[1] == "x"
            && window[2] == "life"
    });
    if !has_deal_x && !has_x_life {
        return Ok(());
    }
    let Some(where_idx) = clause_words
        .windows(3)
        .position(|window| window == ["where", "x", "is"])
    else {
        return Ok(());
    };
    let Some(where_token_idx) = token_index_for_word_index(tokens, where_idx) else {
        return Ok(());
    };
    let where_tokens = &tokens[where_token_idx..];
    let Some(where_value) = parse_where_x_value_clause(where_tokens) else {
        return Ok(());
    };
    replace_unbound_x_in_damage_effects(effects, &where_value, &clause_words.join(" "))
}

pub(crate) fn replace_it_damage_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: damage_target,
            ..
        } => {
            if target_references_it(damage_target) {
                *damage_target = target.clone();
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            replace_it_damage_target_in_effects(if_true, target);
            replace_it_damage_target_in_effects(if_false, target);
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            replace_it_damage_target_in_effects(effects, target);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            replace_it_damage_target_in_effects(effects, target);
            replace_it_damage_target_in_effects(alternative, target);
        }
        _ => {}
    }
}

pub(crate) fn replace_it_target(effect: &mut EffectAst, target: &TargetAst) {
    match effect {
        EffectAst::DealDamage {
            target: effect_target,
            ..
        }
        | EffectAst::DealDamageEqualToPower {
            target: effect_target,
            ..
        }
        | EffectAst::Counter {
            target: effect_target,
        }
        | EffectAst::CounterUnlessPays {
            target: effect_target,
            ..
        }
        | EffectAst::Explore {
            target: effect_target,
        }
        | EffectAst::Connive {
            target: effect_target,
        }
        | EffectAst::Goad {
            target: effect_target,
        }
        | EffectAst::Tap {
            target: effect_target,
        }
        | EffectAst::Untap {
            target: effect_target,
        }
        | EffectAst::RemoveFromCombat {
            target: effect_target,
        }
        | EffectAst::TapOrUntap {
            target: effect_target,
        }
        | EffectAst::Destroy {
            target: effect_target,
        }
        | EffectAst::DestroyNoRegeneration {
            target: effect_target,
        }
        | EffectAst::Exile {
            target: effect_target,
            ..
        }
        | EffectAst::ExileWhenSourceLeaves {
            target: effect_target,
        }
        | EffectAst::SacrificeSourceWhenLeaves {
            target: effect_target,
        }
        | EffectAst::ExileUntilSourceLeaves {
            target: effect_target,
            ..
        }
        | EffectAst::LookAtHand {
            target: effect_target,
        }
        | EffectAst::Transform {
            target: effect_target,
        }
        | EffectAst::Flip {
            target: effect_target,
        }
        | EffectAst::Regenerate {
            target: effect_target,
        }
        | EffectAst::TargetOnly {
            target: effect_target,
        }
        | EffectAst::ReturnToHand {
            target: effect_target,
            ..
        }
        | EffectAst::ReturnToBattlefield {
            target: effect_target,
            ..
        }
        | EffectAst::MoveToZone {
            target: effect_target,
            ..
        }
        | EffectAst::Pump {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantAbilitiesChoiceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::GrantProtectionChoice {
            target: effect_target,
            ..
        }
        | EffectAst::PreventDamage {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllDamageToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::PreventAllCombatDamageFromSource {
            source: effect_target,
            ..
        }
        | EffectAst::RedirectNextDamageFromSourceToTarget {
            target: effect_target,
            ..
        }
        | EffectAst::RedirectNextTimeDamageToSource {
            target: effect_target,
            ..
        }
        | EffectAst::GainControl {
            target: effect_target,
            ..
        } => {
            if target_references_it(effect_target) {
                *effect_target = target.clone();
            }
        }
        EffectAst::Conditional {
            if_true, if_false, ..
        } => {
            replace_it_target_in_effects(if_true, target);
            replace_it_target_in_effects(if_false, target);
        }
        EffectAst::UnlessPays { effects, .. }
        | EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. }
        | EffectAst::ForEachOpponent { effects }
        | EffectAst::ForEachPlayer { effects }
        | EffectAst::ForEachTargetPlayers { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::ForEachTagged { effects, .. }
        | EffectAst::ForEachPlayerDoesNot { effects, .. }
        | EffectAst::ForEachOpponentDoesNot { effects, .. }
        | EffectAst::ForEachPlayerDid { effects, .. }
        | EffectAst::ForEachOpponentDid { effects, .. }
        | EffectAst::ForEachTaggedPlayer { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilEndStepOfExtraTurn { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects }
        | EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedWhenLastObjectDiesThisTurn { effects, .. }
        | EffectAst::VoteOption { effects, .. } => {
            replace_it_target_in_effects(effects, target);
        }
        EffectAst::UnlessAction {
            effects,
            alternative,
            ..
        } => {
            replace_it_target_in_effects(effects, target);
            replace_it_target_in_effects(alternative, target);
        }
        _ => {}
    }
}

pub(crate) fn target_references_it(target: &TargetAst) -> bool {
    match target {
        TargetAst::Tagged(tag, _) => tag.as_str() == IT_TAG,
        TargetAst::Object(filter, _, _) => filter
            .tagged_constraints
            .iter()
            .any(|constraint| constraint.tag.as_str() == IT_TAG),
        TargetAst::WithCount(inner, _) => target_references_it(inner),
        _ => false,
    }
}

pub(crate) fn is_that_turn_end_step_sentence(tokens: &[Token]) -> bool {
    let clause_words = words(tokens);
    clause_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "that",
        "turn",
        "end",
        "step",
    ]) || clause_words.starts_with(&[
        "at",
        "the",
        "beginning",
        "of",
        "that",
        "turns",
        "end",
        "step",
    ])
}

pub(crate) fn most_recent_extra_turn_player(effects: &[EffectAst]) -> Option<PlayerAst> {
    effects.iter().rev().find_map(|effect| {
        if let EffectAst::ExtraTurnAfterTurn { player } = effect {
            Some(*player)
        } else {
            None
        }
    })
}

pub(crate) fn rewrite_when_you_do_clause_prefix(tokens: &[Token]) -> Vec<Token> {
    let clause_words = words(tokens);
    if clause_words.starts_with(&["when", "you", "do"]) {
        let mut rewritten = tokens.to_vec();
        for token in &mut rewritten {
            if let Token::Word(word, _) = token {
                if word.eq_ignore_ascii_case("when") {
                    *word = "if".to_string();
                }
                break;
            }
        }
        return rewritten;
    }

    // Generic "When one or more ... this way, ..." follow-ups are semantically
    // "If you do, ..." against the immediately previous effect result.
    let has_this_way = clause_words
        .windows(2)
        .any(|window| window == ["this", "way"]);
    if (clause_words.starts_with(&["when", "one", "or", "more"])
        || clause_words.starts_with(&["whenever", "one", "or", "more"]))
        && has_this_way
    {
        let Some(comma_idx) = tokens
            .iter()
            .position(|token| matches!(token, Token::Comma(_)))
        else {
            return tokens.to_vec();
        };
        let mut rewritten = Vec::new();

        let mut if_token = tokens[0].clone();
        if let Token::Word(word, _) = &mut if_token {
            *word = "if".to_string();
        }
        rewritten.push(if_token);

        let mut you_token = tokens.get(1).cloned().unwrap_or_else(|| tokens[0].clone());
        if let Token::Word(word, _) = &mut you_token {
            *word = "you".to_string();
        }
        rewritten.push(you_token);

        let mut do_token = tokens.get(2).cloned().unwrap_or_else(|| tokens[0].clone());
        if let Token::Word(word, _) = &mut do_token {
            *word = "do".to_string();
        }
        rewritten.push(do_token);

        rewritten.push(tokens[comma_idx].clone());
        rewritten.extend_from_slice(&tokens[comma_idx + 1..]);
        return rewritten;
    }

    tokens.to_vec()
}

pub(crate) fn strip_otherwise_sentence_prefix(tokens: &[Token]) -> Option<Vec<Token>> {
    if !tokens
        .first()
        .is_some_and(|token| token.is_word("otherwise"))
    {
        return None;
    }

    let mut idx = 1usize;
    while matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }
    if tokens.get(idx).is_some_and(|token| token.is_word("then")) {
        idx += 1;
    }
    while matches!(tokens.get(idx), Some(Token::Comma(_))) {
        idx += 1;
    }

    let remainder = trim_commas(&tokens[idx..]);
    if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    }
}

pub(crate) fn rewrite_otherwise_referential_subject(tokens: Vec<Token>) -> Vec<Token> {
    let clause_words = words(&tokens);
    let is_referential_get = clause_words.len() >= 3
        && clause_words[0] == "that"
        && matches!(clause_words[1], "creature" | "permanent")
        && matches!(clause_words[2], "gets" | "get" | "gains" | "gain");
    if !is_referential_get {
        return tokens;
    }

    let mut rewritten = tokens;
    if let Some(first) = rewritten.get_mut(0)
        && let Token::Word(word, _) = first
    {
        *word = "target".to_string();
    }
    rewritten
}

pub(crate) fn is_nonsemantic_restriction_sentence(tokens: &[Token]) -> bool {
    is_activate_only_restriction_sentence(tokens) || is_trigger_only_restriction_sentence(tokens)
}

pub(crate) fn try_apply_token_copy_followup(
    effects: &mut [EffectAst],
    sentence_effects: &[EffectAst],
) -> Result<bool, CardTextError> {
    if sentence_effects.len() != 1 {
        return Ok(false);
    }

    let Some(last) = effects.last_mut() else {
        return Ok(false);
    };

    let Some((haste, sacrifice, exile_next_end_step, exile_end_of_combat)) =
        (match sentence_effects.first() {
            Some(EffectAst::TokenCopyHasHaste) => Some((true, false, false, false)),
            Some(EffectAst::TokenCopySacrificeAtNextEndStep) => Some((false, true, false, false)),
            Some(EffectAst::TokenCopyExileAtNextEndStep) => Some((false, false, true, false)),
            Some(EffectAst::ExileThatTokenAtEndOfCombat) => Some((false, false, false, true)),
            _ => None,
        })
    else {
        return Ok(false);
    };

    match last {
        EffectAst::CreateTokenCopy {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        }
        | EffectAst::CreateTokenCopyFromSource {
            has_haste,
            exile_at_end_of_combat,
            sacrifice_at_next_end_step,
            exile_at_next_end_step,
            ..
        } => {
            if haste {
                *has_haste = true;
            }
            if sacrifice {
                *sacrifice_at_next_end_step = true;
            }
            if exile_next_end_step {
                *exile_at_next_end_step = true;
            }
            if exile_end_of_combat {
                *exile_at_end_of_combat = true;
            }
            Ok(true)
        }
        EffectAst::CreateTokenWithMods {
            exile_at_end_of_combat,
            ..
        } if exile_end_of_combat => {
            *exile_at_end_of_combat = true;
            Ok(true)
        }
        EffectAst::May { effects }
        | EffectAst::MayByPlayer { effects, .. }
        | EffectAst::MayByTaggedController { effects, .. }
        | EffectAst::IfResult { effects, .. } => {
            if !exile_end_of_combat {
                return Ok(false);
            }
            if effects.is_empty() {
                return Ok(false);
            }
            try_apply_token_copy_followup(effects.as_mut_slice(), sentence_effects)
        }
        _ => Ok(false),
    }
}

