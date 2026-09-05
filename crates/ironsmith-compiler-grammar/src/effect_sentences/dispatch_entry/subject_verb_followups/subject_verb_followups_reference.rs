use super::*;

pub(super) fn append_moved_object_entry_followup_to_optional_move(
    previous: &mut EffectAst,
    grant: EffectAst,
) -> bool {
    let effects = match previous {
        EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. } => effects,
        _ => return false,
    };
    let [move_effect] = effects.as_mut_slice() else {
        return false;
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::MoveToZone {
                target,
                source_top_only,
                zone,
                to_top,
                library_order,
                destination_player_surface,
                destination_player_reference_surface,
                exiled_with_source_surface,
                battlefield_tapped,
                battlefield_attacking,
                battlefield_attack_target_player_or_planeswalker_controlled_by,
                battlefield_face_down,
                battlefield_transformed,
                attached_to,
                all,
                ..
            },
        ..
    }) = move_effect
    else {
        return false;
    };
    let TargetAst::WithCount(inner, count) = target else {
        return false;
    };
    let TargetAst::Object(filter, _, _) = inner.as_ref() else {
        return false;
    };
    let exact_single_hand_object =
        *count == ChoiceCount::exactly(1) && filter.zone == Some(Zone::Hand) && !filter.source;
    let clean_battlefield_move = !*source_top_only
        && *zone == Zone::Battlefield
        && !*to_top
        && library_order.is_none()
        && destination_player_surface.is_none()
        && destination_player_reference_surface.is_none()
        && exiled_with_source_surface.is_none()
        && !*battlefield_tapped
        && !*battlefield_attacking
        && battlefield_attack_target_player_or_planeswalker_controlled_by.is_none()
        && !*battlefield_face_down
        && !*battlefield_transformed
        && attached_to.is_none()
        && !*all;
    let exact_result_grant = matches!(
        &grant,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesToTarget {
                    target: TargetAst::Tagged(tag, _),
                    abilities,
                    duration: Until::EndOfTurn,
                    condition: None,
                    set_quantifier_surface: None,
                },
            ..
        }) if tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str() && !abilities.is_empty()
    );
    if !exact_single_hand_object || !clean_battlefield_move || !exact_result_grant {
        return false;
    }

    *battlefield_tapped = true;
    *battlefield_attacking = true;
    effects.push(grant);
    true
}

pub(super) fn pre_rule_moved_object_entry_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_moved_object_entry_followup_shape(sentence_tokens)
    else {
        return Ok(None);
    };
    let Some(leading_pronoun) = sentence_tokens.first() else {
        return Ok(None);
    };
    let mut grant_tokens = Vec::with_capacity(
        sentence_tokens
            .len()
            .saturating_sub(shape.grant_verb_token_idx)
            + 1,
    );
    grant_tokens.push(leading_pronoun.clone());
    grant_tokens.extend_from_slice(&sentence_tokens[shape.grant_verb_token_idx..]);
    let Some(mut grants) =
        super::super::super::gain_ability::parse_gain_ability_sentence(&grant_tokens)?
    else {
        return Ok(None);
    };
    let [grant] = grants.as_mut_slice() else {
        return Ok(None);
    };
    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    if !append_moved_object_entry_followup_to_optional_move(previous, grant.clone()) {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some(
            "subject-verb verb=Enter subject=previous-moved-object recognizer=entry-grant-followup",
        ),
    }))
}

pub(super) fn tagged_may_battlefield_move(effect: &EffectAst) -> Option<TagKey> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    target: TargetAst::Tagged(tag, _),
                    zone: Zone::Battlefield,
                    ..
                }
                | SubjectVerbActionAst::MayMoveToZone {
                    target: TargetAst::Tagged(tag, _),
                    zone: Zone::Battlefield,
                },
            ..
        }) => Some(tag.clone()),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::MoveToZone {
                    target: TargetAst::Object(filter, _, _),
                    zone: Zone::Battlefield,
                    ..
                }
                | SubjectVerbActionAst::MayMoveToZone {
                    target: TargetAst::Object(filter, _, _),
                    zone: Zone::Battlefield,
                },
            ..
        }) => filter
            .tagged_constraints
            .iter()
            .find(|constraint| constraint.relation == TaggedOpbjectRelation::IsTaggedObject)
            .map(|constraint| constraint.tag.clone()),
        EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. }
            if effects.len() == 1 =>
        {
            tagged_may_battlefield_move(&effects[0])
        }
        _ => None,
    }
}

/// Attach "If you don't put it onto the battlefield" to the immediately
/// preceding optional tagged move.  The fallback happens both when the move's
/// object gate is false and when the player declines the move, so it must be
/// represented inside the existing conditional rather than as an unrelated
/// battlefield-state test.
pub(super) fn pre_rule_declined_tagged_battlefield_move_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some((condition_tokens, fallback_tokens)) =
        grammar::split_lexed_once_on_delimiter(sentence_tokens, TokenKind::Comma)
    else {
        return Ok(None);
    };
    let condition_words = crate::lexer::token_word_refs(condition_tokens);
    if condition_words.len() < 7
        || condition_words.first().copied() != Some("if")
        || condition_words.get(1).copied() != Some("you")
        || !condition_words
            .get(2)
            .is_some_and(|word| matches!(*word, "dont" | "don't"))
        || condition_words.get(3).copied() != Some("put")
        || !crate::word_primitives::any_sequence_occurs(
            &condition_words,
            &[&["onto", "battlefield"], &["onto", "the", "battlefield"]],
        )
    {
        return Ok(None);
    }

    let Some(tag) = state.effects.last().and_then(|effect| match effect {
        EffectAst::Conditional {
            if_true, if_false, ..
        } if if_false.is_empty() && if_true.len() == 1 => tagged_may_battlefield_move(&if_true[0]),
        EffectAst::TrailingIf { effects, .. } if effects.len() == 1 => {
            tagged_may_battlefield_move(&effects[0])
        }
        _ => None,
    }) else {
        return Ok(None);
    };

    let fallback_tokens = trim_commas(fallback_tokens);
    let mut fallback = parse_effect_sentence_lexed(&fallback_tokens)?;
    if fallback.is_empty() {
        return Ok(None);
    }
    let explicit_target = TargetAst::Tagged(tag, span_from_tokens(condition_tokens));
    replace_it_target_in_effects(&mut fallback, &explicit_target);

    if let Some(EffectAst::TrailingIf { predicate, effects }) = state.effects.last_mut() {
        let predicate = predicate.clone();
        let if_true = std::mem::take(effects);
        *state.effects.last_mut().expect("trailing-if still present") = EffectAst::Conditional {
            predicate,
            if_true,
            if_false: Vec::new(),
        };
    }
    let Some(EffectAst::Conditional {
        if_true, if_false, ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };
    if_true.push(EffectAst::IfResult {
        predicate: IfResultPredicate::WasDeclined,
        effects: fallback.clone(),
    });
    *if_false = fallback;
    *state.carried_context = None;

    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some(
            "subject-verb verb=Put subject=implicit recognizer=declined-tagged-move-followup",
        ),
    }))
}

pub(super) fn last_remove_abilities_all_filter(effects: &[EffectAst]) -> Option<ObjectFilter> {
    effects.iter().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. },
            ..
        }) => Some(filter.clone()),
        _ => None,
    })
}

pub(super) fn rebind_source_match_to_target(predicate: PredicateAst) -> PredicateAst {
    match predicate {
        PredicateAst::SourceMatches(filter) | PredicateAst::ItMatches(filter) => {
            PredicateAst::TargetMatches(filter)
        }
        PredicateAst::Not(inner) => {
            PredicateAst::Not(Box::new(rebind_source_match_to_target(*inner)))
        }
        PredicateAst::And(left, right) => PredicateAst::And(
            Box::new(rebind_source_match_to_target(*left)),
            Box::new(rebind_source_match_to_target(*right)),
        ),
        PredicateAst::Or(left, right) => PredicateAst::Or(
            Box::new(rebind_source_match_to_target(*left)),
            Box::new(rebind_source_match_to_target(*right)),
        ),
        other => other,
    }
}

pub(super) fn target_is_explicitly_a_land(target: &TargetAst) -> bool {
    match target {
        TargetAst::Object(filter, _, _) | TargetAst::ObjectOrPlayer(filter, _, _) => {
            filter.card_types.contains(&crate::CardType::Land)
                || filter
                    .subtypes
                    .iter()
                    .any(crate::Subtype::is_basic_land_type)
        }
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_is_explicitly_a_land(inner)
        }
        _ => false,
    }
}

/// The standalone condition parser conservatively treats a bare `it has
/// <keyword>` clause as a source predicate. In an `instead` follow-up after an
/// explicit target, however, that pronoun repeats the targeted object from the
/// default action. Preserve that local antecedent before lowering turns the
/// replacement into a runtime self-replacement branch.
pub(super) fn bind_self_replacement_condition_to_previous_target(
    predicate: PredicateAst,
    sentence_tokens: &[OwnedLexToken],
    previous_target: Option<&TargetAst>,
) -> PredicateAst {
    let words = LexedClause::new(sentence_tokens).word_refs();
    let has_local_it_condition = crate::word_primitives::any_sequence_occurs(
        &words,
        &[
            &["if", "it"],
            &["if", "its"],
            &["if", "it's"],
            &["if", "that"],
            &["if", "those"],
        ],
    );
    if !has_local_it_condition || !previous_target.is_some_and(target_is_explicitly_chosen) {
        return predicate;
    }
    // A typed demonstrative can point past a later action target to an
    // earlier event subject. Landfall's "that land" is the canonical case:
    // Akoum Hellkite damages any target and Emeria Shepherd targets a nonland
    // graveyard card, but their replacement gate must inspect the land that
    // triggered the ability. Only bind an explicit "that land" locally when
    // the previous target was itself authored as a land target.
    if predicate_explicitly_says_that_land(&predicate)
        && !previous_target.is_some_and(target_is_explicitly_a_land)
    {
        return bind_demonstrative_land_match_to_triggering_object(predicate);
    }
    rebind_source_match_to_target(predicate)
}

/// Rebind the characteristic gate inside a replacement action when the
/// authored clause repeats the action target with a local `if it ...`.
///
/// In `destroy that creature if it has mana value ... instead if ...`, the
/// final condition is the self-replacement gate while the earlier `if it`
/// remains nested around the replacement action. The ordinary predicate
/// parser conservatively emits `SourceMatches`; once the prior announced
/// target and the local pronoun are both proven, move only that nested gate to
/// `TargetMatches` alongside the action-target rebinding.
pub(super) fn bind_nested_self_replacement_condition_to_previous_target(
    effects: &mut [EffectAst],
    sentence_tokens: &[OwnedLexToken],
    previous_target: Option<&TargetAst>,
) {
    let words = LexedClause::new(sentence_tokens).word_refs();
    let has_local_it_condition = crate::word_primitives::any_sequence_occurs(
        &words,
        &[&["if", "it"], &["if", "its"], &["if", "it's"]],
    );
    if !has_local_it_condition || !previous_target.is_some_and(target_is_explicitly_chosen) {
        return;
    }

    fn rebind_first(effects: &mut [EffectAst]) -> bool {
        for effect in effects {
            match effect {
                EffectAst::Conditional {
                    predicate,
                    if_true,
                    if_false,
                } => {
                    if matches!(
                        predicate,
                        PredicateAst::SourceMatches(_) | PredicateAst::ItMatches(_)
                    ) && !if_true.is_empty()
                    {
                        *predicate = rebind_source_match_to_target(predicate.clone());
                        return true;
                    }
                    if rebind_first(if_true) || rebind_first(if_false) {
                        return true;
                    }
                }
                EffectAst::TrailingIf { predicate, effects } => {
                    if matches!(
                        predicate,
                        PredicateAst::SourceMatches(_) | PredicateAst::ItMatches(_)
                    ) && !effects.is_empty()
                    {
                        *predicate = rebind_source_match_to_target(predicate.clone());
                        return true;
                    }
                    if rebind_first(effects) {
                        return true;
                    }
                }
                EffectAst::ControlFlow(control) => {
                    if let crate::model::control_flow::ControlFlowNodeAst::Condition {
                        condition,
                        ..
                    } = &mut control.node
                        && let crate::model::control_flow::ControlPredicateAst::State(predicate) =
                            &mut condition.predicate
                        && matches!(
                            predicate,
                            PredicateAst::SourceMatches(_) | PredicateAst::ItMatches(_)
                        )
                    {
                        *predicate = rebind_source_match_to_target(predicate.clone());
                        return true;
                    }
                    let mut changed = false;
                    crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                        if !changed {
                            changed = rebind_first(nested);
                        }
                    });
                    if changed {
                        return true;
                    }
                }
                _ => {
                    let mut changed = false;
                    crate::model::visit::for_each_nested_effects_mut(effect, true, |nested| {
                        if !changed {
                            changed = rebind_first(nested);
                        }
                    });
                    if changed {
                        return true;
                    }
                }
            }
        }
        false
    }

    let _ = rebind_first(effects);
}

/// Correlate a physical coin face with the player who flipped it. A called
/// coin flip models win/loss and is not equivalent: a player may call tails.
/// Rewriting the antecedent to the face-only producer makes its per-player
/// result count `1` for heads and `0` for tails, which the existing
/// `ForEachPlayerDid` lowering can consume without losing player identity.
pub(super) fn post_rule_each_player_coin_face_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    let result_predicate = match words.get(..7) {
        Some(["each", "player", "whose", "coin", "comes", "up", "heads"]) => IfResultPredicate::Did,
        Some(["each", "player", "whose", "coin", "comes", "up", "tails"]) => {
            IfResultPredicate::DidNot
        }
        _ => return Ok(None),
    };

    let Some(EffectAst::ForEachPlayer {
        effects: flip_effects,
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };
    let [EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. })] = flip_effects.as_mut_slice()
    else {
        return Ok(None);
    };
    if !matches!(action, SubjectVerbActionAst::FlipCoin) {
        return Ok(None);
    }

    let [
        EffectAst::ForEachPlayer {
            effects: followup_effects,
        },
    ] = sentence_effects.as_slice()
    else {
        return Ok(None);
    };
    if followup_effects.is_empty() {
        return Ok(None);
    }

    *action = SubjectVerbActionAst::FlipCoinFaceOnly;
    *sentence_effects = vec![EffectAst::ForEachPlayerDid {
        effects: followup_effects.clone(),
        predicate: None,
        result_predicate,
    }];
    Ok(Some(PostParseFollowupResult::Annotated))
}

pub(super) fn pre_rule_each_player_coin_face_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    if !matches!(
        words.get(..7),
        Some([
            "each",
            "player",
            "whose",
            "coin",
            "comes",
            "up",
            "heads" | "tails"
        ])
    ) {
        return Ok(None);
    }
    let view = crate::lexer::TokenWordView::new(sentence_tokens);
    let Some(tail_start) = view.token_index_after_words(7) else {
        return Ok(None);
    };
    let mut tokens = sentence_tokens[..2].to_vec();
    tokens.extend_from_slice(&sentence_tokens[tail_start..]);
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan::new(
        tokens,
    ))))
}

pub(super) fn carried_player_from_effect(effect: &EffectAst) -> Option<PlayerAst> {
    if let Some(CarryContext::Player(player)) = explicit_player_for_carry(effect)
        && !matches!(player, PlayerAst::That | PlayerAst::Implicit)
    {
        return Some(player);
    }

    // Sentence normalization may preserve a multi-clause default as one
    // Sequence/SourceSentence node.  A target declaration inside that node is
    // still the antecedent for a following `instead` branch, so inspect the
    // authored children from newest to oldest before concluding that the
    // replacement has no player to carry.
    let mut carried = None;
    for_each_nested_effects(effect, true, |nested| {
        if carried.is_none() {
            carried = nested.iter().rev().find_map(carried_player_from_effect);
        }
    });
    carried
}

pub(super) fn effect_has_that_player_subject(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst {
                player: PlayerAst::That,
                ..
            },
            ..
        })
    ) {
        return true;
    }

    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_has_that_player_subject);
        }
    });
    found
}

pub(super) fn bind_that_player_subjects(effect: &mut EffectAst, player: PlayerAst) {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, action }) = effect {
        if subject.player == PlayerAst::That {
            subject.player = player;
        }
        if let SubjectVerbActionAst::SearchLibrary {
            filter,
            player: library_owner,
            ..
        } = action
            && *library_owner == PlayerAst::That
            && filter.owner.is_none()
        {
            // SearchLibrary deliberately models the search actor (`chooser`)
            // separately from the library owner.  Rebinding only the generic
            // SubjectVerb subject therefore leaves "that player's library"
            // unbound when an `instead` branch is lowered independently.
            // Carry the established target into the owner slot without
            // changing an omitted/imperative chooser into that target.
            *library_owner = player;
        }
    }

    for_each_nested_effects_mut(effect, true, |nested| {
        for nested_effect in nested {
            bind_that_player_subjects(nested_effect, player);
        }
    });
}

pub(super) fn bind_that_player_subjects_in_effects(effects: &mut [EffectAst], player: PlayerAst) {
    for effect in effects {
        bind_that_player_subjects(effect, player);
    }
}

pub(super) fn tagged_object_reference(filter: &ObjectFilter) -> Option<&TagKey> {
    let [constraint] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    (constraint.relation == TaggedOpbjectRelation::IsTaggedObject).then_some(&constraint.tag)
}

pub(super) fn tag_latest_prior_exile(effects: &mut [EffectAst]) -> bool {
    let Some(exile_idx) = crate::slice_primitives::select_last_position(effects, |effect| {
        matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::Exile { .. },
                ..
            })
        )
    }) else {
        return false;
    };
    let prior = effects[exile_idx].clone();
    effects[exile_idx] = EffectAst::TagAffected {
        effect: Box::new(prior),
        tag: crate::tag::CompilerReferenceTag::PriorExiledCard.bind(),
    };
    for effect in &mut effects[exile_idx + 1..] {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PumpAll { power, .. },
            ..
        }) = effect
        {
            bind_prior_exiled_mana_value(power);
        }
    }
    true
}

pub(super) fn post_rule_consult_remainder_reference(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    fn consult_tags(effect: &EffectAst) -> Option<(TagKey, TagKey, PlayerAst)> {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::ConsultTopOfLibrary {
                    all_tag,
                    match_tag,
                    player,
                    ..
                },
            ..
        }) = effect
        {
            return Some((all_tag.clone(), match_tag.clone(), *player));
        }
        let mut found = None;
        for_each_nested_effects(effect, true, |nested| {
            if found.is_none() {
                found = nested.iter().rev().find_map(consult_tags);
            }
        });
        found
    }

    let Some((all_tag, match_tag, consult_player)) =
        state.effects.iter().rev().find_map(consult_tags)
    else {
        return Ok(None);
    };

    fn rewrite(
        effect: &mut EffectAst,
        all_tag: &TagKey,
        match_tag: &TagKey,
        consult_player: PlayerAst,
    ) {
        let EffectAst::SubjectVerb(subject_verb) = effect else {
            for_each_nested_effects_mut(effect, true, |nested| {
                for child in nested {
                    rewrite(child, all_tag, match_tag, consult_player);
                }
            });
            return;
        };
        let SubjectVerbActionAst::MoveToZone {
            target: TargetAst::Tagged(tag, _),
            zone,
            library_order,
            library_order_chooser,
            ..
        } = &subject_verb.action
        else {
            return;
        };
        if tag.as_str() != crate::tag::CompilerReferenceTag::Rest.as_str() {
            return;
        }
        subject_verb.action = if *zone == Zone::Library {
            let Some(order) = *library_order else {
                return;
            };
            SubjectVerbActionAst::PutTaggedRemainderOnBottomOfLibrary {
                tag: all_tag.clone(),
                keep_tagged: Some(match_tag.clone()),
                order,
                player: if matches!(library_order_chooser, PlayerAst::Implicit) {
                    consult_player
                } else {
                    *library_order_chooser
                },
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            }
        } else {
            SubjectVerbActionAst::PutTaggedRemainderInZone {
                tag: all_tag.clone(),
                keep_tagged: match_tag.clone(),
                zone: *zone,
                surface: ironsmith_core::LibraryRemainderSurface::Rest,
            }
        };
    }
    for effect in sentence_effects {
        rewrite(effect, &all_tag, &match_tag, consult_player);
    }
    Ok(Some(PostParseFollowupResult::Annotated))
}

pub(super) fn bind_targeted_leaves_filter(
    trigger: &mut crate::cards::builders::TriggerSpec,
    tag: &TagKey,
) -> bool {
    match trigger {
        crate::cards::builders::TriggerSpec::WithIntro { trigger, .. } => {
            bind_targeted_leaves_filter(trigger, tag)
        }
        crate::cards::builders::TriggerSpec::LeavesBattlefield(filter) => {
            *filter = filter
                .clone()
                .match_tagged(tag.clone(), TaggedOpbjectRelation::IsTaggedObject);
            true
        }
        _ => false,
    }
}

pub(super) fn effects_are_copy_retarget_followup(effects: &[EffectAst]) -> bool {
    fn contains_retarget(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RetargetStackObject {
                    target: TargetAst::Tagged(tag, _),
                    ..
                },
                ..
            }) if tag.as_str() == crate::tag::CompilerReferenceTag::CopiedStackObject.as_str()
        ) {
            return true;
        }
        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(contains_retarget);
        });
        found
    }

    effects.iter().any(contains_retarget)
}

pub(super) fn effects_are_one_copy_retarget_followup(effects: &[EffectAst]) -> bool {
    fn is_one_retarget(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RetargetStackObject {
                    target: TargetAst::Tagged(tag, _),
                    ..
                },
                ..
            }) if tag.as_str() == crate::tag::CompilerReferenceTag::CopiedStackObject.as_str()
        ) {
            return true;
        }
        match effect {
            EffectAst::SourceSentence { effects, .. }
            | EffectAst::Sequence { effects }
            | EffectAst::CommaThen { effects }
            | EffectAst::Coordinated { effects, .. }
            | EffectAst::May { effects }
            | EffectAst::MayByPlayer { effects, .. } => {
                matches!(effects.as_slice(), [effect] if is_one_retarget(effect))
            }
            _ => false,
        }
    }

    matches!(effects, [effect] if is_one_retarget(effect))
}

pub(super) fn effects_copy_a_stack_object(effects: &[EffectAst]) -> bool {
    fn contains_copy(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CopySpell { .. }
                    | SubjectVerbActionAst::CopySpellForEachTarget { .. },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(contains_copy);
        });
        found
    }

    effects.iter().any(contains_copy)
}

pub(super) fn append_copy_retarget_to_trailing_optional_copy(
    previous: &mut EffectAst,
    followups: &mut Vec<EffectAst>,
) -> bool {
    if !effects_are_one_copy_retarget_followup(followups) {
        return false;
    }
    let Some(optional_effects) = trailing_optional_copy_effects_mut(previous) else {
        return false;
    };
    optional_effects.append(followups);
    true
}

/// A fixed target assignment for "the copy" belongs to the same optional
/// procedure that creates that copy. As an outer sibling it would execute
/// even after the player declined to copy the spell or ability.
pub(in super::super) fn transport_copy_retarget_into_trailing_optional_copy(
    effects: &mut Vec<EffectAst>,
) {
    let mut index = 1usize;
    while index < effects.len() {
        let mut followups = vec![effects[index].clone()];
        if append_copy_retarget_to_trailing_optional_copy(&mut effects[index - 1], &mut followups) {
            effects.remove(index);
        } else {
            index += 1;
        }
    }
}

pub(super) fn post_rule_optional_copy_retarget_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    if !append_copy_retarget_to_trailing_optional_copy(previous, sentence_effects) {
        return Ok(None);
    }
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}
