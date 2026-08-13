use super::*;
use crate::ChoiceCount;
use crate::cards::builders::SubjectVerbSubjectAst;
use crate::runtime_backend::grammar::effects::followup_shapes;
use crate::runtime_backend::grammar::structure::parse_trailing_if_predicate_lexed;

pub(super) enum PreParseFollowupResult {
    Handled {
        consumed_sentences: usize,
        route: Option<&'static str>,
    },
    Plan(SentenceParsePlan),
}

pub(super) enum PostParseFollowupResult {
    Handled { consumed_sentences: usize },
}

fn effect_contains_search_library(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SearchLibrary { .. },
            ..
        })
    ) {
        return true;
    }

    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_contains_search_library);
        }
    });
    found
}

fn last_demonstrative_collection_filter(effects: &[EffectAst]) -> Option<ObjectFilter> {
    match effects.last()? {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }) => {
            let Value::Count(filter) = count.unhinted() else {
                return None;
            };
            Some(filter.clone())
        }
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::PumpAll { filter, .. },
            ..
        }) => Some(filter.clone()),
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::ScalePowerToughnessAll { filter, .. },
            ..
        }) => Some(filter.clone()),
        EffectAst::ForEachObject { filter, .. } => Some(filter.clone()),
        _ => None,
    }
}

fn starts_with_demonstrative_object_gain(words: &[&str]) -> bool {
    let subject_is_demonstrative = words
        .first()
        .is_some_and(|word| matches!(*word, "that" | "those"))
        || (words.len() >= 3
            && matches!(words[0], "each" | "all")
            && words[1] == "of"
            && matches!(words[2], "that" | "those"));
    subject_is_demonstrative && words.iter().any(|word| matches!(*word, "gain" | "gains"))
}

fn build_grant_all_from_demonstrative_gain(
    filter: ObjectFilter,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let mut parsed = parse_effect_sentence_lexed(sentence_tokens)?;
    let [effect] = parsed.as_mut_slice() else {
        return Ok(None);
    };
    let EffectAst::SubjectVerb(SubjectVerbEffectAst { action, .. }) = effect else {
        return Ok(None);
    };
    let (abilities, duration, condition, parsed_set_quantifier_surface) = match action.clone() {
        SubjectVerbActionAst::GrantAbilitiesToTarget {
            abilities,
            duration,
            condition,
            set_quantifier_surface,
            ..
        }
        | SubjectVerbActionAst::GrantAbilitiesAll {
            abilities,
            duration,
            condition,
            set_quantifier_surface,
            ..
        } => (abilities, duration, condition, set_quantifier_surface),
        _ => return Ok(None),
    };
    let authored_words = LexedClause::new(sentence_tokens).word_refs();
    let authored_set_quantifier_surface = (authored_words.first() == Some(&"those")
        || authored_words.starts_with(&["each", "of", "those"])
        || authored_words.starts_with(&["all", "of", "those"]))
    .then_some(ironsmith_core::SetQuantifierSurface::Those);
    let set_quantifier_surface = parsed_set_quantifier_surface.or(authored_set_quantifier_surface);
    let mut rebuilt = if let Some(condition) = condition {
        EffectAst::subject_verb_grant_abilities_all_with_condition(
            filter, abilities, duration, condition,
        )
    } else {
        EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration)
    };
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::GrantAbilitiesAll {
                set_quantifier_surface: rebuilt_surface,
                ..
            },
        ..
    }) = &mut rebuilt
    {
        *rebuilt_surface = set_quantifier_surface;
    }
    Ok(Some(rebuilt))
}

#[cfg(test)]
mod demonstrative_grant_surface_tests {
    use super::*;

    #[test]
    fn rebuilding_a_demonstrative_grant_keeps_its_those_surface() {
        let tokens = crate::runtime_backend::lex_line(
            "Those creatures gain vigilance until end of turn.",
            0,
        )
        .expect("demonstrative grant should lex");
        let antecedent = ObjectFilter::creature().you_control().other();
        let rebuilt = build_grant_all_from_demonstrative_gain(antecedent.clone(), &tokens)
            .expect("demonstrative grant should parse")
            .expect("demonstrative grant should rebuild against its antecedent");

        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::GrantAbilitiesAll {
                    filter,
                    set_quantifier_surface,
                    ..
                },
            ..
        }) = rebuilt
        else {
            panic!("expected a filter-wide grant");
        };
        assert_eq!(filter, antecedent);
        assert_eq!(
            set_quantifier_surface,
            Some(ironsmith_core::SetQuantifierSurface::Those)
        );
    }
}

fn effect_needs_followup_library_shuffle(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::ChooseObjectsAcrossZones { zones, .. }
            if zones.iter().any(|zone| zone == &Zone::Library)
    ) {
        return true;
    }

    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_needs_followup_library_shuffle);
        }
    });
    found
}

fn shuffle_library_if_previous_effect_happened() -> EffectAst {
    EffectAst::IfResult {
        predicate: IfResultPredicate::SearchedLibrary,
        effects: vec![EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::You,
            SubjectVerbActionAst::ShuffleLibrary,
        )],
    }
}

fn append_library_shuffle_followup_to_latest_search(effects: &mut Vec<EffectAst>) -> bool {
    let Some(last) = effects.last_mut() else {
        return false;
    };

    if let EffectAst::VoteOption { effects, .. } = last
        && effects.iter().any(effect_contains_search_library)
    {
        effects.push(shuffle_library_if_previous_effect_happened());
        return true;
    }

    if effect_contains_search_library(last) {
        effects.push(shuffle_library_if_previous_effect_happened());
        return true;
    }

    false
}

fn is_if_you_search_library_this_way_shuffle_sentence(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        followup_shapes::parse_library_shuffle_followup_shape(tokens),
        Some(followup_shapes::LibraryShuffleFollowupShape::IfSearchedThisWay)
    )
}

fn is_then_that_player_shuffles_sentence(tokens: &[OwnedLexToken]) -> bool {
    matches!(
        followup_shapes::parse_library_shuffle_followup_shape(tokens),
        Some(followup_shapes::LibraryShuffleFollowupShape::ThatPlayer)
    )
}

fn is_if_you_do_return_source_exiled_cards_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    if words.len() < 7 || words.get(0..5) != Some(&["if", "you", "do", "return", "those"]) {
        return false;
    }
    if words.get(5) != Some(&"cards") || words.get(6) != Some(&"to") {
        return false;
    }
    words.iter().any(|word| *word == "battlefield")
        && words
            .iter()
            .any(|word| matches!(*word, "owner" | "owners" | "owner's" | "owners'"))
        && words.iter().any(|word| *word == "control")
}

fn sacrifice_effect_targets_tagged_it(effect: &EffectAst) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Sacrifice {
                filter,
                count,
                target,
                one_of_referenced_set,
            },
        ..
    }) = effect
    else {
        return false;
    };
    *count == 1
        && !*one_of_referenced_set
        && target.is_none()
        && filter.tagged_constraints.len() == 1
        && filter.tagged_constraints[0].tag.as_str() == crate::cards::builders::IT_TAG
        && filter.tagged_constraints[0].relation == TaggedOpbjectRelation::IsTaggedObject
}

fn sacrifice_effect_targets_source(effect: &EffectAst) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::Sacrifice {
                filter,
                count,
                target,
                ..
            },
        ..
    }) = effect
    else {
        return false;
    };
    *count == 1 && target.is_none() && filter.source
}

fn pre_rule_library_shuffle_followups(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if is_if_you_search_library_this_way_shuffle_sentence(sentence_tokens) {
        if state
            .effects
            .iter()
            .any(effect_needs_followup_library_shuffle)
        {
            state
                .effects
                .push(shuffle_library_if_previous_effect_happened());
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: None,
            }));
        }
        if append_library_shuffle_followup_to_latest_search(state.effects) {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: None,
            }));
        }
    }

    if is_then_that_player_shuffles_sentence(sentence_tokens)
        && state.effects.iter().any(effect_contains_search_library)
    {
        state.effects.push(EffectAst::subject_verb(
            SubjectVerbRoleAst::LibraryOwner,
            PlayerAst::That,
            SubjectVerbActionAst::ShuffleLibrary,
        ));
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }

    Ok(None)
}

/// Parse the optional source-exile plus collect-evidence procedure as one
/// correlated result. The evidence cards are a real graveyard selection with
/// an aggregate lower bound, and the `if you do` return is linked to the exact
/// source object moved to exile by the optional branch.
fn pre_rule_optional_source_exile_and_collect_evidence(
    _state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
    let expected_head = ["you", "may", "exile", "it", "and", "collect", "evidence"];
    if words.len() != 8
        || !words[..7]
            .iter()
            .zip(expected_head)
            .all(|(word, expected)| word.eq_ignore_ascii_case(expected))
    {
        return Ok(None);
    }
    let Some(amount) =
        crate::runtime_backend::front_end::shared::util::parse_number_word_u32(words[7])
    else {
        return Ok(None);
    };
    let Some(followup) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let followup_words = crate::runtime_backend::token_word_refs(followup.lexed());
    let expected_followup = [
        "if",
        "you",
        "do",
        "return",
        "this",
        "card",
        "to",
        "the",
        "battlefield",
        "tapped",
    ];
    if followup_words.len() != expected_followup.len()
        || !followup_words
            .iter()
            .zip(expected_followup)
            .all(|(word, expected)| word.eq_ignore_ascii_case(expected))
    {
        return Ok(None);
    }

    let source_exiled_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentence_tokens,
        "exiled",
    );
    let evidence_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentence_tokens,
        "evidence",
    );
    let evidence_filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .match_tagged(
            TagKey::from("triggering"),
            crate::filter::TaggedOpbjectRelation::IsNotTaggedObject,
        );
    let choose_evidence = EffectAst::ChooseObjectsWithAggregateConstraint {
        filter: evidence_filter,
        count: ChoiceCount::any_number(),
        player: PlayerAst::You,
        tag: evidence_tag.clone(),
        constraint: crate::effect::ChoiceAggregateConstraint::total_mana_value_at_least(amount),
    };
    let exile_source = EffectAst::TagAffected {
        effect: Box::new(EffectAst::subject_verb_exile(
            TargetAst::Tagged(TagKey::from("triggering"), None),
            false,
        )),
        tag: source_exiled_tag.clone(),
    };
    let collect_evidence = EffectAst::MoveTaggedGroupToZone {
        tag: evidence_tag,
        zone: Zone::Exile,
    };
    let return_source = EffectAst::subject_verb_return_to_battlefield(
        TargetAst::Tagged(source_exiled_tag, None),
        true,
        false,
        false,
        ReturnControllerAst::Preserve,
        None,
    );

    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![
            EffectAst::May {
                effects: vec![choose_evidence, exile_source, collect_evidence],
            },
            EffectAst::IfResult {
                predicate: IfResultPredicate::Did,
                effects: vec![return_source],
            },
        ]),
        consumed_sentences: 2,
    })))
}

#[cfg(test)]
mod collect_evidence_followup_tests {
    use super::*;

    #[test]
    fn optional_self_exile_collects_a_real_aggregate_evidence_set() {
        let tokens = crate::runtime_backend::lex_line(
            "You may exile it and collect evidence 4. If you do, return this card to the battlefield tapped.",
            0,
        )
        .expect("collect-evidence procedure should lex");
        let effects =
            parse_effect_sentences_lexed(&tokens).expect("collect-evidence procedure should parse");
        let [
            EffectAst::May { effects: optional },
            EffectAst::IfResult {
                effects: returned, ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected one optional procedure and linked return: {effects:#?}");
        };
        let [
            EffectAst::ChooseObjectsWithAggregateConstraint {
                filter,
                count,
                tag: evidence_tag,
                constraint,
                ..
            },
            EffectAst::TagAffected {
                tag: source_exiled_tag,
                ..
            },
            EffectAst::MoveTaggedGroupToZone {
                tag: moved_evidence_tag,
                zone: Zone::Exile,
            },
        ] = optional.as_slice()
        else {
            panic!("expected choose, source exile, and evidence exile: {optional:#?}");
        };
        assert!(count.is_any_number());
        assert_eq!(evidence_tag, moved_evidence_tag);
        assert!(matches!(
            constraint.minimum.as_ref().map(|value| value.unhinted()),
            Some(Value::Fixed(4))
        ));
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == "triggering"
                && constraint.relation == crate::filter::TaggedOpbjectRelation::IsNotTaggedObject
        }));
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::ReturnToBattlefield {
                        target,
                        tapped: true,
                        ..
                    },
                ..
            }),
        ] = returned.as_slice()
        else {
            panic!("expected a tapped return from exile: {returned:#?}");
        };
        assert!(matches!(
            target,
            TargetAst::Tagged(tag, _) if tag == source_exiled_tag
        ));
    }

    #[test]
    fn plain_optional_self_exile_does_not_gain_evidence_selection() {
        let tokens = crate::runtime_backend::lex_line(
            "You may exile it. If you do, return this card to the battlefield tapped.",
            0,
        )
        .expect("plain optional exile should lex");
        let effects =
            parse_effect_sentences_lexed(&tokens).expect("plain optional exile should parse");
        assert!(
            !format!("{effects:#?}").contains("ChooseObjectsWithAggregateConstraint"),
            "evidence must not be inferred without the keyword action: {effects:#?}"
        );
    }
}

fn pre_rule_return_source_exiled_cards_if_source_sacrificed(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !is_if_you_do_return_source_exiled_cards_sentence(sentence_tokens) {
        return Ok(None);
    }

    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    if sacrifice_effect_targets_tagged_it(previous) {
        *previous =
            EffectAst::subject_verb_sacrifice(PlayerAst::You, ObjectFilter::source(), 1, None);
    } else if !sacrifice_effect_targets_source(previous) {
        return Ok(None);
    }

    state.effects.push(EffectAst::IfResult {
        predicate: IfResultPredicate::Did,
        effects: vec![EffectAst::subject_verb_return_all_to_battlefield(
            ObjectFilter::tagged(crate::tag::SOURCE_EXILED_TAG).in_zone(Zone::Exile),
            false,
            false,
            ReturnControllerAst::Owner,
        )],
    });
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some(
            "subject-verb verb=Return subject=source-exiled recognizer=source-sacrifice-followup",
        ),
    }))
}

fn pre_rule_future_zone_replacement_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !sentence_contains(sentence_tokens, WOULD_DIE_THIS_TURN_PHRASE) {
        return Ok(None);
    }
    if !matches!(
        classify_instead_followup_tokens(sentence_tokens),
        InsteadSemantics::FutureReplacement
    ) {
        return Ok(None);
    }
    let Some(replacement) = future_zone_replacement_from_sentence_tokens(sentence_tokens) else {
        return Ok(None);
    };
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![replacement]),
        consumed_sentences: 1,
    })))
}

fn pre_rule_skip_tapped_source_turn_replacement(
    _state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !followup_shapes::is_skip_tapped_source_turn_replacement(sentence_tokens) {
        return Ok(None);
    }
    let has_untap_followup = sentences
        .get(sentence_idx + 1)
        .is_some_and(|sentence| followup_shapes::is_if_did_untap_source_followup(sentence.lexed()));
    let mut optional_skip_effects = vec![EffectAst::subject_verb_skip_turn(PlayerAst::You)];
    if has_untap_followup {
        optional_skip_effects.push(EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![EffectAst::subject_verb_untap(TargetAst::Source(None))],
        });
    }
    let if_true = vec![EffectAst::May {
        effects: optional_skip_effects,
    }];
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![EffectAst::Conditional {
            predicate: PredicateAst::SourceIsTapped,
            if_true,
            if_false: Vec::new(),
        }]),
        consumed_sentences: if has_untap_followup { 2 } else { 1 },
    })))
}

fn pre_rule_damage_this_way_player_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let direct_effects = match followup_shapes::parse_damaged_player_followup_shape(sentence_tokens)
    {
        Some(followup_shapes::DamagedPlayerFollowupShape::CantCastNoncreatureSpellsThisTurn) => {
            vec![EffectAst::ForEachTaggedPlayer {
                tag: TagKey::from("damaged_0"),
                effects: vec![EffectAst::subject_verb_cant(
                    crate::effect::Restriction::cast_spells_matching(
                        PlayerFilter::IteratedPlayer,
                        ObjectFilter::noncreature_spell(),
                    ),
                    crate::effect::Until::EndOfTurn,
                    None,
                )],
            }]
        }
        Some(followup_shapes::DamagedPlayerFollowupShape::CantGainLifeRestOfGame) => {
            vec![EffectAst::IfResult {
                predicate: IfResultPredicate::DealtDamageToPlayer,
                effects: vec![EffectAst::subject_verb_cant(
                    crate::effect::Restriction::gain_life(PlayerFilter::DamagedPlayer),
                    crate::effect::Until::Forever,
                    None,
                )],
            }]
        }
        None => return Ok(None),
    };
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(direct_effects),
        consumed_sentences: 1,
    })))
}

fn pre_rule_tap_damage_this_way_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !followup_shapes::is_tap_damaged_creatures_followup(sentence_tokens) {
        return Ok(None);
    }

    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![EffectAst::subject_verb_tap(TargetAst::Tagged(
            TagKey::from("damaged_0"),
            None,
        ))]),
        consumed_sentences: 1,
    })))
}

fn pre_rule_still_lands_followup(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let is_still_lands_followup = is_still_lands_followup_sentence(sentence_tokens);
    let previous_sentence_is_land_animation =
        previous_sentence_is_temporary_land_animation(sentences, sentence_idx);
    let marked_preceding_animation =
        is_still_lands_followup && mark_last_animation_as_still_a_land(state.effects);
    if is_still_lands_followup
        && (marked_preceding_animation || previous_sentence_is_land_animation)
    {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    Ok(None)
}

fn mark_last_animation_as_still_a_land(effects: &mut [EffectAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        if mark_animation_as_still_a_land(effect) {
            return true;
        }
    }
    false
}

fn mark_animation_as_still_a_land(effect: &mut EffectAst) -> bool {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action:
            SubjectVerbActionAst::BecomeBasePtCreature {
                preserve_other_types,
                type_retention_surface,
                ..
            },
        ..
    }) = effect
    {
        *preserve_other_types = true;
        *type_retention_surface = Some(ironsmith_core::TypeRetentionSurface::StillALand);
        return true;
    }

    let mut marked = false;
    for_each_nested_effects_mut(effect, true, |nested| {
        if !marked {
            marked = mark_last_animation_as_still_a_land(nested);
        }
    });
    marked
}

pub(super) fn is_still_lands_followup_sentence(sentence_tokens: &[OwnedLexToken]) -> bool {
    followup_shapes::is_still_land_followup(sentence_tokens)
}

pub(super) fn previous_sentence_is_temporary_land_animation(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    sentence_idx
        .checked_sub(1)
        .and_then(|idx| sentences.get(idx))
        .is_some_and(|previous_sentence| {
            followup_shapes::is_temporary_land_animation_sentence(previous_sentence.lowered())
        })
}

fn pre_rule_cant_be_regenerated_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_cant_be_regenerated_followup(sentence_tokens) else {
        return Ok(None);
    };
    let applied = if shape.subject == followup_shapes::CantBeRegeneratedSubject::They {
        apply_cant_be_regenerated_to_last_destroy_group(state.effects)
    } else {
        apply_cant_be_regenerated_to_last_destroy_effect(state.effects)
    };
    if applied {
        if shape.subject == followup_shapes::CantBeRegeneratedSubject::CreatureDestroyedThisWay {
            super::mark_last_destroy_creature_destroyed_this_way_surface(state.effects);
        }
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    if is_cant_be_regenerated_this_turn_followup_sentence(sentence_tokens)
        && apply_cant_be_regenerated_to_last_target_effect(state.effects)
    {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    Err(CardTextError::ParseError(format!(
        "unsupported standalone cant-be-regenerated clause (clause: '{}')",
        LexedClause::new(sentence_tokens).text()
    )))
}

fn pre_rule_copy_and_cast_followups(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if let Some(spec) = parse_same_sentence_copy_and_may_cast_copy(sentence_tokens)? {
        state.effects.push(build_may_cast_tagged_effect(&spec));
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }

    if sentence_idx + 1 < sentences.len() && is_simple_copy_reference_sentence(sentence_tokens) {
        let next_tokens = strip_embedded_token_rules_text(sentences[sentence_idx + 1].lexed());
        if let Some(spec) = parse_may_cast_it_sentence(&next_tokens)
            && spec.as_copy
        {
            return Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
                tokens: sentence_tokens.to_vec(),
                wrap_if_result: None,
                direct_effects: Some(vec![build_may_cast_tagged_effect(&spec)]),
                consumed_sentences: 2,
            })));
        }
    }

    if let Some(reduction) =
        crate::runtime_backend::activation_and_restrictions::parse_copy_reference_cost_reduction_sentence(
            sentence_tokens,
        )
    {
        if attach_copy_cost_reduction_to_effects(state.effects, &reduction) {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: None,
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone copy cost-reduction clause (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    }

    if let Some(spec) = parse_may_cast_it_sentence(sentence_tokens) {
        let mut cast = build_may_cast_tagged_effect(&spec);
        if spec.as_copy
            && let Some(delayed_effects) = state
                .effects
                .last_mut()
                .and_then(trailing_delayed_trigger_effects_mut)
        {
            if delayed_effects
                .iter()
                .any(effect_references_prior_exiled_card)
            {
                bind_cast_tag_to_prior_exiled_card(&mut cast);
            }
            delayed_effects.push(cast);
        } else {
            state.effects.push(cast);
        }
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }

    Ok(None)
}

fn pre_rule_damage_cant_be_prevented_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if effect_grammar::clause_dispatch_shapes::parse_direct_clause_shape(sentence_tokens)
        != Some(effect_grammar::clause_dispatch_shapes::DirectClauseShape::DamageCantBePrevented)
    {
        return Ok(None);
    }
    if !mark_last_deal_damage_unpreventable(state.effects) {
        return Err(CardTextError::ParseError(format!(
            "unpreventable-damage rider has no preceding damage effect (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    }
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

fn mark_last_deal_damage_unpreventable(effects: &mut [EffectAst]) -> bool {
    for effect in effects.iter_mut().rev() {
        if mark_deal_damage_unpreventable_in_effect(effect) {
            return true;
        }
    }
    false
}

fn mark_deal_damage_unpreventable_in_effect(effect: &mut EffectAst) -> bool {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
            SubjectVerbActionAst::DealDamage { unpreventable, .. } => {
                *unpreventable = true;
                true
            }
            SubjectVerbActionAst::DealDamageEqualToPower { unpreventable, .. } => {
                *unpreventable = true;
                true
            }
            _ => false,
        },
        _ => {
            let mut marked = false;
            for_each_nested_effects_mut(effect, true, |nested| {
                if marked {
                    return;
                }
                for nested_effect in nested.iter_mut().rev() {
                    if mark_deal_damage_unpreventable_in_effect(nested_effect) {
                        marked = true;
                        break;
                    }
                }
            });
            marked
        }
    }
}

fn has_trailing_unpreventable_damage_rider(tokens: &[OwnedLexToken]) -> bool {
    let words = LexedClause::new(tokens).word_refs();
    const RIDERS: &[&[&str]] = &[
        &["the", "damage", "cant", "be", "prevented"],
        &["the", "damage", "can't", "be", "prevented"],
        &["damage", "cant", "be", "prevented"],
        &["damage", "can't", "be", "prevented"],
        &["that", "damage", "cant", "be", "prevented"],
        &["that", "damage", "can't", "be", "prevented"],
    ];
    RIDERS.iter().any(|rider| {
        words
            .get(words.len().saturating_sub(rider.len())..)
            .is_some_and(|tail| tail == *rider)
    })
}

/// Bind a conditional entry modifier to the immediately preceding typed token
/// producer. This keeps the modifier executable: the true branch creates the
/// token tapped and attacking, while the false branch preserves the authored
/// ordinary creation. A standalone conditional sentence, or a different
/// token follow-up, deliberately stays on the ordinary parser path.
pub(super) fn is_conditional_token_entry_followup_sentence(
    sentence_tokens: &[OwnedLexToken],
) -> bool {
    if !sentence_tokens
        .first()
        .is_some_and(|token| token.is_word("if"))
    {
        return false;
    }
    let Some(comma_idx) = sentence_tokens.iter().position(OwnedLexToken::is_comma) else {
        return false;
    };
    let followup_tokens = trim_commas(&sentence_tokens[comma_idx + 1..]);
    matches!(
        parse_token_copy_followup_sentence_lexed(&followup_tokens),
        Some(TokenCopyFollowup::EnterTappedAndAttacking)
    )
}

pub(super) fn try_bind_conditional_token_entry_followup(
    effects: &mut Vec<EffectAst>,
    sentence_tokens: &[OwnedLexToken],
) -> Result<bool, CardTextError> {
    if !is_conditional_token_entry_followup_sentence(sentence_tokens) {
        return Ok(false);
    }
    let Some(comma_idx) = sentence_tokens.iter().position(OwnedLexToken::is_comma) else {
        return Ok(false);
    };
    let predicate_tokens = trim_commas(&sentence_tokens[1..comma_idx]);
    let followup_tokens = trim_commas(&sentence_tokens[comma_idx + 1..]);
    if predicate_tokens.is_empty() || followup_tokens.is_empty() {
        return Ok(false);
    }
    let followup = TokenCopyFollowup::EnterTappedAndAttacking;
    let Some(previous) = effects.last().cloned() else {
        return Ok(false);
    };
    let mut modified = vec![previous.clone()];
    if !try_apply_token_copy_followup(&mut modified, followup)? {
        return Ok(false);
    }
    let predicate = crate::runtime_backend::grammar::filters::parse_condition_predicate_lexed(
        &predicate_tokens,
    )?;

    effects.pop();
    effects.push(EffectAst::Conditional {
        predicate,
        if_true: modified,
        if_false: vec![previous],
    });
    Ok(true)
}

fn pre_rule_token_followups(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    // The generic dispatch path trims terminal punctuation before running the
    // followup registry. Quoted token rules need their closing quote and the
    // period inside that quote, so recognize and merge them from the original
    // sentence slice while leaving every other followup on the normalized
    // tokens supplied by the caller.
    let reminder_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lowered)
        .unwrap_or(sentence_tokens);
    let authored_reminder_tokens = sentences
        .get(sentence_idx)
        .map(SentenceInput::lexed)
        .unwrap_or(sentence_tokens);
    let reminder_facts = followup_shapes::token_reminder_followup_facts(reminder_tokens);
    if try_bind_conditional_token_entry_followup(state.effects, authored_reminder_tokens)? {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some(
                "subject-verb verb=Create subject=implicit recognizer=conditional-token-entry",
            ),
        }));
    }
    if let Some(followup) = parse_create_more_of_prior_tokens(sentence_tokens, state.effects) {
        if followup.instead {
            let Some(previous) = state.effects.pop() else {
                return Err(CardTextError::InvariantViolation(
                    "typed prior-token replacement lost its default effect".to_string(),
                ));
            };
            if !effect_creates_any_token(&previous) {
                state.effects.push(previous);
                return Err(CardTextError::ParseError(
                    "prior-token replacement does not immediately follow token creation"
                        .to_string(),
                ));
            }
            state.effects.push(EffectAst::SelfReplacement {
                predicate: followup.predicate,
                if_true: vec![followup.create],
                if_false: vec![previous],
                attach_to_previous_ability: false,
            });
        } else {
            state.effects.push(EffectAst::Conditional {
                predicate: followup.predicate,
                if_true: vec![followup.create],
                if_false: Vec::new(),
            });
        }
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some("subject-verb verb=Create subject=implicit recognizer=prior-token-instead"),
        }));
    }
    if is_spawn_scion_token_mana_reminder(sentence_tokens) {
        if state
            .effects
            .last()
            .is_some_and(effect_creates_eldrazi_spawn_or_scion)
        {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: None,
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone token mana reminder clause (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    }
    if let Some(effect) =
        parse_sentence_exile_that_token_when_source_leaves(sentence_tokens, state.effects)
    {
        state.effects.push(effect);
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    if let Some(effect) =
        parse_sentence_sacrifice_source_when_that_token_leaves(sentence_tokens, state.effects)
    {
        state.effects.push(effect);
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    // A copy-token lifecycle modifier is more specific than the broad token
    // reminder family. Apply it through nested source-sentence/loop wrappers
    // first; otherwise the generic reminder path sees that a token exists but
    // cannot reach the nested copy action and reports a false standalone
    // reminder error.
    let token_copy_followup = parse_token_copy_followup_sentence(sentence_tokens);
    if let Some(followup) = token_copy_followup
        && try_apply_token_copy_followup(state.effects, followup)?
    {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some(
                "subject-verb verb=Exile subject=implicit recognizer=token-copy-delayed-followup",
            ),
        }));
    }
    // A duration-scoped grant is an effect on the created objects, not part of
    // their copiable token definition. Let the typed follow-up plan bind it to
    // the preceding creation instead of folding haste into the token forever.
    let is_temporary_token_grant = matches!(
        token_copy_followup,
        Some(TokenCopyFollowup::GainHasteUntilEndOfTurn(_))
    );
    if !is_temporary_token_grant
        && crate::runtime_backend::sentences::effect_sentences::mixed_pronoun_token_rule_list(
            authored_reminder_tokens,
        )
        .is_some()
        && state.effects.last().is_some_and(effect_creates_any_token)
        && crate::runtime_backend::sentences::effect_sentences::
            attach_mixed_pronoun_token_rules_to_last_create(
                state.effects,
                authored_reminder_tokens,
            )
    {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: Some(
                "subject-verb verb=Grant subject=implicit recognizer=created-token-ability-followup",
            ),
        }));
    }
    if !is_temporary_token_grant
        && is_generic_token_reminder_sentence(reminder_tokens)
        && state.effects.last().is_some_and(effect_creates_any_token)
    {
        if append_token_reminder_to_last_create_effect(state.effects, reminder_tokens)? {
            let route = reminder_facts.lifecycle_head.then_some(
                "subject-verb verb=Exile subject=implicit recognizer=token-copy-delayed-followup",
            );
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route,
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone token reminder clause (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    }
    let parses_under_token_source_identity =
        crate::runtime_backend::util::source_reference_surface_for_words(&["this", "token"])
            .is_some();
    if !is_temporary_token_grant
        && is_generic_token_reminder_sentence(reminder_tokens)
        && !parses_under_token_source_identity
    {
        if !reminder_facts.delayed_pronoun_lifecycle && !reminder_facts.pronoun_trigger_prefix {
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                LexedClause::new(sentence_tokens).text()
            )));
        }
    }
    // Target-declaration normalization may collapse repeated `target`
    // markers into one union filter.  The authored sentence is still the
    // authoritative declaration surface, so split its independent target
    // slots before the broad normalized target parser can merge them.
    if let Some(effects) = parse_choose_target_prelude_sentence(authored_reminder_tokens)? {
        state.effects.extend(effects);
        *state.carried_context = None;
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    if let Some(followup) = token_copy_followup {
        let mut plan = SentenceParsePlan::new(sentence_tokens.to_vec());
        plan.direct_effects = Some(apply_unapplied_token_copy_followup(
            sentences[sentence_idx].lowered(),
            sentence_tokens,
            followup,
            state.effects.is_empty(),
        )?);
        return Ok(Some(PreParseFollowupResult::Plan(plan)));
    }
    if let Some(abilities) = parse_token_granted_ability_followup_sentence_lexed(reminder_tokens)? {
        let presentation = if crate::runtime_backend::grammar::token_definitions::
            token_ability_sentence_uses_gain_verb(reminder_tokens)
        {
            ironsmith_core::TokenAbilityPresentation::SeparateSentenceGain
        } else {
            ironsmith_core::TokenAbilityPresentation::SeparateSentence
        };
        if try_apply_token_granted_ability_followup(state.effects, &abilities, presentation)? {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: Some(
                    "subject-verb verb=Grant subject=implicit recognizer=created-token-ability-followup",
                ),
            }));
        }
    }
    Ok(None)
}

fn append_moved_object_entry_followup_to_optional_move(
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
        }) if tag.as_str() == IT_TAG && !abilities.is_empty()
    );
    if !exact_single_hand_object || !clean_battlefield_move || !exact_result_grant {
        return false;
    }

    *battlefield_tapped = true;
    *battlefield_attacking = true;
    effects.push(grant);
    true
}

fn pre_rule_moved_object_entry_followup(
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
    let Some(mut grants) = super::super::gain_ability::parse_gain_ability_sentence(&grant_tokens)?
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

#[cfg(test)]
mod moved_object_entry_followup_tests {
    use super::*;

    #[test]
    fn optional_single_hand_move_keeps_entry_state_and_grant_in_may_scope() {
        let tokens = crate::runtime_backend::lex_line(
            "You may put a creature card with mana value 3 or less from your hand onto the battlefield. It enters tapped and attacking and gains indestructible until end of turn.",
            0,
        )
        .expect("follow-up fixture should lex");
        let parsed = parse_effect_sentences_lexed(&tokens).expect("follow-up should parse");
        let [EffectAst::May { effects }] = parsed.as_slice() else {
            panic!("expected one optional procedure: {parsed:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::MoveToZone {
                        battlefield_tapped: true,
                        battlefield_attacking: true,
                        ..
                    },
                ..
            }),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::GrantAbilitiesToTarget {
                        target: TargetAst::Tagged(tag, _),
                        abilities,
                        duration: Until::EndOfTurn,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("entry follow-up escaped the may branch: {effects:#?}");
        };
        assert_eq!(tag.as_str(), IT_TAG);
        assert_eq!(
            abilities,
            &[GrantedAbilityAst::KeywordAction(
                KeywordAction::Indestructible
            )]
        );
    }

    #[test]
    fn entry_followup_does_not_attach_to_a_mandatory_move() {
        let mut hand_creature = ObjectFilter::creature();
        hand_creature.zone = Some(Zone::Hand);
        let mut previous = EffectAst::subject_verb_move_to_zone(
            TargetAst::WithCount(
                Box::new(TargetAst::Object(hand_creature, None, None)),
                ChoiceCount::exactly(1),
            ),
            Zone::Battlefield,
            false,
            crate::cards::builders::ReturnControllerAst::Preserve,
            false,
            None,
        );
        let grant = EffectAst::subject_verb_grant_abilities_to_target(
            TargetAst::Tagged(TagKey::from(IT_TAG), None),
            vec![GrantedAbilityAst::KeywordAction(
                KeywordAction::Indestructible,
            )],
            Until::EndOfTurn,
        );

        assert!(!append_moved_object_entry_followup_to_optional_move(
            &mut previous,
            grant
        ));
    }
}

fn pre_rule_draw_count_demonstrative_gain_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = LexedClause::new(sentence_tokens).word_refs();
    if !starts_with_demonstrative_object_gain(&words) {
        return Ok(None);
    }
    let Some(filter) = last_demonstrative_collection_filter(state.effects) else {
        return Ok(None);
    };
    let Some(effect) = build_grant_all_from_demonstrative_gain(filter, sentence_tokens)? else {
        return Ok(None);
    };
    state.effects.push(effect);
    *state.carried_context = None;
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: Some("subject-verb verb=Grant subject=demonstrative recognizer=draw-count-followup"),
    }))
}

struct PriorTokenCreateFollowup {
    predicate: PredicateAst,
    create: EffectAst,
    instead: bool,
}

fn parse_create_more_of_prior_tokens(
    sentence_tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<PriorTokenCreateFollowup> {
    let shape = followup_shapes::parse_create_more_prior_tokens(sentence_tokens)?;
    let predicate = parse_trailing_if_predicate_lexed(shape.predicate_tokens)?;
    let mut create = prior_effects.last()?.clone();
    let EffectAst::SubjectVerb(subject_verb) = &mut create else {
        return None;
    };
    let (count, previous_target) = match &mut subject_verb.action {
        SubjectVerbActionAst::CreateTokenWithMods { count, .. }
        | SubjectVerbActionAst::CreateTokenCopy { count, .. } => (count, None),
        SubjectVerbActionAst::CreateTokenCopyFromSource { source, count, .. } => {
            (count, Some(source.clone()))
        }
        _ => return None,
    };
    *count = Value::Fixed(shape.count as i32);
    let predicate = bind_self_replacement_condition_to_previous_target(
        predicate,
        shape.predicate_tokens,
        previous_target.as_ref(),
    );

    Some(PriorTokenCreateFollowup {
        predicate,
        create,
        instead: shape.instead,
    })
}

fn pre_rule_otherwise_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(without_otherwise) = strip_otherwise_sentence_prefix(sentence_tokens) else {
        return Ok(None);
    };
    let mut plan = SentenceParsePlan::new(rewrite_otherwise_referential_subject(without_otherwise));
    plan.wrap_if_result = Some(IfResultPredicate::Otherwise);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

fn is_if_card_put_into_exile_this_way_sentence(tokens: &[OwnedLexToken]) -> bool {
    let has_expected_prefix = grammar::match_word_prefix(
        tokens,
        &[
            "if", "a", "card", "is", "put", "into", "exile", "this", "way",
        ],
    )
    .is_some()
        || grammar::match_word_prefix(
            tokens,
            &["if", "card", "is", "put", "into", "exile", "this", "way"],
        )
        .is_some()
        || grammar::match_word_prefix(
            tokens,
            &[
                "if", "a", "card", "was", "put", "into", "exile", "this", "way",
            ],
        )
        .is_some()
        || grammar::match_word_prefix(
            tokens,
            &["if", "card", "was", "put", "into", "exile", "this", "way"],
        )
        .is_some();

    has_expected_prefix
}

fn pre_rule_exile_this_way_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !is_if_card_put_into_exile_this_way_sentence(sentence_tokens) {
        return Ok(None);
    }

    let Some((_before, after)) =
        grammar::split_lexed_once_on_delimiter(sentence_tokens, TokenKind::Comma)
    else {
        return Err(CardTextError::ParseError(format!(
            "missing comma after if-card-put-into-exile-this-way clause (clause: '{}')",
            LexedClause::new(sentence_tokens).text()
        )));
    };

    let mut plan = SentenceParsePlan::new(trim_commas(after).to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::Did);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

fn tagged_may_battlefield_move(effect: &EffectAst) -> Option<TagKey> {
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
fn pre_rule_declined_tagged_battlefield_move_followup(
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
    let condition_words = crate::runtime_backend::token_word_refs(condition_tokens);
    if condition_words.len() < 7
        || condition_words.first().copied() != Some("if")
        || condition_words.get(1).copied() != Some("you")
        || !condition_words
            .get(2)
            .is_some_and(|word| matches!(*word, "dont" | "don't"))
        || condition_words.get(3).copied() != Some("put")
        || !(condition_words
            .windows(2)
            .any(|window| window == ["onto", "battlefield"])
            || condition_words
                .windows(3)
                .any(|window| window == ["onto", "the", "battlefield"]))
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

fn pre_rule_when_milled_this_way_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    if shape.kind != followup_shapes::ConditionalFollowupKind::WhenMilledThisWay {
        return Ok(None);
    }
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::Did);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

fn pre_rule_if_no_one_does_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    if shape.kind != followup_shapes::ConditionalFollowupKind::IfNoOneDoes {
        return Ok(None);
    }
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::DidNot);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

fn pre_rule_if_you_win_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some(shape) = followup_shapes::parse_conditional_followup(sentence_tokens) else {
        return Ok(None);
    };
    let predicate = match shape.kind {
        followup_shapes::ConditionalFollowupKind::IfYouWinClash => IfResultPredicate::WonClash,
        followup_shapes::ConditionalFollowupKind::IfYouWinFlip => IfResultPredicate::Did,
        followup_shapes::ConditionalFollowupKind::IfYouWin => {
            let preceded_by_clash = state.effects.last().is_some_and(|effect| {
                terminal_result_producer(effect) == Some(TerminalResultProducer::Clash)
            });
            if preceded_by_clash {
                IfResultPredicate::WonClash
            } else {
                IfResultPredicate::Did
            }
        }
        _ => return Ok(None),
    };
    let mut plan = SentenceParsePlan::new(trim_commas(shape.continuation_tokens).to_vec());
    plan.wrap_if_result = Some(predicate);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

fn rewrite_each_player_choice_complement_chooser(effect: &mut EffectAst) -> bool {
    let EffectAst::ForEachPlayer { effects } = effect else {
        return false;
    };
    let Some((sacrifice, choices)) = effects.split_last_mut() else {
        return false;
    };
    if choices.is_empty() {
        return false;
    }

    let mut keep_tag = None::<TagKey>;
    for choice in choices {
        let EffectAst::ChooseObjects {
            filter,
            player,
            tag,
            ..
        } = choice
        else {
            return false;
        };
        if *player != PlayerAst::That
            || filter.zone != Some(Zone::Battlefield)
            || filter.controller != Some(PlayerFilter::IteratedPlayer)
        {
            return false;
        }
        if let Some(expected) = keep_tag.as_ref() {
            if expected != tag {
                return false;
            }
        } else {
            keep_tag = Some(tag.clone());
        }
    }
    let Some(keep_tag) = keep_tag else {
        return false;
    };
    let valid_complement = matches!(
        sacrifice,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player: PlayerAst::That, .. },
            action: SubjectVerbActionAst::SacrificeAll { filter },
            ..
        }) if filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == keep_tag
                && constraint.relation == TaggedOpbjectRelation::IsNotTaggedObject
        })
    );
    if !valid_complement {
        return false;
    }

    let choice_count = effects.len() - 1;
    for choice in effects.iter_mut().take(choice_count) {
        let EffectAst::ChooseObjects { player, .. } = choice else {
            unreachable!("choice-complement shape was validated above");
        };
        *player = PlayerAst::You;
    }
    true
}

/// Materialize a chooser-only self replacement for a preceding per-player
/// choose-and-sacrifice-complement procedure. The replacement changes who
/// makes each selection; the iterated player's eligible permanents and their
/// sacrifice of the unchosen remainder remain unchanged.
fn pre_rule_choose_for_each_player_instead(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let Some((condition_tokens, replacement_tokens)) =
        grammar::split_lexed_once_on_delimiter(sentence_tokens, TokenKind::Comma)
    else {
        return Ok(None);
    };
    let replacement_words = crate::runtime_backend::token_word_refs(replacement_tokens);
    if !matches!(
        replacement_words.as_slice(),
        [
            "you",
            "choose",
            "the",
            "permanents",
            "for",
            "each",
            "player",
            "instead"
        ]
    ) {
        return Ok(None);
    }
    let Some(predicate) = parse_trailing_if_predicate_lexed(condition_tokens) else {
        return Ok(None);
    };
    let Some(default) = state.effects.last().cloned() else {
        return Ok(None);
    };
    let mut replacement = default.clone();
    if !rewrite_each_player_choice_complement_chooser(&mut replacement) {
        return Ok(None);
    }

    state.effects.pop();
    state.effects.push(EffectAst::SelfReplacement {
        predicate,
        if_true: vec![replacement],
        if_false: vec![default],
        attach_to_previous_ability: false,
    });
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

#[cfg(test)]
mod choose_for_each_player_instead_tests {
    use super::*;

    fn choice_players(effect: &EffectAst) -> Vec<PlayerAst> {
        let EffectAst::ForEachPlayer { effects } = effect else {
            panic!("expected per-player procedure: {effect:#?}");
        };
        effects
            .iter()
            .filter_map(|effect| match effect {
                EffectAst::ChooseObjects { player, .. } => Some(*player),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn paid_colors_can_replace_only_the_per_player_chooser() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player chooses an artifact, a creature, an enchantment, and a planeswalker from among the nonland permanents they control, then sacrifices the rest. If {B}{R} was spent to cast this spell, you choose the permanents for each player instead.",
            0,
        )
        .expect("chooser replacement should lex");
        let parsed =
            parse_effect_sentences_lexed(&tokens).expect("chooser replacement should parse");
        let [
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                ..
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one self replacement: {parsed:#?}");
        };
        assert!(format!("{predicate:#?}").contains("Mana"), "{predicate:#?}");
        assert_eq!(choice_players(&if_true[0]), vec![PlayerAst::You; 4]);
        assert_eq!(choice_players(&if_false[0]), vec![PlayerAst::That; 4]);
    }

    #[test]
    fn chooser_rewrite_rejects_a_non_complement_procedure() {
        let mut effect = EffectAst::ForEachPlayer {
            effects: vec![EffectAst::ChooseObjects {
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::IteratedPlayer),
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::That,
                tag: TagKey::from("chosen"),
            }],
        };
        assert!(!rewrite_each_player_choice_complement_chooser(&mut effect));
        assert_eq!(choice_players(&effect), vec![PlayerAst::That]);
    }
}

fn is_destroy_those_creatures_sentence(tokens: &[OwnedLexToken]) -> bool {
    followup_shapes::is_destroy_those_creatures_followup(tokens)
}

fn last_remove_abilities_all_filter(effects: &[EffectAst]) -> Option<ObjectFilter> {
    effects.iter().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RemoveAbilitiesAll { filter, .. },
            ..
        }) => Some(filter.clone()),
        _ => None,
    })
}

fn pre_rule_destroy_those_creatures_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    if !is_destroy_those_creatures_sentence(sentence_tokens) {
        return Ok(None);
    }
    let Some(filter) = last_remove_abilities_all_filter(state.effects) else {
        return Ok(None);
    };
    state
        .effects
        .push(EffectAst::subject_verb_destroy_all(filter));
    Ok(Some(PreParseFollowupResult::Handled {
        consumed_sentences: 1,
        route: None,
    }))
}

fn post_rule_token_copy_and_extra_turn(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    collapse_token_copy_next_end_step_exile_followup(sentence_effects, sentence_tokens);
    collapse_token_copy_end_of_combat_exile_followup(sentence_effects, sentence_tokens);
    if is_that_turn_end_step_sentence(sentence_tokens)
        && let Some(extra_turn_player) = most_recent_extra_turn_player(state.effects)
        && !sentence_effects.is_empty()
    {
        // The leading delayed-schedule grammar already recognizes
        // "that turn's end step". Rebind its anaphoric player to the extra
        // turn we just parsed instead of wrapping the schedule a second time.
        // A second wrapper would register a delayed trigger whose payload is
        // another identical delayed trigger.
        if let [EffectAst::DelayedUntilEndStepOfExtraTurn { player, .. }] =
            sentence_effects.as_mut_slice()
        {
            *player = extra_turn_player;
        } else {
            // Older/narrower sentence routes can still surface this wording as
            // a plain next-end-step wrapper. Preserve only its payload when
            // specializing it to the preceding extra turn.
            let delayed_effects = if matches!(
                sentence_effects.as_slice(),
                [EffectAst::DelayedUntilNextEndStep { .. }]
            ) {
                match sentence_effects.pop().expect("matched one delayed effect") {
                    EffectAst::DelayedUntilNextEndStep { effects, .. } => effects,
                    _ => unreachable!("matched delayed-next-end-step effect"),
                }
            } else {
                std::mem::take(sentence_effects)
            };
            sentence_effects.push(EffectAst::DelayedUntilEndStepOfExtraTurn {
                player: extra_turn_player,
                effects: delayed_effects,
            });
        }
    }
    Ok(None)
}

fn primary_damage_source_from_effect(effect: &EffectAst) -> Option<TargetAst> {
    match effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DealDamage { .. } => Some(TargetAst::Source(None)),
            SubjectVerbActionAst::DealDamageEqualToPower { source, .. }
            | SubjectVerbActionAst::DealDistributedDamage { source, .. } => Some(source.clone()),
            _ => None,
        },
        _ => {
            let mut found = None;
            for_each_nested_effects(effect, false, |nested| {
                if found.is_none() {
                    found = nested.iter().find_map(primary_damage_source_from_effect);
                }
            });
            found
        }
    }
}

fn replace_anaphoric_damage_source_in_effects(effects: &mut [EffectAst], source: &TargetAst) {
    for effect in effects {
        match effect {
            EffectAst::SubjectVerb(subject_verb) => match &mut subject_verb.action {
                SubjectVerbActionAst::DealDamageEqualToPower {
                    source: effect_source,
                    ..
                }
                | SubjectVerbActionAst::DealDistributedDamage {
                    source: effect_source,
                    ..
                } if target_references_it(effect_source) => {
                    *effect_source = source.clone();
                }
                _ => {}
            },
            _ => for_each_nested_effects_mut(effect, true, |nested| {
                replace_anaphoric_damage_source_in_effects(nested, source);
            }),
        }
    }
}

fn sole_damage_payload(effects: &[EffectAst]) -> Option<(Value, bool)> {
    let [effect] = effects else {
        return None;
    };
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::DealDamage {
                    amount,
                    unpreventable,
                    ..
                }
                | SubjectVerbActionAst::DealDamageEqualToPower {
                    amount,
                    unpreventable,
                    ..
                },
            ..
        }) => Some((amount.clone(), *unpreventable)),
        EffectAst::Sequence { effects }
        | EffectAst::SourceSentence { effects, .. }
        | EffectAst::ForEachObject { effects, .. }
        | EffectAst::Coordinated { effects, .. } => sole_damage_payload(effects),
        _ => None,
    }
}

/// Collapse an authored singular damage-source anaphor before reference
/// resolution can interpret it as the most recent object result. In a
/// self-replacement, both "It" and "that creature" repeat the source and
/// target of the default damage event; neither refers to an object used to pay
/// an earlier cost.
fn normalize_anaphoric_damage_self_replacement(
    effects: &mut Vec<EffectAst>,
    tokens: &[OwnedLexToken],
    source: &TargetAst,
    target: &TargetAst,
) -> bool {
    if !effect_grammar::followup_shapes::is_anaphoric_damage_self_replacement(tokens) {
        return false;
    }
    let Some((amount, unpreventable)) = sole_damage_payload(effects) else {
        return false;
    };
    *effects = vec![EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        PlayerAst::Implicit,
        SubjectVerbActionAst::DealDamageEqualToPower {
            source: source.clone(),
            amount,
            target: target.clone(),
            unpreventable,
        },
    )];
    true
}

fn take_self_replacement_condition(
    effect: EffectAst,
) -> Option<(PredicateAst, Vec<EffectAst>, Vec<EffectAst>)> {
    match effect {
        EffectAst::Conditional {
            predicate,
            if_true,
            if_false,
        } => Some((predicate, if_true, if_false)),
        // Damage parsing preserves authored trailing condition order with a
        // typed `TrailingIf`. Once an `instead` follow-up has been classified
        // as a self-replacement, both surfaces carry the same semantic branch
        // and must be normalized before ordinary object-reference lowering.
        EffectAst::TrailingIf { predicate, effects } => Some((predicate, effects, Vec::new())),
        _ => None,
    }
}

fn first_library_search_shape(
    effects: &[EffectAst],
) -> Option<(ObjectFilter, Vec<Zone>, ChoiceCount)> {
    for effect in effects {
        if let EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            zones,
            ..
        } = effect
            && zones.contains(&Zone::Library)
        {
            return Some((filter.clone(), zones.clone(), count.clone()));
        }
        let mut nested_shape = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_shape.is_none() {
                nested_shape = first_library_search_shape(nested);
            }
        });
        if nested_shape.is_some() {
            return nested_shape;
        }
    }
    None
}

fn replace_matching_library_search_count(
    effects: &mut [EffectAst],
    replacement_filter: &ObjectFilter,
    replacement_zones: &[Zone],
    replacement_count: &ChoiceCount,
) -> bool {
    for effect in effects {
        if let EffectAst::ChooseObjectsAcrossZones {
            filter,
            count,
            zones,
            ..
        } = effect
            && zones.as_slice() == replacement_zones
            && filter == replacement_filter
        {
            *count = replacement_count.clone();
            return true;
        }
        let mut replaced = false;
        for_each_nested_effects_mut(effect, true, |nested| {
            if !replaced {
                replaced = replace_matching_library_search_count(
                    nested,
                    replacement_filter,
                    replacement_zones,
                    replacement_count,
                );
            }
        });
        if replaced {
            return true;
        }
    }
    false
}

/// A count-only search self-replacement ("search for up to three ... instead
/// of two") changes the selection count of the earlier search procedure. It
/// does not discard that procedure's reveal, split destinations, or trailing
/// shuffle. Clone the complete prior procedure into both arms and change only
/// the matching library selection in the replacement arm.
fn materialize_search_count_self_replacement(
    state_effects: &mut Vec<EffectAst>,
    predicate: PredicateAst,
    parsed_replacement: &[EffectAst],
    sentence_tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let words = LexedClause::new(sentence_tokens).word_refs();
    let count_only_surface = words.windows(2).any(|window| window == ["instead", "of"])
        && words.iter().filter(|word| **word == "search").count() == 1
        && !words
            .iter()
            .any(|word| matches!(*word, "put" | "reveal" | "shuffle" | "exile"));
    if !count_only_surface {
        return None;
    }

    let (replacement_filter, replacement_zones, replacement_count) =
        first_library_search_shape(parsed_replacement)?;
    let search_idx = state_effects.iter().rposition(|effect| {
        first_library_search_shape(std::slice::from_ref(effect)).is_some_and(
            |(filter, zones, _)| filter == replacement_filter && zones == replacement_zones,
        )
    })?;

    let default_effects = state_effects.split_off(search_idx);
    let mut replacement_effects = default_effects.clone();
    if !replace_matching_library_search_count(
        &mut replacement_effects,
        &replacement_filter,
        &replacement_zones,
        &replacement_count,
    ) {
        state_effects.extend(default_effects);
        return None;
    }

    Some(EffectAst::SelfReplacement {
        predicate,
        if_true: replacement_effects,
        if_false: default_effects,
        attach_to_previous_ability: false,
    })
}

fn target_is_explicitly_chosen(target: &TargetAst) -> bool {
    match target {
        TargetAst::AnyTarget(span)
        | TargetAst::AnyOtherTarget(span)
        | TargetAst::AttackedPlayerOrPlaneswalker(span)
        | TargetAst::Spell(span) => span.is_some(),
        TargetAst::Player(_, span)
        | TargetAst::PlayerOrPlaneswalker(_, span)
        | TargetAst::ObjectOrPlayer(_, _, span) => span.is_some(),
        TargetAst::Object(_, target_span, _) => target_span.is_some(),
        TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => {
            target_is_explicitly_chosen(inner)
        }
        TargetAst::Source(_) | TargetAst::Tagged(_, _) => false,
    }
}

fn rebind_source_match_to_target(predicate: PredicateAst) -> PredicateAst {
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

fn predicate_explicitly_says_that_land(predicate: &PredicateAst) -> bool {
    match predicate {
        PredicateAst::SourceMatches(filter)
        | PredicateAst::ItMatches(filter)
        | PredicateAst::TargetMatches(filter) => {
            filter.demonstrative_antecedent_surface()
                == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land)
        }
        PredicateAst::Not(inner) => predicate_explicitly_says_that_land(inner),
        PredicateAst::And(left, right) | PredicateAst::Or(left, right) => {
            predicate_explicitly_says_that_land(left) || predicate_explicitly_says_that_land(right)
        }
        _ => false,
    }
}

fn bind_demonstrative_land_match_to_triggering_object(predicate: PredicateAst) -> PredicateAst {
    match predicate {
        PredicateAst::SourceMatches(filter)
        | PredicateAst::ItMatches(filter)
        | PredicateAst::TargetMatches(filter)
            if filter.demonstrative_antecedent_surface()
                == Some(ironsmith_core::DemonstrativeAntecedentSurface::Land) =>
        {
            PredicateAst::TaggedMatches(crate::TagKey::from("triggering"), filter)
        }
        PredicateAst::Not(inner) => PredicateAst::Not(Box::new(
            bind_demonstrative_land_match_to_triggering_object(*inner),
        )),
        PredicateAst::And(left, right) => PredicateAst::And(
            Box::new(bind_demonstrative_land_match_to_triggering_object(*left)),
            Box::new(bind_demonstrative_land_match_to_triggering_object(*right)),
        ),
        PredicateAst::Or(left, right) => PredicateAst::Or(
            Box::new(bind_demonstrative_land_match_to_triggering_object(*left)),
            Box::new(bind_demonstrative_land_match_to_triggering_object(*right)),
        ),
        other => other,
    }
}

fn target_is_explicitly_a_land(target: &TargetAst) -> bool {
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
fn bind_self_replacement_condition_to_previous_target(
    predicate: PredicateAst,
    sentence_tokens: &[OwnedLexToken],
    previous_target: Option<&TargetAst>,
) -> PredicateAst {
    let words = LexedClause::new(sentence_tokens).word_refs();
    let has_local_it_condition = words.windows(2).any(|window| {
        window[0] == "if" && matches!(window[1], "it" | "its" | "it's" | "that" | "those")
    });
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

/// Effects authored after a self-replacement happen regardless of which arm
/// replaced the original event. Keep that common suffix in both arms so the
/// lowering boundary remains one executable self-replacement segment and
/// branch-local pronouns resolve against the object produced by that arm.
fn post_rule_self_replacement_common_suffix(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    if sentence_effects.is_empty()
        || matches!(
            classify_instead_followup_tokens(sentence_tokens),
            InsteadSemantics::SelfReplacement
        )
    {
        return Ok(None);
    }
    let Some(EffectAst::SelfReplacement {
        if_true, if_false, ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };

    let words = crate::runtime_backend::lexer::parser_token_word_refs(sentence_tokens);
    if words.starts_with(&["exile", "the", "chosen", "creature", "then"])
        && words
            .windows(5)
            .any(|window| window == ["controller", "gains", "life", "equal", "to"])
        && words.windows(2).any(|window| window == ["mana", "value"])
        && !format!("{sentence_effects:#?}").contains("GainLife")
    {
        let iterated = TagKey::from(IT_TAG);
        sentence_effects.push(EffectAst::ForEachTagged {
            tag: TagKey::from("__chosen_objects__"),
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::ItsController,
                SubjectVerbActionAst::GainLife {
                    amount: Value::ManaValueOf(Box::new(ChooseSpec::Tagged(iterated)))
                        .with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
                },
            )],
        });
    }

    if_true.extend(sentence_effects.iter().cloned());
    if_false.append(sentence_effects);
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

/// Retain an authored label inside an exact numeric-result row.
///
/// The document grammar keeps `N | ...` rows attached to the roll instruction,
/// while the ordinary statement-label parser intentionally strips a label such
/// as `Trapped! —` before parsing its executable body. Reattach that label only
/// when both pieces are still proven here: the outer typed numeric predicate and
/// the inner label/body split from the same source sentence.
fn post_rule_numeric_result_branch_label(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(prefix) =
        crate::runtime_backend::grammar::structure::split_leading_result_prefix_lexed(
            sentence_tokens,
        )
    else {
        return Ok(None);
    };
    let IfResultPredicate::Value(_) = &prefix.predicate else {
        return Ok(None);
    };
    let Some(label_split) =
        crate::runtime_backend::grammar::document_shapes::parse_statement_label_split_tokens(
            prefix.trailing_tokens,
        )
    else {
        return Ok(None);
    };
    let label = crate::runtime_backend::lexer::render_token_slice(label_split.label_tokens)
        .trim()
        .to_string();
    if label.is_empty() {
        return Ok(None);
    }
    let [EffectAst::IfResult { predicate, effects }] = sentence_effects.as_mut_slice() else {
        return Ok(None);
    };
    if predicate != &prefix.predicate || effects.is_empty() {
        return Ok(None);
    }
    let nested = std::mem::take(effects);
    effects.push(EffectAst::ResultBranchLabel {
        label,
        effects: nested,
    });
    // This is a local annotation of the current sentence, not a follow-up
    // consumed into an earlier effect. Let ordinary dispatch append it.
    Ok(None)
}

#[cfg(test)]
mod numeric_result_branch_label_tests {
    use super::*;

    fn parsed_row(text: &str) -> EffectAst {
        let tokens =
            crate::runtime_backend::lex_line(text, 0).expect("numeric result row should lex");
        let mut effects =
            parse_effect_sentences_lexed(&tokens).expect("numeric result row should parse");
        assert_eq!(effects.len(), 1, "{effects:#?}");
        effects.pop().expect("one parsed row")
    }

    #[test]
    fn exact_numeric_result_row_retains_its_authored_inner_label() {
        let effect = parsed_row("1 | Trapped! — You lose 3 life.");
        let EffectAst::IfResult {
            predicate: IfResultPredicate::Value(crate::effect::Comparison::Equal(1)),
            effects,
        } = effect
        else {
            panic!("expected exact numeric result predicate: {effect:#?}");
        };
        let [EffectAst::ResultBranchLabel { label, effects }] = effects.as_slice() else {
            panic!("expected one typed labeled result body: {effects:#?}");
        };
        assert_eq!(label, "Trapped!");
        assert!(!effects.is_empty());
    }

    #[test]
    fn unlabeled_numeric_result_row_does_not_gain_a_label_wrapper() {
        let effect = parsed_row("1 | You lose 3 life.");
        let EffectAst::IfResult { effects, .. } = effect else {
            panic!("expected numeric result branch: {effect:#?}");
        };
        assert!(
            !matches!(effects.as_slice(), [EffectAst::ResultBranchLabel { .. }]),
            "{effects:#?}"
        );
    }
}

fn post_rule_future_zone_and_self_replacement(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    maybe_rewrite_future_zone_replacement_sentence(sentence_effects, sentence_tokens);
    if matches!(
        classify_instead_followup_tokens(sentence_tokens),
        InsteadSemantics::SelfReplacement
    ) && sentence_effects.len() == 1
        && !state.effects.is_empty()
        && matches!(
            sentence_effects.first(),
            Some(EffectAst::Conditional { .. } | EffectAst::TrailingIf { .. })
        )
    {
        if let Some((predicate, mut if_true, mut if_false)) = sentence_effects
            .pop()
            .and_then(take_self_replacement_condition)
        {
            if let Some(replacement) = materialize_search_count_self_replacement(
                &mut state.effects,
                predicate.clone(),
                &if_true,
                sentence_tokens,
            ) {
                state.effects.push(replacement);
                return Ok(Some(PostParseFollowupResult::Handled {
                    consumed_sentences: 1,
                }));
            }
            let Some(previous) = state.effects.pop() else {
                return Err(CardTextError::InvariantViolation(
                    "expected previous effect for 'instead' conditional rewrite".to_string(),
                ));
            };
            let previous_target = primary_target_from_effect(&previous);
            let previous_damage_target = primary_damage_target_from_effect(&previous);
            let previous_damage_source = primary_damage_source_from_effect(&previous);
            let predicate = bind_self_replacement_condition_to_previous_target(
                predicate,
                sentence_tokens,
                previous_target.as_ref(),
            );
            if has_trailing_unpreventable_damage_rider(sentence_tokens)
                && !mark_last_deal_damage_unpreventable(&mut if_true)
            {
                return Err(CardTextError::ParseError(format!(
                    "unpreventable-damage replacement rider has no damage effect (clause: '{}')",
                    LexedClause::new(sentence_tokens).text(),
                )));
            }
            let (mut default_effects, carried_player) =
                default_effects_for_self_replacement(state.effects, previous);
            if let Some(mill_count) = default_effects
                .iter()
                .rev()
                .find_map(mill_count_from_effect)
            {
                replace_mill_event_amounts_with_value(&mut if_true, &mill_count);
            }
            if let Some(player) = carried_player {
                bind_that_player_subjects_in_effects(&mut if_true, player);
            }
            preserve_search_owner_anaphor_in_self_replacement(&mut default_effects);
            preserve_search_owner_anaphor_in_self_replacement(&mut if_true);
            if let Some(owner) = first_search_library_owner(&default_effects) {
                bind_self_replacement_search_owner(&mut if_true, &owner);
            }
            if let Some(target) = previous_target.as_ref() {
                replace_it_target_in_effects(&mut if_true, target);
            }
            if let Some(target) = previous_damage_target.as_ref() {
                replace_it_damage_target_in_effects(&mut if_true, target);
                replace_placeholder_damage_target_in_effects(&mut if_true, target);
            }
            if let Some(source) = previous_damage_source.as_ref()
                && !previous_damage_target.as_ref().is_some_and(|target| {
                    normalize_anaphoric_damage_self_replacement(
                        &mut if_true,
                        sentence_tokens,
                        source,
                        target,
                    )
                })
            {
                // In an authored damage self-replacement, a leading source
                // pronoun ("It deals ... instead") repeats the source of the
                // default damage event. It must not bind to the most recent
                // object antecedent, which may come from an additional cost.
                replace_anaphoric_damage_source_in_effects(&mut if_true, source);
            }
            for effect in default_effects.into_iter().rev() {
                if_false.insert(0, effect);
            }
            state.effects.push(EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                attach_to_previous_ability: false,
            });
            return Ok(Some(PostParseFollowupResult::Handled {
                consumed_sentences: 1,
            }));
        }
    }
    Ok(None)
}

/// Preserve the exact result of a correlated plural sacrifice.
///
/// In a sequence such as "for each player, choose target permanent that
/// player controls. Those players sacrifice those permanents", the second
/// sentence is not a new sacrifice choice. It consumes the preceding target
/// set, partitioned by the iterated sacrificing player. Tag the action's
/// actual affected objects so a later "player who sacrificed a permanent
/// this way" predicate observes the sacrifice result, not merely the earlier
/// target declaration.
fn post_rule_correlated_plural_sacrifice_result(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
    if !matches!(
        words.as_slice(),
        [
            "those",
            "players",
            "sacrifice",
            "those",
            "permanents" | "creatures" | "tokens"
        ]
    ) || !matches!(state.effects.last(), Some(EffectAst::ForEachPlayer { .. }))
    {
        return Ok(None);
    }

    let [effect] = sentence_effects.as_mut_slice() else {
        return Ok(None);
    };
    let sacrifice = match effect {
        EffectAst::ForEachPlayer { effects } => {
            let [sacrifice] = effects.as_mut_slice() else {
                return Ok(None);
            };
            sacrifice
        }
        EffectAst::SubjectVerb(_) => effect,
        _ => return Ok(None),
    };
    let consumes_prior_result = matches!(
        sacrifice,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SacrificeAll { filter },
            ..
        }) if filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == IT_TAG
                && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
        })
    );
    if !consumes_prior_result {
        return Ok(None);
    }

    let result_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentence_tokens,
        "sacrificed",
    );
    let mut sacrifice = sacrifice.clone();
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, .. }) = &mut sacrifice {
        subject.player = PlayerAst::That;
    }
    let tagged = EffectAst::TagAffected {
        effect: Box::new(sacrifice),
        tag: result_tag,
    };
    *effect = EffectAst::ForEachPlayer {
        effects: vec![tagged],
    };
    Ok(None)
}

/// Correlate a physical coin face with the player who flipped it. A called
/// coin flip models win/loss and is not equivalent: a player may call tails.
/// Rewriting the antecedent to the face-only producer makes its per-player
/// result count `1` for heads and `0` for tails, which the existing
/// `ForEachPlayerDid` lowering can consume without losing player identity.
fn post_rule_each_player_coin_face_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
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
    Ok(None)
}

#[cfg(test)]
mod each_player_coin_face_followup_tests {
    use super::*;

    #[test]
    fn heads_and_tails_followups_keep_face_only_player_correlation() {
        for (face, expected_predicate) in [
            ("heads", IfResultPredicate::Did),
            ("tails", IfResultPredicate::DidNot),
        ] {
            let text = format!(
                "Each player flips a coin. Each player whose coin comes up {face} sacrifices a creature of their choice."
            );
            let tokens = crate::runtime_backend::lex_line(&text, 0)
                .expect("coin-face player sequence should lex");
            let effects = parse_effect_sentences_lexed(&tokens)
                .expect("coin-face player sequence should parse");

            let [
                EffectAst::ForEachPlayer {
                    effects: flip_effects,
                },
                EffectAst::ForEachPlayerDid {
                    result_predicate,
                    effects: followups,
                    ..
                },
            ] = effects.as_slice()
            else {
                panic!("expected correlated flip/follow-up pair: {effects:#?}");
            };
            assert!(matches!(
                flip_effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::FlipCoinFaceOnly,
                    ..
                })]
            ));
            assert_eq!(result_predicate, &expected_predicate);
            assert!(!followups.is_empty());
        }
    }
}

/// Connect a typed `for each ... sacrificed this way` iterator to the exact
/// result set of the preceding each-player sacrifice.  Wrapping the complete
/// player loop is important: one shared tag must contain every player's
/// affected objects rather than being overwritten once per player.
fn post_rule_typed_sacrificed_result_iterator(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
    if !words.starts_with(&["for", "each"])
        || !words
            .windows(3)
            .any(|window| window == ["sacrificed", "this", "way"])
    {
        return Ok(None);
    }

    let [EffectAst::ForEachTagged { tag, .. }] = sentence_effects.as_mut_slice() else {
        return Ok(None);
    };
    if tag.as_str() != IT_TAG {
        return Ok(None);
    }

    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    let is_each_player_sacrifice_all = matches!(
        previous,
        EffectAst::ForEachPlayer { effects }
            if matches!(
                effects.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::SacrificeAll { .. },
                    ..
                })]
            )
    );
    if !is_each_player_sacrifice_all {
        return Ok(None);
    }

    if let Some(previous_sentence) = sentence_idx
        .checked_sub(1)
        .and_then(|index| sentences.get(index))
        && let EffectAst::ForEachPlayer { effects } = previous
        && let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SacrificeAll { filter },
                ..
            }),
        ] = effects.as_mut_slice()
    {
        super::super::zone_handlers::preserve_terminal_nonbasic_land_union(
            previous_sentence.lexed(),
            filter,
        );
    }

    let result_tag = crate::runtime_backend::front_end::shared::util::helper_tag_for_tokens(
        sentence_tokens,
        "sacrificed",
    );
    let previous_effect = previous.clone();
    *previous = EffectAst::TagAffected {
        effect: Box::new(previous_effect),
        tag: result_tag.clone(),
    };
    *tag = result_tag;
    Ok(None)
}

/// Preserve the comparison set in "for each of those cards that has the same
/// mana value as another card revealed this way." A plain `ForEachTagged`
/// would otherwise count every revealed card, while comparing the iterated
/// card to the implicit `__it__` tag would make every card match itself.
fn post_rule_revealed_same_mana_value_as_another_iterator(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
    const PREFIX: &[&str] = &[
        "for", "each", "of", "those", "cards", "that", "has", "the", "same", "mana", "value", "as",
        "another", "card", "revealed", "this", "way",
    ];
    if !words.starts_with(PREFIX) {
        return Ok(None);
    }

    let Some(revealed_tag) = state.effects.iter().rev().find_map(|effect| match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::RevealTagged { tag },
            ..
        }) => Some(tag.clone()),
        _ => None,
    }) else {
        return Ok(None);
    };

    let conditional_effects = match sentence_effects.as_mut_slice() {
        [EffectAst::ForEachTagged { tag, effects }]
            if tag.as_str() == IT_TAG && !effects.is_empty() =>
        {
            std::mem::take(effects)
        }
        [EffectAst::RepeatEffects { count, effects }]
            if !effects.is_empty()
                && matches!(
                    count.unhinted(),
                    Value::PendingPriorEffectMetric(query)
                        if query.source == ironsmith_core::EffectMetricSource::AffectedObjects
                            && query.metric == ironsmith_core::EffectMetric::Count
                            && query.player.is_none()
                            && query.action.is_none()
                            && query.counter_type.is_none()
                            && query.filter.as_ref().is_some_and(|filter| {
                                let expected = ObjectFilter::default().match_tagged(
                                    TagKey::from(IT_TAG),
                                    crate::filter::TaggedOpbjectRelation::IsTaggedObject,
                                );
                                filter == &expected
                            })
                ) =>
        {
            std::mem::take(effects)
        }
        _ => return Ok(None),
    };
    let filter = ObjectFilter::default().match_tagged(
        revealed_tag.clone(),
        crate::filter::TaggedOpbjectRelation::SameManaValueAsAnotherTagged,
    );
    *sentence_effects = vec![EffectAst::ForEachTagged {
        tag: revealed_tag,
        effects: vec![EffectAst::TrailingIf {
            predicate: PredicateAst::ItMatches(filter),
            effects: conditional_effects,
        }],
    }];
    Ok(None)
}

#[cfg(test)]
mod revealed_same_mana_value_iterator_tests {
    use super::*;

    #[test]
    fn revealed_cards_compare_against_a_different_card_in_the_revealed_set() {
        let tokens = crate::runtime_backend::lex_line(
            "Reveal up to five nonland cards from your hand. For each of those cards that has the same mana value as another card revealed this way, create a Treasure token.",
            0,
        )
        .expect("correlated revealed-card sentence should lex");
        let effects = parse_effect_sentences_lexed(&tokens)
            .expect("correlated revealed-card sentence should parse");

        let reveal_tag = effects.iter().find_map(|effect| match effect {
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RevealTagged { tag },
                ..
            }) => Some(tag),
            _ => None,
        });
        let Some(reveal_tag) = reveal_tag else {
            panic!("expected an explicitly tagged reveal: {effects:#?}");
        };
        let Some(EffectAst::ForEachTagged {
            tag,
            effects: iterator_effects,
        }) = effects.last()
        else {
            panic!("expected a tagged revealed-card iterator: {effects:#?}");
        };
        assert_eq!(tag, reveal_tag);
        let [
            EffectAst::TrailingIf {
                predicate: PredicateAst::ItMatches(filter),
                effects: create_effects,
            },
        ] = iterator_effects.as_slice()
        else {
            panic!("expected one typed per-card condition: {iterator_effects:#?}");
        };
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag == *reveal_tag
                && constraint.relation
                    == crate::filter::TaggedOpbjectRelation::SameManaValueAsAnotherTagged
        }));
        assert!(!create_effects.is_empty());
    }

    #[test]
    fn ordinary_for_each_revealed_card_does_not_gain_a_mana_value_condition() {
        let tokens = crate::runtime_backend::lex_line(
            "Reveal up to five nonland cards from your hand. For each card revealed this way, create a Treasure token.",
            0,
        )
        .expect("ordinary revealed-card sentence should lex");
        let effects = parse_effect_sentences_lexed(&tokens)
            .expect("ordinary revealed-card sentence should parse");
        assert!(
            !format!("{effects:#?}").contains("SameManaValueAsAnotherTagged"),
            "the qualifier must not be inferred: {effects:#?}"
        );
    }
}

#[cfg(test)]
mod correlated_plural_sacrifice_result_tests {
    use super::*;

    #[test]
    fn chosen_permanents_and_sacrifice_results_keep_distinct_typed_sets() {
        let tokens = crate::runtime_backend::lex_line(
            "For each player, choose target permanent that player controls. Those players sacrifice those permanents. Each player who sacrificed a permanent this way reveals the top card of their library, then puts it onto the battlefield if it's a permanent card.",
            0,
        )
        .expect("correlated each-player sequence should lex");
        let effects = parse_effect_sentences_lexed(&tokens)
            .expect("correlated each-player sequence should parse");

        let [
            EffectAst::ForEachPlayer { .. },
            EffectAst::ForEachPlayer {
                effects: sacrifice_effects,
            },
            EffectAst::ForEachPlayerDid {
                effects: followups,
                predicate: Some(_),
                ..
            },
        ] = effects.as_slice()
        else {
            panic!("expected target, tagged sacrifice, and correlated follow-up: {effects:#?}");
        };
        let [EffectAst::TagAffected { effect, tag }] = sacrifice_effects.as_slice() else {
            panic!(
                "the plural sacrifice must export its actual result set: {sacrifice_effects:#?}"
            );
        };
        assert!(
            tag.as_str().starts_with("__sentence_helper_sacrificed_"),
            "{tag:?}"
        );
        assert!(matches!(
            effect.as_ref(),
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SacrificeAll { .. },
                ..
            })
        ));

        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RevealTop,
                ..
            }),
            EffectAst::TrailingIf {
                effects: move_effects,
                ..
            },
        ] = followups.as_slice()
        else {
            panic!("the reveal and conditional move must both survive: {followups:#?}");
        };
        assert!(matches!(
            move_effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::MoveToZone {
                    zone: Zone::Battlefield,
                    ..
                },
                ..
            })]
        ));
    }

    #[test]
    fn wave_of_vitriol_keeps_sacrificed_lands_partitioned_by_snapshot_controller() {
        let tokens = crate::runtime_backend::lex_line(
            "Each player sacrifices all artifacts, enchantments, and nonbasic lands they control. For each land sacrificed this way, its controller may search their library for a basic land card and put it onto the battlefield tapped. Then each player who searched their library this way shuffles.",
            0,
        )
        .expect("Wave of Vitriol should lex");
        let effects = parse_effect_sentences_lexed(&tokens)
            .expect("Wave of Vitriol should parse structurally");

        let [
            EffectAst::TagAffected {
                effect: sacrifice,
                tag: sacrificed_tag,
            },
            EffectAst::ForEachTagged {
                tag: iterated_tag,
                effects: land_effects,
            },
            EffectAst::ForEachPlayerDid { .. },
        ] = effects.as_slice()
        else {
            panic!(
                "expected tagged sacrifice, typed iterator, and searched-player gate: {effects:#?}"
            );
        };
        assert_eq!(sacrificed_tag, iterated_tag);
        let EffectAst::ForEachPlayer {
            effects: sacrifice_effects,
        } = sacrifice.as_ref()
        else {
            panic!("the tagged producer must remain an each-player loop: {sacrifice:#?}");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::SacrificeAll { filter: union },
                ..
            }),
        ] = sacrifice_effects.as_slice()
        else {
            panic!(
                "the each-player loop must contain one all-set sacrifice: {sacrifice_effects:#?}"
            );
        };
        assert_eq!(union.any_of.len(), 3, "{union:#?}");
        let artifact = union
            .any_of
            .iter()
            .find(|branch| branch.card_types == [crate::types::CardType::Artifact])
            .expect("artifact union arm");
        let enchantment = union
            .any_of
            .iter()
            .find(|branch| branch.card_types == [crate::types::CardType::Enchantment])
            .expect("enchantment union arm");
        let nonbasic_land = union
            .any_of
            .iter()
            .find(|branch| branch.card_types == [crate::types::CardType::Land])
            .expect("land union arm");
        assert!(artifact.excluded_supertypes.is_empty());
        assert!(enchantment.excluded_supertypes.is_empty());
        assert_eq!(
            nonbasic_land.excluded_supertypes,
            [crate::types::Supertype::Basic]
        );
        let [
            EffectAst::Conditional {
                predicate: PredicateAst::ItMatchedLastKnown(filter),
                if_true,
                if_false,
            },
        ] = land_effects.as_slice()
        else {
            panic!("sacrifice iterator must gate each LKI snapshot by type: {land_effects:#?}");
        };
        assert_eq!(filter.card_types.as_slice(), [crate::types::CardType::Land]);
        assert!(if_false.is_empty());
        assert!(if_true.iter().any(|effect| matches!(
            effect,
            EffectAst::MayByPlayer {
                player: PlayerAst::That,
                ..
            }
        )));
    }
}

fn carried_player_from_effect(effect: &EffectAst) -> Option<PlayerAst> {
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

fn effect_has_that_player_subject(effect: &EffectAst) -> bool {
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

fn bind_that_player_subjects(effect: &mut EffectAst, player: PlayerAst) {
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

fn preserve_search_owner_anaphor_in_self_replacement(effects: &mut [EffectAst]) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    filter,
                    chooser: PlayerAst::Implicit,
                    player,
                    ..
                },
            ..
        }) = effect
            && *player == PlayerAst::Target
            && matches!(
                filter.owner.as_ref(),
                Some(PlayerFilter::Target(_) | PlayerFilter::IteratedPlayer)
            )
        {
            // The owner filter retains the executable target identity. Inside
            // both arms of a self-replacement, the action surface is an
            // anaphoric reuse of that one target, not a second target choice.
            *player = PlayerAst::That;
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            preserve_search_owner_anaphor_in_self_replacement(nested);
        });
    }
}

fn first_search_library_owner(effects: &[EffectAst]) -> Option<PlayerFilter> {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::SearchLibrary { filter, .. },
            ..
        }) = effect
            && let Some(owner) = filter.owner.clone()
        {
            return Some(owner);
        }
        let mut nested_owner = None;
        for_each_nested_effects(effect, true, |nested| {
            if nested_owner.is_none() {
                nested_owner = first_search_library_owner(nested);
            }
        });
        if nested_owner.is_some() {
            return nested_owner;
        }
    }
    None
}

fn bind_self_replacement_search_owner(effects: &mut [EffectAst], established: &PlayerFilter) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action:
                SubjectVerbActionAst::SearchLibrary {
                    filter,
                    chooser: PlayerAst::Implicit,
                    player: PlayerAst::That,
                    ..
                },
            ..
        }) = effect
            && matches!(filter.owner.as_ref(), Some(PlayerFilter::IteratedPlayer))
        {
            filter.owner = Some(established.clone());
        }
        for_each_nested_effects_mut(effect, true, |nested| {
            bind_self_replacement_search_owner(nested, established);
        });
    }
}

fn bind_that_player_subjects_in_effects(effects: &mut [EffectAst], player: PlayerAst) {
    for effect in effects {
        bind_that_player_subjects(effect, player);
    }
}

fn mill_count_from_effect(effect: &EffectAst) -> Option<Value> {
    match effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mill { count },
            ..
        }) => Some(count.clone()),
        _ => None,
    }
}

fn replace_event_amount_with_value(value: &mut Value, replacement: &Value) {
    match value {
        Value::EventValue(crate::effect::EventValueSpec::Amount) => {
            *value = replacement.clone();
        }
        Value::EventValueOffset(crate::effect::EventValueSpec::Amount, offset) => {
            *value = Value::Add(
                Box::new(replacement.clone()),
                Box::new(Value::Fixed(*offset)),
            );
        }
        Value::Add(left, right) | Value::Min(left, right) => {
            replace_event_amount_with_value(left, replacement);
            replace_event_amount_with_value(right, replacement);
        }
        Value::Scaled(inner, _)
        | Value::DividedRoundedDown(inner, _)
        | Value::HalfRoundedDown(inner)
        | Value::SurfaceHinted { value: inner, .. } => {
            replace_event_amount_with_value(inner, replacement);
        }
        _ => {}
    }
}

fn replace_mill_event_amounts_with_value(effects: &mut [EffectAst], replacement: &Value) {
    for effect in effects {
        if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Mill { count },
            ..
        }) = effect
        {
            replace_event_amount_with_value(count, replacement);
        }

        for_each_nested_effects_mut(effect, true, |nested| {
            replace_mill_event_amounts_with_value(nested, replacement);
        });
    }
}

fn default_effects_for_self_replacement(
    prior_effects: &mut Vec<EffectAst>,
    previous: EffectAst,
) -> (Vec<EffectAst>, Option<PlayerAst>) {
    let mut default_effects = vec![previous];
    let mut carried_player = default_effects
        .iter()
        .rev()
        .find_map(carried_player_from_effect);

    let anchor_idx =
        if carried_player.is_none() && default_effects.iter().any(effect_has_that_player_subject) {
            let mut idx = prior_effects.len();
            let mut found = None;
            while idx > 0 {
                idx -= 1;
                if carried_player_from_effect(&prior_effects[idx]).is_some() {
                    found = Some(idx);
                    break;
                }
            }
            found
        } else {
            None
        };
    if let Some(anchor_idx) = anchor_idx {
        carried_player = carried_player_from_effect(&prior_effects[anchor_idx]);
        let mut anchored_default_effects = prior_effects.split_off(anchor_idx);
        anchored_default_effects.append(&mut default_effects);
        default_effects = anchored_default_effects;
    }

    if let Some(player) = carried_player {
        bind_that_player_subjects_in_effects(&mut default_effects, player);
    }

    (default_effects, carried_player)
}

fn tagged_object_reference(filter: &ObjectFilter) -> Option<&TagKey> {
    let [constraint] = filter.tagged_constraints.as_slice() else {
        return None;
    };
    (constraint.relation == TaggedOpbjectRelation::IsTaggedObject).then_some(&constraint.tag)
}

fn chosen_card_tag_from_hand_choice_branch(effects: &[EffectAst]) -> Option<TagKey> {
    fn collect_exact_hand_choices(effects: &[EffectAst], tags: &mut Vec<TagKey>) {
        for pair in effects.windows(2) {
            let [
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action:
                        SubjectVerbActionAst::RevealCardsFromHand {
                            tag: revealed_tag, ..
                        },
                    ..
                }),
                EffectAst::ChooseObjects {
                    filter,
                    count,
                    tag: chosen_tag,
                    ..
                },
            ] = pair
            else {
                continue;
            };
            let chooses_one = count.min == 1
                && count.max == Some(1)
                && !count.dynamic_x
                && !count.up_to_x
                && !count.random;
            if chooses_one
                && tagged_object_reference(filter) == Some(revealed_tag)
                && !tags.contains(chosen_tag)
            {
                tags.push(chosen_tag.clone());
            }
        }

        for effect in effects {
            match effect {
                EffectAst::ChooseObjects {
                    filter, count, tag, ..
                } if filter.zone == Some(Zone::Hand)
                    && count.min == 1
                    && count.max == Some(1)
                    && !count.dynamic_x
                    && !count.up_to_x
                    && !count.random =>
                {
                    if !tags.contains(tag) {
                        tags.push(tag.clone());
                    }
                }
                EffectAst::Coordinated {
                    effects: nested, ..
                }
                | EffectAst::Sequence { effects: nested }
                | EffectAst::SourceSentence {
                    effects: nested, ..
                } => {
                    collect_exact_hand_choices(nested, tags);
                }
                _ => {}
            }
        }
    }

    let mut tags = Vec::new();
    collect_exact_hand_choices(effects, &mut tags);
    let [tag] = tags.as_slice() else {
        return None;
    };
    Some(tag.clone())
}

fn is_dependent_that_player_discard(effect: &EffectAst, chosen_tag: &TagKey) -> bool {
    let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        subject:
            SubjectVerbSubjectAst {
                role: SubjectVerbRoleAst::AffectedPlayer,
                player: PlayerAst::That,
            },
        action:
            SubjectVerbActionAst::Discard {
                count: Value::Fixed(1),
                random: false,
                any_number: false,
                filter: Some(filter),
                tag: None,
            },
    }) = effect
    else {
        return false;
    };
    filter.zone == Some(Zone::Hand) && tagged_object_reference(filter) == Some(chosen_tag)
}

/// Keep a dependent "That player discards that card" inside the `if you do`
/// branch that established both the player and the chosen-card antecedents.
/// Reference resolution runs after sentence grouping, so moving the AST here
/// lets both demonstratives bind within the branch instead of leaking to the
/// surrounding ability sequence.
fn post_rule_hand_reveal_choice_discard_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let [discard] = sentence_effects.as_slice() else {
        return Ok(None);
    };
    let Some(EffectAst::IfResult {
        effects: branch_effects,
        ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };
    let Some(chosen_tag) = chosen_card_tag_from_hand_choice_branch(branch_effects) else {
        return Ok(None);
    };
    if !is_dependent_that_player_discard(discard, &chosen_tag) {
        return Ok(None);
    }

    branch_effects.extend(sentence_effects.drain(..));
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

/// Keep an object-dependent continuation inside the reflexive trigger that
/// establishes its antecedent. A `WhenResult` lowers to a new stack entry, so
/// leaving the continuation as an outer sibling would make it execute before
/// the tagged object exists.
fn post_rule_reflexive_object_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let references_reflexive_object =
        crate::runtime_backend::compile_support::effects_reference_it_tag(sentence_effects)
            || crate::runtime_backend::compile_support::effects_reference_its_controller(
                sentence_effects,
            );
    if sentence_effects.is_empty() || !references_reflexive_object {
        return Ok(None);
    }
    let Some(EffectAst::WhenResult {
        effects: reflexive_effects,
        ..
    }) = state.effects.last_mut()
    else {
        return Ok(None);
    };

    reflexive_effects.append(sentence_effects);
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

fn effect_references_prior_exiled_card(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell {
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if tag.as_str() == crate::tag::PRIOR_EXILED_CARD_TAG
    ) {
        return true;
    }

    let mut found = false;
    for_each_nested_effects(effect, true, |nested| {
        if !found {
            found = nested.iter().any(effect_references_prior_exiled_card);
        }
    });
    found
}

fn bind_cast_tag_to_prior_exiled_card(effect: &mut EffectAst) {
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
        action: SubjectVerbActionAst::CastTagged {
            tag, as_copy: true, ..
        },
        ..
    }) = effect
        && tag.as_str() == IT_TAG
    {
        *tag = TagKey::from(crate::tag::PRIOR_EXILED_CARD_TAG);
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for effect in nested {
            bind_cast_tag_to_prior_exiled_card(effect);
        }
    });
}

/// Bind an authored cross-ability "the exiled card" reference to cards
/// linked to the source permanent.  A same-ability exile is handled by
/// `tag_latest_prior_exile`; when there is no such effect, Magic's linked
/// ability wording refers to the object exiled by another ability of this
/// source (for example, an Imprint ability).
fn bind_prior_exiled_card_to_source_link(effect: &mut EffectAst) {
    let is_prior_exiled_copy = matches!(
        effect,
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::CopySpell {
                target: TargetAst::Tagged(tag, _),
                ..
            },
            ..
        }) if tag.as_str() == crate::tag::PRIOR_EXILED_CARD_TAG
    );
    if is_prior_exiled_copy {
        // Copying a card outside the stack is represented by selecting the
        // linked exiled card and letting the following CastTagged(as_copy)
        // create/cast the copy. This is the same generic program used for the
        // explicit "a card exiled with this artifact" wording.
        *effect = EffectAst::ChooseObjectsAcrossZones {
            filter: ObjectFilter::default().in_zone(Zone::Exile).match_tagged(
                TagKey::from(crate::tag::SOURCE_EXILED_TAG),
                crate::target::TaggedOpbjectRelation::IsTaggedObject,
            ),
            count: crate::ChoiceCount::exactly(1),
            count_value: None,
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            zones: vec![Zone::Exile],
            search_mode: None,
        };
        return;
    }
    for_each_nested_effects_mut(effect, true, |nested| {
        for effect in nested {
            bind_prior_exiled_card_to_source_link(effect);
        }
    });
}

fn tag_latest_prior_exile(effects: &mut [EffectAst]) -> bool {
    let Some(exile_idx) = effects.iter().rposition(|effect| {
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
        tag: TagKey::from(crate::tag::PRIOR_EXILED_CARD_TAG),
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

fn bind_prior_exiled_mana_value(value: &mut Value) {
    match value {
        Value::SurfaceHinted { value, .. } => bind_prior_exiled_mana_value(value),
        Value::ManaValueOf(spec) if matches!(spec.base(), ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG) =>
        {
            **spec = ChooseSpec::Tagged(TagKey::from(crate::tag::PRIOR_EXILED_CARD_TAG));
        }
        _ => {}
    }
}

/// Keep an authored "the exiled card" reference tied to the exact object
/// moved by the latest prior exile, even when the reference occurs inside a
/// delayed trigger whose ordinary `it` antecedent is the triggering object.
fn post_rule_prior_exiled_card_reference(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    if !sentence_effects
        .iter()
        .any(effect_references_prior_exiled_card)
    {
        return Ok(None);
    }
    if !tag_latest_prior_exile(&mut state.effects) {
        for effect in sentence_effects {
            bind_prior_exiled_card_to_source_link(effect);
        }
    }
    Ok(None)
}

fn bind_targeted_leaves_filter(
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

/// A later delayed trigger whose subject is "the targeted ..." watches the
/// exact object selected by the nearest earlier target declaration. Keeping
/// only the noun filter (for example, `creature`) makes every matching object
/// capable of firing the delayed trigger.
fn post_rule_targeted_object_delayed_leave(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
    if !words.starts_with(&["when", "the", "targeted"])
        && !words.starts_with(&["whenever", "the", "targeted"])
    {
        return Ok(None);
    }
    let Some(tag) = state.effects.iter().rev().find_map(|effect| match effect {
        EffectAst::ChooseObjects { tag, .. } => Some(tag.clone()),
        _ => None,
    }) else {
        return Ok(None);
    };
    for effect in sentence_effects {
        if let EffectAst::DelayedTriggerThisTurn { trigger, .. }
        | EffectAst::DelayedTriggerForDuration { trigger, .. } = effect
        {
            bind_targeted_leaves_filter(trigger, &tag);
        }
    }
    Ok(None)
}

fn is_singular_explicit_return_to_battlefield(effect: &EffectAst) -> bool {
    let EffectAst::SubjectVerb(subject_verb) = effect else {
        return false;
    };
    let SubjectVerbActionAst::ReturnToBattlefield { target, .. } = &subject_verb.action else {
        return false;
    };
    if !target_is_explicitly_chosen(target) {
        return false;
    }
    match target {
        TargetAst::WithCount(_, count) | TargetAst::WithCountValue(_, count, _) => {
            count.is_single()
        }
        _ => true,
    }
}

/// A spell can create a one-shot delayed trigger tied to the exact permanent
/// returned by its preceding instruction. Keep the trigger typed instead of
/// allowing the sentence parser to flatten its payload into an immediate
/// follow-up effect.
fn post_rule_returned_permanent_enters(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(comma_idx) = sentence_tokens.iter().position(OwnedLexToken::is_comma) else {
        return Ok(None);
    };
    if crate::runtime_backend::token_word_refs(&sentence_tokens[..comma_idx])
        != ["when", "that", "permanent", "enters"]
        || !state
            .effects
            .last()
            .is_some_and(is_singular_explicit_return_to_battlefield)
        || sentence_effects.is_empty()
    {
        return Ok(None);
    }

    let effects = std::mem::take(sentence_effects);
    sentence_effects.push(EffectAst::DelayedTriggerForDuration {
        trigger: crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface {
            surface: crate::target::SourceReferenceSurface::ThisPermanentType(
                "that permanent".to_string(),
            ),
            subject_number: ironsmith_core::trigger_model::TriggerSubjectNumber::Singular,
            origin_condition: None,
        },
        effects,
        one_shot: true,
        duration: Until::Forever,
        either_of_watched_objects: false,
        while_any_tagged_object_in_zone: None,
    });
    Ok(None)
}

fn post_rule_delayed_trigger_result_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let [EffectAst::IfResult { .. } | EffectAst::WhenResult { .. }] = sentence_effects.as_slice()
    else {
        return Ok(None);
    };
    let Some(
        EffectAst::DelayedTriggerThisTurn { effects, .. }
        | EffectAst::DelayedTriggerForDuration { effects, .. }
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextCleanupStep { effects, .. }
        | EffectAst::DelayedUntilNextUntapStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. }
        | EffectAst::DelayedUntilNextMainPhase { effects, .. }
        | EffectAst::DelayedUntilNextFirstMainPhase { effects, .. }
        | EffectAst::DelayedUntilEndOfCombat { effects },
    ) = state.effects.last_mut()
    else {
        return Ok(None);
    };
    effects.extend(sentence_effects.drain(..));
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

fn effects_are_copy_retarget_followup(effects: &[EffectAst]) -> bool {
    fn contains_retarget(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RetargetStackObject {
                    target: TargetAst::Tagged(tag, _),
                    ..
                },
                ..
            }) if tag.as_str() == crate::cards::builders::COPIED_STACK_OBJECT_TAG
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

fn effects_are_one_copy_retarget_followup(effects: &[EffectAst]) -> bool {
    fn is_one_retarget(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RetargetStackObject {
                    target: TargetAst::Tagged(tag, _),
                    ..
                },
                ..
            }) if tag.as_str() == crate::cards::builders::COPIED_STACK_OBJECT_TAG
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

fn effects_copy_a_stack_object(effects: &[EffectAst]) -> bool {
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

fn trailing_delayed_trigger_effects_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::DelayedTriggerThisTurn { effects, .. } => Some(effects),
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::Coordinated { effects, .. } => effects
            .last_mut()
            .and_then(trailing_delayed_trigger_effects_mut),
        EffectAst::Conditional { if_true, .. } => if_true
            .last_mut()
            .and_then(trailing_delayed_trigger_effects_mut),
        _ => None,
    }
}

fn append_copy_retarget_to_trailing_delayed_trigger(
    previous: &mut EffectAst,
    followups: &mut Vec<EffectAst>,
) -> bool {
    if !effects_are_copy_retarget_followup(followups) {
        return false;
    }
    let Some(delayed_effects) = trailing_delayed_trigger_effects_mut(previous) else {
        return false;
    };
    if !effects_copy_a_stack_object(delayed_effects) {
        return false;
    }
    delayed_effects.append(followups);
    true
}

fn trailing_optional_copy_effects_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    let is_optional_copy = match &*effect {
        EffectAst::May { effects }
        | EffectAst::MayByPlayer {
            player: PlayerAst::You | PlayerAst::Implicit,
            effects,
        } => effects_copy_a_stack_object(effects),
        _ => false,
    };
    if is_optional_copy {
        return match effect {
            EffectAst::May { effects }
            | EffectAst::MayByPlayer {
                player: PlayerAst::You | PlayerAst::Implicit,
                effects,
            } => Some(effects),
            _ => None,
        };
    }
    match effect {
        EffectAst::SourceSentence { effects, .. }
        | EffectAst::Sequence { effects }
        | EffectAst::Coordinated { effects, .. } => effects
            .last_mut()
            .and_then(trailing_optional_copy_effects_mut),
        _ => None,
    }
}

fn append_copy_retarget_to_trailing_optional_copy(
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

/// Sequence dispatch can claim a complete optional retarget sentence before
/// the post-parse follow-up registry runs. Repair that exact adjacency at the
/// public family root as well: a retarget of the copied-stack result belongs
/// inside the immediately preceding delayed trigger that creates that result,
/// never on the outer resolution program where the copy does not exist yet.
pub(super) fn transport_copy_retarget_into_trailing_delayed_trigger(effects: &mut Vec<EffectAst>) {
    let mut index = 1usize;
    while index < effects.len() {
        let mut followups = vec![effects[index].clone()];
        if append_copy_retarget_to_trailing_delayed_trigger(&mut effects[index - 1], &mut followups)
        {
            effects.remove(index);
        } else {
            index += 1;
        }
    }
}

/// A fixed target assignment for "the copy" belongs to the same optional
/// procedure that creates that copy. As an outer sibling it would execute
/// even after the player declined to copy the spell or ability.
pub(super) fn transport_copy_retarget_into_trailing_optional_copy(effects: &mut Vec<EffectAst>) {
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

fn post_rule_delayed_trigger_copy_retarget_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    if !append_copy_retarget_to_trailing_delayed_trigger(previous, sentence_effects) {
        return Ok(None);
    }
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

fn post_rule_optional_copy_retarget_followup(
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

#[cfg(test)]
mod delayed_copy_retarget_followup_tests {
    use super::*;

    fn contains_plural_retarget(effect: &EffectAst) -> bool {
        if matches!(
            effect,
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::RetargetStackObject {
                    copy_reference_plural: true,
                    ..
                },
                ..
            })
        ) {
            return true;
        }
        let mut found = false;
        for_each_nested_effects(effect, true, |nested| {
            found |= nested.iter().any(contains_plural_retarget);
        });
        found
    }

    #[test]
    fn optional_copy_retarget_stays_inside_repeating_delayed_trigger() {
        let tokens = crate::runtime_backend::lex_line(
            "Choose a planeswalker type. Until end of turn, whenever you activate an ability of a planeswalker of that type, copy that ability. You may choose new targets for the copies.",
            0,
        )
        .expect("repeating delayed-copy procedure should lex");
        let parsed = parse_effect_sentences_lexed(&tokens)
            .expect("repeating delayed-copy procedure should parse");
        let [EffectAst::SubjectVerb(_), delayed] = parsed.as_slice() else {
            panic!("retarget must not remain on the outer program: {parsed:#?}");
        };
        let delayed_effects = match delayed {
            EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedTriggerForDuration { effects, .. } => effects,
            _ => panic!("expected a repeating delayed trigger: {delayed:#?}"),
        };
        assert!(effects_copy_a_stack_object(delayed_effects));
        assert!(delayed_effects.iter().any(contains_plural_retarget));
    }

    #[test]
    fn retarget_does_not_attach_to_a_delayed_trigger_that_creates_no_copy() {
        let tokens = crate::runtime_backend::lex_line(
            "Until end of turn, whenever you draw a card, draw a card. You may choose new targets for the copies.",
            0,
        )
        .expect("noncopy delayed-trigger near miss should lex");
        let parsed = parse_effect_sentences_lexed(&tokens)
            .expect("noncopy delayed-trigger near miss should parse");
        assert_eq!(parsed.len(), 2, "{parsed:#?}");
        let delayed_effects = match &parsed[0] {
            EffectAst::DelayedTriggerThisTurn { effects, .. }
            | EffectAst::DelayedTriggerForDuration { effects, .. } => effects,
            effect => panic!("expected delayed near miss: {effect:#?}"),
        };
        assert!(!delayed_effects.iter().any(contains_plural_retarget));
        assert!(contains_plural_retarget(&parsed[1]));
    }

    #[test]
    fn fixed_copy_target_stays_inside_the_optional_copy_branch() {
        let tokens =
            crate::runtime_backend::lex_line("You may copy that spell. The copy targets Ivy.", 0)
                .expect("optional copy procedure should lex");
        let parsed =
            crate::runtime_backend::front_end::shared::util::with_card_source_reference_context(
                "Ivy, Gleeful Spellthief",
                &[crate::CardType::Creature],
                &[crate::Subtype::Faerie, crate::Subtype::Rogue],
                || parse_effect_sentences_lexed(&tokens),
            )
            .expect("optional copy procedure should parse");
        let [optional] = parsed.as_slice() else {
            panic!("the fixed retarget must not remain an outer sibling: {parsed:#?}");
        };
        let optional_effects = match optional {
            EffectAst::May { effects }
            | EffectAst::MayByPlayer {
                player: PlayerAst::You | PlayerAst::Implicit,
                effects,
            } => effects,
            _ => panic!("expected one optional copy owner: {optional:#?}"),
        };
        assert!(effects_copy_a_stack_object(optional_effects));
        assert!(effects_are_one_copy_retarget_followup(
            &optional_effects[optional_effects.len() - 1..]
        ));
    }

    #[test]
    fn unconditional_copy_does_not_acquire_an_optional_owner() {
        let tokens =
            crate::runtime_backend::lex_line("Copy that spell. The copy targets this creature.", 0)
                .expect("unconditional copy procedure should lex");
        let parsed = parse_effect_sentences_lexed(&tokens)
            .expect("unconditional copy procedure should parse");
        assert_eq!(parsed.len(), 2, "{parsed:#?}");
        assert!(!matches!(
            parsed[0],
            EffectAst::May { .. } | EffectAst::MayByPlayer { .. }
        ));
    }
}


#[cfg(test)]
mod retained_land_followup_tests {
    use super::*;

    fn animation() -> EffectAst {
        EffectAst::subject_verb_become_base_pt_creature(
            Value::Fixed(3),
            Value::Fixed(3),
            TargetAst::Source(None),
            vec![crate::types::CardType::Creature],
            Vec::new(),
            Vec::new(),
            None,
            Vec::new(),
            Vec::new(),
            false,
            None,
            Some(ironsmith_core::AnimationPtSurface::LeadingPowerToughness),
            None,
            Until::EndOfTurn,
        )
    }

    #[test]
    fn still_land_followup_reaches_animation_inside_conditional_may() {
        let mut effects = vec![EffectAst::Conditional {
            predicate: PredicateAst::SourceIsTapped,
            if_true: vec![EffectAst::May {
                effects: vec![animation()],
            }],
            if_false: Vec::new(),
        }];

        assert!(mark_last_animation_as_still_a_land(&mut effects));
        let EffectAst::Conditional { if_true, .. } = &effects[0] else {
            panic!("expected conditional wrapper");
        };
        let [EffectAst::May { effects }] = if_true.as_slice() else {
            panic!("expected may wrapper");
        };
        let [
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::BecomeBasePtCreature {
                        preserve_other_types,
                        type_retention_surface,
                        ..
                    },
                ..
            }),
        ] = effects.as_slice()
        else {
            panic!("expected nested animation");
        };

        assert!(*preserve_other_types);
        assert_eq!(
            *type_retention_surface,
            Some(ironsmith_core::TypeRetentionSurface::StillALand)
        );
    }
}

#[cfg(test)]
mod copy_cast_followup_tests {
    use super::*;

    #[test]
    fn delayed_copy_of_prior_exiled_card_keeps_cast_inside_trigger() {
        let lexed = crate::runtime_backend::lex_line(
            "Exile target instant or sorcery card from your graveyard. Creatures you control get +X/+0 until end of turn, where X is that card's mana value. Whenever a creature you control deals combat damage to a player this turn, copy the exiled card. You may cast the copy without paying its mana cost.",
            0,
        )
        .expect("Surge to Victory text should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("Surge to Victory text should parse");

        assert_eq!(
            parsed.len(),
            3,
            "cast follow-up escaped delayed trigger: {parsed:#?}"
        );
        let EffectAst::DelayedTriggerThisTurn { effects, .. } = &parsed[2] else {
            panic!("expected delayed combat-damage trigger: {parsed:#?}");
        };
        assert!(
            format!("{effects:#?}").contains("CastTagged"),
            "copy cast should remain inside delayed trigger: {parsed:#?}"
        );

        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Surge Shape")
            .card_types(vec![crate::CardType::Sorcery])
            .parse_text(
                "Exile target instant or sorcery card from your graveyard. Creatures you control get +X/+0 until end of turn, where X is that card's mana value. Whenever a creature you control deals combat damage to a player this turn, copy the exiled card. You may cast the copy without paying its mana cost.",
            )
            .expect("Surge to Victory shape should compile");
        let debug = format!("{definition:#?}");
        let cast = debug
            .split_once("CastTaggedEffect")
            .map(|(_, tail)| &tail[..tail.len().min(500)])
            .expect("delayed trigger should contain a tagged cast");
        assert!(cast.contains(crate::tag::PRIOR_EXILED_CARD_TAG), "{debug}");
        assert!(!cast.contains("triggering"), "{debug}");
        let mana_value = debug
            .split_once("ManaValueOf")
            .map(|(_, tail)| &tail[..tail.len().min(500)])
            .expect("pump should contain a mana-value reference");
        assert!(
            mana_value.contains(crate::tag::PRIOR_EXILED_CARD_TAG),
            "pump should use the exiled card's mana value: {debug}"
        );
    }

    #[test]
    fn immediate_exiled_card_cast_keeps_its_may_scope() {
        let lexed = crate::runtime_backend::lex_line("You may cast the exiled card.", 0)
            .expect("optional tagged cast should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("optional tagged cast should parse");

        assert!(
            matches!(
                parsed.as_slice(),
                [EffectAst::May { effects }]
                    if matches!(
                        effects.as_slice(),
                        [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                            action: SubjectVerbActionAst::CastTagged { .. },
                            ..
                        })]
                    )
            ),
            "expected immediate cast inside a may scope, got {parsed:#?}"
        );
    }

    #[test]
    fn cross_ability_exiled_card_copy_uses_source_link() {
        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Imprint Copy")
            .card_types(vec![crate::CardType::Artifact])
            .subtypes(vec![crate::Subtype::Equipment])
            .parse_text(
                "Imprint — When this Equipment enters, you may exile an instant card from your hand.\n\
                 Whenever equipped creature deals combat damage to a player, you may copy the exiled card. If you do, you may cast the copy without paying its mana cost.\n\
                 Equip {4}",
            )
            .expect("a linked Imprint copy ability should compile");
        let debug = format!("{definition:#?}");

        assert!(debug.contains("ImprintFromHandEffect"), "{debug}");
        assert!(debug.contains(crate::tag::SOURCE_EXILED_TAG), "{debug}");
        assert!(
            !debug.contains("CopySpellEffect"),
            "an exiled card is not a stack spell and must be selected before the copy-cast: {debug}"
        );
        assert!(debug.contains("CastTaggedEffect"), "{debug}");
        let cast_debug = debug
            .split_once("CastTaggedEffect")
            .map(|(_, tail)| tail)
            .expect("combat-damage trigger should contain a tagged cast");
        assert!(cast_debug.contains(IT_TAG), "{debug}");
        assert!(cast_debug.contains("as_copy: true"), "{debug}");
        assert!(
            cast_debug.contains("without_paying_mana_cost: true"),
            "{debug}"
        );
    }

    #[test]
    fn copy_card_then_may_cast_copy_uses_prior_moved_tag_without_copying_source() {
        let definition = crate::CardDefinitionBuilder::new(crate::CardId::new(), "Copy Variant")
            .card_types(vec![crate::CardType::Sorcery])
            .parse_text(
                "Exile target instant or sorcery card from your graveyard. Copy that card. You may cast the copy without paying its mana cost.",
            )
            .expect("copy-and-cast sequence should compile");
        let debug = format!("{definition:#?}");

        assert!(!debug.contains("CopySpellEffect"), "{debug}");
        let cast_debug = debug
            .split_once("CastTaggedEffect")
            .map(|(_, tail)| tail)
            .expect("sequence should lower to a tagged cast");
        assert!(cast_debug.contains("__sentence_helper_exiled"), "{debug}");
        assert!(cast_debug.contains("as_copy: true"), "{debug}");
        assert!(
            cast_debug.contains("without_paying_mana_cost: true"),
            "{debug}"
        );
    }
}

#[cfg(test)]
mod revealed_hand_actor_tests {
    use super::*;

    #[test]
    fn dependent_exile_keeps_the_revealing_player_as_actor() {
        let lexed = crate::runtime_backend::lex_line(
            "Target opponent reveals X cards from their hand, where X is the number of Goblins you control. You choose one of those cards. That player exiles it.",
            0,
        )
        .expect("dependent hand reveal should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("dependent hand reveal should parse");
        let debug = format!("{parsed:#?}");
        assert!(debug.contains("player: That"), "{debug}");
    }
}

#[cfg(test)]
mod declined_move_followup_tests {
    use super::*;

    #[test]
    fn source_exiled_move_and_decline_fallback_stay_one_conditional() {
        let lexed = crate::runtime_backend::lex_line(
            "You may put the exiled card onto the battlefield if it's a creature card. If you don't put it onto the battlefield, put it into its owner's hand.",
            0,
        )
        .expect("source-exiled move should lex");
        let parsed = parse_effect_sentences_lexed(&lexed)
            .expect("source-exiled move and decline fallback should parse");

        assert_eq!(parsed.len(), 1, "{parsed:#?}");
    }
}

#[cfg(test)]
mod damage_self_replacement_followup_tests {
    use super::*;

    #[test]
    fn it_deals_to_that_creature_ignores_prior_cost_object_provenance() {
        let lexed = crate::runtime_backend::lex_line(
            "This deals 2 damage to target creature. It deals 4 damage to that creature instead if this spell's additional cost was paid.",
            0,
        )
        .expect("damage self-replacement should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("damage self-replacement should parse");
        let [
            EffectAst::SelfReplacement {
                if_true, if_false, ..
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one typed self-replacement: {parsed:#?}");
        };
        assert!(
            matches!(
                if_true.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::DealDamageEqualToPower {
                        source: TargetAst::Source(_),
                        target: TargetAst::Object(_, Some(_), _),
                        ..
                    },
                    ..
                })]
            ),
            "the replacement should directly reuse the default spell source and target: {if_true:#?}"
        );
        assert_eq!(if_false.len(), 1, "default damage should remain intact");
        assert!(
            !format!("{parsed:#?}").contains("TrailingIf"),
            "the authored trailing-if surface must be consumed by the typed self-replacement: {parsed:#?}"
        );

        let lowered =
            crate::runtime_backend::compile_support::compile_statement_effects_with_imports(
                &parsed,
                &crate::runtime_backend::reference_model::ReferenceImports::with_last_object_tag(
                    "counters_0",
                ),
            )
            .expect("damage self-replacement should lower");
        let debug = format!("{lowered:#?}");
        assert!(debug.contains("ExecuteWithSourceEffect"), "{debug}");
        assert!(debug.contains("source: Source"), "{debug}");
        assert!(!debug.contains("ForEachObject"), "{debug}");
        assert!(!debug.contains("counters_0"), "{debug}");
    }

    #[test]
    fn omitted_damage_target_reuses_the_default_target() {
        let lexed = crate::runtime_backend::lex_line(
            "This deals 3 damage to target creature. It deals 5 damage instead if you control an artifact.",
            0,
        )
        .expect("damage self-replacement should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("damage self-replacement should parse");
        let [EffectAst::SelfReplacement { if_true, .. }] = parsed.as_slice() else {
            panic!("expected one typed self-replacement: {parsed:#?}");
        };
        assert!(
            matches!(
                if_true.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::DealDamageEqualToPower {
                        target: TargetAst::Object(_, Some(_), _),
                        ..
                    },
                    ..
                })]
            ),
            "the omitted replacement target should reuse the default creature target: {if_true:#?}"
        );
    }
}

#[cfg(test)]
mod counter_self_replacement_followup_tests {
    use super::*;

    #[test]
    fn double_counters_on_that_creature_reuses_the_default_target() {
        let lexed = crate::runtime_backend::lex_line(
            "Put a +1/+1 counter on target creature you control. If this is the second time this ability has resolved this turn, double the number of +1/+1 counters on that creature instead.",
            0,
        )
        .expect("counter self-replacement should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("counter self-replacement should parse");
        let [
            EffectAst::SelfReplacement {
                if_true, if_false, ..
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one typed self-replacement: {parsed:#?}");
        };

        assert!(
            matches!(
                if_true.as_slice(),
                [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::DoubleCountersOnTarget { .. },
                    ..
                })]
            ),
            "the replacement branch should keep the typed double-counter action: {if_true:#?}"
        );
        let default_target = primary_target_from_effect(&if_false[0])
            .expect("the default counter effect should have a target");
        let replacement_target = primary_target_from_effect(&if_true[0])
            .expect("the replacement counter effect should reuse that target");
        assert_eq!(replacement_target, default_target);
        assert!(target_is_explicitly_chosen(&replacement_target));

        let lowered =
            crate::runtime_backend::compile_support::compile_statement_effects_with_imports(
                &parsed,
                &crate::runtime_backend::reference_model::ReferenceImports::default(),
            )
            .expect("counter self-replacement should lower");
        let [segment] = lowered.effects.segments.as_slice() else {
            panic!(
                "expected one self-replacement segment: {:#?}",
                lowered.effects
            );
        };
        let [target_declaration, put_counters] = segment.default_effects.as_slice() else {
            panic!("expected a target prelude and default counter action: {segment:#?}");
        };
        let target_declaration = target_declaration
            .downcast_ref::<crate::effects::TaggedEffect>()
            .expect("the shared target declaration should carry an alias tag");
        let target_tag = target_declaration.tag.clone();
        assert!(
            target_declaration
                .effect
                .downcast_ref::<crate::effects::TargetOnlyEffect>()
                .is_some(),
            "the unconditional prelude should select the one authored target: {target_declaration:#?}"
        );
        let put_counters = put_counters
            .downcast_ref::<crate::effects::TaggedEffect>()
            .and_then(|tagged| {
                tagged
                    .effect
                    .downcast_ref::<crate::effects::PutCountersEffect>()
            })
            .expect("the default branch should put the counter");
        assert!(
            matches!(&put_counters.target, ChooseSpec::Tagged(tag) if tag == &target_tag),
            "the default counter action must consume the shared target alias: {put_counters:#?}"
        );
        let [replacement] = segment.self_replacements[0].replacement_effects.as_slice() else {
            panic!("expected one replacement counter action: {segment:#?}");
        };
        let replacement = replacement
            .downcast_ref::<crate::effects::DoubleCountersEffect>()
            .expect("the replacement branch should double counters");
        assert!(
            matches!(&replacement.target, ChooseSpec::Tagged(tag) if tag == &target_tag),
            "the replacement must reuse the one authored target alias: {replacement:#?}"
        );
        assert_eq!(
            lowered.choices.len(),
            1,
            "the anaphoric replacement must not announce a second target"
        );
    }
}

#[cfg(test)]
mod prior_token_copy_self_replacement_tests {
    use super::*;

    fn copy_count(effects: &[EffectAst]) -> Option<Value> {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::CreateTokenCopy { count, .. }
                    | SubjectVerbActionAst::CreateTokenCopyFromSource { count, .. },
                ..
            }) = effect
            {
                return Some(count.clone());
            }
            let mut nested_count = None;
            for_each_nested_effects(effect, true, |nested| {
                if nested_count.is_none() {
                    nested_count = copy_count(nested);
                }
            });
            if nested_count.is_some() {
                return nested_count;
            }
        }
        None
    }

    fn copy_source(effects: &[EffectAst]) -> Option<TargetAst> {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::CreateTokenCopyFromSource { source, .. },
                ..
            }) = effect
            {
                return Some(source.clone());
            }
            let mut nested_source = None;
            for_each_nested_effects(effect, true, |nested| {
                if nested_source.is_none() {
                    nested_source = copy_source(nested);
                }
            });
            if nested_source.is_some() {
                return nested_source;
            }
        }
        None
    }

    #[test]
    fn conditional_of_those_tokens_replaces_copy_token_count() {
        let lexed = crate::runtime_backend::lex_line(
            "Create a tapped and attacking token that's a copy of another target attacking creature. If that creature is a Kraken, Leviathan, Octopus, or Serpent, create two of those tokens instead.",
            0,
        )
        .expect("copy-token replacement should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("copy-token replacement should parse");
        let [
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                attach_to_previous_ability: false,
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one typed copy-token self-replacement: {parsed:#?}");
        };

        assert_eq!(
            copy_count(if_false),
            Some(Value::Fixed(1)),
            "default copy branch: {if_false:#?}"
        );
        assert_eq!(
            copy_count(if_true),
            Some(Value::Fixed(2)),
            "replacement copy branch: {if_true:#?}"
        );
        let PredicateAst::TargetMatches(filter) = predicate else {
            panic!(
                "the demonstrative subtype condition must test the copy source target: {predicate:#?}"
            );
        };
        for subtype in ["Kraken", "Leviathan", "Octopus", "Serpent"] {
            assert!(format!("{filter:#?}").contains(subtype), "{filter:#?}");
        }
        let default_source = copy_source(if_false).expect("default copy source target");
        let replacement_source = copy_source(if_true).expect("replacement copy source target");
        assert_eq!(replacement_source, default_source);
        assert!(target_is_explicitly_chosen(&default_source));
    }

    #[test]
    fn triggered_copy_replacement_lowers_subtype_check_to_declared_target() {
        let definition = crate::CardDefinitionBuilder::new(
            crate::CardId::new(),
            "Triggered Copy Replacement",
        )
        .card_types(vec![crate::CardType::Creature])
        .parse_text(
            "Whenever this creature attacks, create a tapped and attacking token that's a copy of another target attacking creature. If that creature is a Kraken, Leviathan, Octopus, or Serpent, create two of those tokens instead.",
        )
        .expect("triggered copy replacement should lower");
        let debug = format!("{:#?}", definition.abilities);
        assert!(debug.contains("condition: TargetMatches"), "{debug}");
        assert!(!debug.contains("condition: TaggedObjectMatches"), "{debug}");
    }
}

#[cfg(test)]
mod targeted_delayed_leave_followup_tests {
    use super::*;

    fn leaves_filter(trigger: &crate::cards::builders::TriggerSpec) -> Option<&ObjectFilter> {
        match trigger {
            crate::cards::builders::TriggerSpec::WithIntro { trigger, .. } => {
                leaves_filter(trigger)
            }
            crate::cards::builders::TriggerSpec::LeavesBattlefield(filter) => Some(filter),
            _ => None,
        }
    }

    #[test]
    fn targeted_creature_leave_watcher_reuses_delayed_target_choice() {
        let lexed = crate::runtime_backend::lex_line(
            "Whenever target creature deals combat damage to a non-Wall creature this turn, destroy that non-Wall creature. When the targeted creature leaves the battlefield this turn, sacrifice this artifact.",
            0,
        )
        .expect("linked delayed triggers should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("linked delayed triggers should parse");
        let chosen_tag = parsed
            .iter()
            .find_map(|effect| match effect {
                EffectAst::ChooseObjects { tag, .. } => Some(tag),
                _ => None,
            })
            .expect("the target creature should be selected at resolution");
        let leave_filter = parsed
            .iter()
            .find_map(|effect| match effect {
                EffectAst::DelayedTriggerThisTurn { trigger, .. } => leaves_filter(trigger),
                _ => None,
            })
            .expect("the later sentence should register a leave watcher");

        assert!(
            leave_filter.tagged_constraints.iter().any(|constraint| {
                constraint.tag == *chosen_tag
                    && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
            }),
            "the leave watcher must be restricted to the chosen target: {parsed:#?}"
        );
    }
}

#[cfg(test)]
mod returned_permanent_enters_followup_tests {
    use super::*;

    #[test]
    fn singular_return_result_gets_a_one_shot_enter_watcher() {
        let lexed = crate::runtime_backend::lex_line(
            "Return target permanent card from an opponent's graveyard to the battlefield. When that permanent enters, return up to one target permanent card from your graveyard to the battlefield.",
            0,
        )
        .expect("linked return should lex");
        let parsed =
            parse_effect_sentences_lexed(&lexed).expect("linked return should parse structurally");

        let [
            EffectAst::SubjectVerb(first),
            EffectAst::DelayedTriggerForDuration {
                trigger:
                    crate::cards::builders::TriggerSpec::ThisEntersBattlefieldWithSurface {
                        surface: crate::target::SourceReferenceSurface::ThisPermanentType(surface),
                        ..
                    },
                effects,
                one_shot: true,
                duration: Until::Forever,
                ..
            },
        ] = parsed.as_slice()
        else {
            panic!("expected return followed by linked enter watcher: {parsed:#?}");
        };
        assert!(matches!(
            first.action,
            SubjectVerbActionAst::ReturnToBattlefield { .. }
        ));
        assert_eq!(surface, "that permanent");
        assert!(matches!(
            effects.as_slice(),
            [EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action: SubjectVerbActionAst::ReturnToBattlefield { .. },
                ..
            })]
        ));
    }

    #[test]
    fn anaphor_and_singular_return_guards_reject_near_misses() {
        for text in [
            "Return target permanent card from an opponent's graveyard to the battlefield. When that card enters, return target permanent card from your graveyard to the battlefield.",
            "Return up to two target permanent cards from an opponent's graveyard to the battlefield. When that permanent enters, return target permanent card from your graveyard to the battlefield.",
        ] {
            let lexed = crate::runtime_backend::lex_line(text, 0).expect("near miss should lex");
            let parsed =
                parse_effect_sentences_lexed(&lexed).expect("near miss should still parse");
            assert!(
                !parsed
                    .iter()
                    .any(|effect| matches!(effect, EffectAst::DelayedTriggerForDuration { .. })),
                "near miss must not acquire linked delayed semantics: {parsed:#?}"
            );
        }
    }
}

#[cfg(test)]
mod conditional_target_self_replacement_followup_tests {
    use super::*;
    use crate::cards::builders::TurnHistoryPredicateAst;

    fn assert_it_characteristic_threshold(predicate: &PredicateAst, toughness: bool) {
        match predicate {
            PredicateAst::ValueComparison {
                left,
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                ..
            } => {
                let spec = match left {
                    Value::ToughnessOf(spec) if toughness => spec,
                    Value::ManaValueOf(spec) if !toughness => spec,
                    _ => panic!("expected the authored target characteristic: {predicate:#?}"),
                };
                assert!(
                    matches!(
                        spec.base(),
                        ChooseSpec::Tagged(tag) if tag.as_str() == IT_TAG
                    ),
                    "the threshold must remain linked to the targeted object: {predicate:#?}"
                );
            }
            PredicateAst::ItMatches(filter)
                if matches!(
                    (&filter.toughness, &filter.mana_value, toughness),
                    (Some(crate::filter::Comparison::LessThanOrEqual(_)), _, true)
                        | (
                            _,
                            Some(crate::filter::Comparison::LessThanOrEqual(_)),
                            false
                        )
                ) => {}
            _ => panic!("expected a typed at-most threshold: {predicate:#?}"),
        }
    }

    fn trailing_threshold(effects: &[EffectAst], toughness: bool) {
        let [EffectAst::TrailingIf { predicate, effects }] = effects else {
            panic!("expected one trailing target threshold: {effects:#?}");
        };
        assert_it_characteristic_threshold(predicate, toughness);
        assert_eq!(effects.len(), 1, "threshold branch must retain one action");
    }

    #[test]
    fn madness_replacement_keeps_both_target_toughness_thresholds() {
        let lexed = crate::runtime_backend::lex_line(
            "Gain control of target creature if its toughness is 2 or less. If this spell's madness cost was paid, instead gain control of that creature if its toughness is X or less.",
            0,
        )
        .expect("conditional target replacement should lex");
        let parsed = parse_effect_sentences_lexed(&lexed)
            .expect("conditional target replacement should parse");

        let [
            EffectAst::SelfReplacement {
                predicate: PredicateAst::ThisSpellPaidLabel(label),
                if_true,
                if_false,
                attach_to_previous_ability: false,
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one paid-cost self-replacement: {parsed:#?}");
        };
        assert!(label.display_label().eq_ignore_ascii_case("Madness"));
        trailing_threshold(if_false, true);
        trailing_threshold(if_true, true);
    }

    #[test]
    fn kicked_replacement_keeps_both_target_mana_value_thresholds() {
        let lexed = crate::runtime_backend::lex_line(
            "Destroy target artifact if its mana value is 2 or less. If this spell was kicked, destroy that artifact if its mana value is 5 or less instead.",
            0,
        )
        .expect("conditional target replacement should lex");
        let parsed = parse_effect_sentences_lexed(&lexed)
            .expect("conditional target replacement should parse");

        let [
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                attach_to_previous_ability: false,
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one kicked self-replacement: {parsed:#?}");
        };
        assert!(
            matches!(
                predicate,
                PredicateAst::ThisSpellWasKicked
                    | PredicateAst::TurnHistory(TurnHistoryPredicateAst::SourceWasKicked { .. })
            ),
            "expected a kicked-source predicate: {predicate:#?}"
        );
        trailing_threshold(if_false, false);
        trailing_threshold(if_true, false);
    }

    #[test]
    fn kicked_target_replacement_carries_the_common_exile_life_suffix_into_both_arms() {
        let lexed = crate::runtime_backend::lex_line(
            "Choose target creature with mana value 3 or less. If this spell was kicked, instead choose target creature. Exile the chosen creature, then its controller gains life equal to its mana value.",
            0,
        )
        .expect("kicked target replacement should lex");
        let parsed = parse_effect_sentences_lexed(&lexed)
            .expect("kicked target replacement and common suffix should parse");

        let [
            EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
                attach_to_previous_ability: false,
            },
        ] = parsed.as_slice()
        else {
            panic!("expected one executable self-replacement: {parsed:#?}");
        };
        assert!(matches!(
            predicate,
            PredicateAst::ThisSpellWasKicked
                | PredicateAst::TurnHistory(TurnHistoryPredicateAst::SourceWasKicked { .. })
        ));
        for branch in [if_false, if_true] {
            let debug = format!("{branch:#?}");
            assert!(debug.contains("Exile"), "missing common exile: {debug}");
            assert!(
                debug.contains("GainLife"),
                "missing common life gain: {debug}"
            );
            assert!(
                debug.contains("ManaValueOf"),
                "life amount lost its chosen-object basis: {debug}"
            );
        }
    }

    #[test]
    fn a_non_instead_kicked_choice_is_not_rewritten_as_a_self_replacement() {
        let lexed = crate::runtime_backend::lex_line(
            "Choose target creature with mana value 3 or less. If this spell was kicked, choose target creature.",
            0,
        )
        .expect("conditional choice near miss should lex");
        let parsed = parse_effect_sentences_lexed(&lexed)
            .expect("conditional choice near miss should remain parseable");
        assert!(
            !parsed
                .iter()
                .any(|effect| matches!(effect, EffectAst::SelfReplacement { .. })),
            "only an authored instead clause can replace the default choice: {parsed:#?}"
        );
    }
}

#[cfg(test)]
mod targeted_search_self_replacement_followup_tests {
    use super::*;

    fn search_subjects(
        effects: &[EffectAst],
        found: &mut Vec<(PlayerAst, PlayerAst, Option<PlayerFilter>)>,
    ) {
        for effect in effects {
            if let EffectAst::SubjectVerb(SubjectVerbEffectAst {
                action:
                    SubjectVerbActionAst::SearchLibrary {
                        filter,
                        chooser,
                        player,
                        ..
                    },
                ..
            }) = effect
            {
                found.push((*chooser, *player, filter.owner.clone()));
            }
            for_each_nested_effects(effect, true, |nested| search_subjects(nested, found));
        }
    }

    #[test]
    fn targeted_search_instead_branch_carries_owner_without_changing_implicit_chooser() {
        let lexed = crate::runtime_backend::lex_line(
            "Search target player's library for up to three cards, exile them, then that player shuffles. If this spell was kicked, instead search that player's library for up to fifteen cards, exile them, then that player shuffles.",
            0,
        )
        .expect("targeted search self-replacement should lex");
        let parsed = parse_effect_sentences_lexed(&lexed)
            .expect("targeted search self-replacement should parse");
        let (if_true, if_false) = parsed
            .iter()
            .find_map(|effect| match effect {
                EffectAst::SelfReplacement {
                    if_true, if_false, ..
                } => Some((if_true, if_false)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("expected a targeted-search self-replacement: {parsed:#?}"));

        let mut subjects = Vec::new();
        search_subjects(if_false, &mut subjects);
        search_subjects(if_true, &mut subjects);
        assert_eq!(
            subjects,
            vec![
                (
                    PlayerAst::Implicit,
                    PlayerAst::That,
                    Some(PlayerFilter::target_player()),
                ),
                (
                    PlayerAst::Implicit,
                    PlayerAst::That,
                    Some(PlayerFilter::target_player()),
                ),
            ],
            "both branches carry the target-qualified library while preserving a demonstrative action surface"
        );

        let lowered =
            crate::runtime_backend::compile_support::compile_statement_effects_with_imports(
                &parsed,
                &crate::runtime_backend::reference_model::ReferenceImports::default(),
            )
            .expect("targeted search self-replacement should lower");
        let debug = format!("{lowered:#?}");
        assert!(!debug.contains("IteratedPlayer"), "{debug}");
        assert_eq!(debug.matches("chooser: You").count(), 2, "{debug}");
    }
}
