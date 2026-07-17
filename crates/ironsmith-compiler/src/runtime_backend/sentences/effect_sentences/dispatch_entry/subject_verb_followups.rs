use super::*;
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

type PreParseFollowupRuleFn = for<'a> fn(
    &mut SentenceDispatchState<'a>,
    &[SentenceInput],
    usize,
    &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError>;

type PostParseFollowupRuleFn = for<'a> fn(
    &mut SentenceDispatchState<'a>,
    &[SentenceInput],
    usize,
    &[OwnedLexToken],
    &mut Vec<EffectAst>,
)
    -> Result<Option<PostParseFollowupResult>, CardTextError>;

struct SubjectVerbFollowupRuleDef {
    id: &'static str,
    priority: u16,
    heads: &'static [&'static str],
    run: PreParseFollowupRuleFn,
}

struct SubjectVerbPostParseRuleDef {
    id: &'static str,
    priority: u16,
    heads: &'static [&'static str],
    run: PostParseFollowupRuleFn,
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
    match action.clone() {
        SubjectVerbActionAst::GrantAbilitiesToTarget {
            abilities,
            duration,
            condition,
            ..
        } => Ok(Some(if let Some(condition) = condition {
            EffectAst::subject_verb_grant_abilities_all_with_condition(
                filter, abilities, duration, condition,
            )
        } else {
            EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration)
        })),
        SubjectVerbActionAst::GrantAbilitiesAll {
            abilities,
            duration,
            condition,
            ..
        } => Ok(Some(if let Some(condition) = condition {
            EffectAst::subject_verb_grant_abilities_all_with_condition(
                filter, abilities, duration, condition,
            )
        } else {
            EffectAst::subject_verb_grant_abilities_all(filter, abilities, duration)
        })),
        _ => Ok(None),
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

fn rule_matches_sentence_head(heads: &[&str], tokens: &[OwnedLexToken]) -> bool {
    if heads.is_empty() {
        return true;
    }
    LexedClause::new(tokens)
        .first_word()
        .is_some_and(|head| heads.iter().any(|candidate| head == *candidate))
}

fn pre_followup_subject_verb_route(id: &str) -> &'static str {
    match id {
        "library-shuffle" => {
            "subject-verb verb=Shuffle subject=implicit recognizer=library-followup"
        }
        "still-lands" => {
            "subject-verb verb=Remain subject=implicit recognizer=land-animation-followup"
        }
        "cant-be-regenerated" => {
            "subject-verb verb=Cant subject=implicit recognizer=regeneration-followup"
        }
        "damage-cant-be-prevented" => {
            "subject-verb verb=Deal subject=previous recognizer=unpreventable-damage-followup"
        }
        "copy-and-cast" => "subject-verb verb=Copy subject=implicit recognizer=copy-cast-followup",
        "token-followups" => "subject-verb verb=Create subject=implicit recognizer=token-followup",
        "exile-this-way" => "subject-verb verb=Exile subject=implicit recognizer=this-way-followup",
        "tap-damage-this-way" => {
            "subject-verb verb=Tap subject=implicit recognizer=damage-this-way-followup"
        }
        "destroy-those-creatures" => {
            "subject-verb verb=Destroy subject=implicit recognizer=referential-followup"
        }
        "otherwise" => "subject-verb verb=Do subject=implicit recognizer=otherwise-followup",
        _ => "subject-verb verb=Do subject=implicit recognizer=pre-parse-followup",
    }
}

pub(super) fn run_pre_parse_followup_registry(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let mut matching_rules = PRE_PARSE_SUBJECT_VERB_FOLLOWUP_RULES
        .iter()
        .filter(|rule| rule_matches_sentence_head(rule.heads, sentence_tokens))
        .collect::<Vec<_>>();
    matching_rules.sort_by_key(|rule| rule.priority);

    for rule in matching_rules {
        if let Some(mut result) = (rule.run)(state, sentences, sentence_idx, sentence_tokens)? {
            parser_trace(
                format!(
                    "parse_effect_sentences:subject-verb-followup-pre:{}",
                    rule.id
                )
                .as_str(),
                sentence_tokens,
            );
            if let PreParseFollowupResult::Handled { route, .. } = &mut result
                && route.is_none()
            {
                *route = Some(pre_followup_subject_verb_route(rule.id));
            }
            return Ok(Some(result));
        }
    }
    Ok(None)
}

pub(super) fn run_post_parse_followup_registry(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    let mut matching_rules = POST_PARSE_SUBJECT_VERB_FOLLOWUP_RULES
        .iter()
        .filter(|rule| rule_matches_sentence_head(rule.heads, sentence_tokens))
        .collect::<Vec<_>>();
    matching_rules.sort_by_key(|rule| rule.priority);

    for rule in matching_rules {
        if let Some(result) = (rule.run)(
            state,
            sentences,
            sentence_idx,
            sentence_tokens,
            sentence_effects,
        )? {
            parser_trace(
                format!(
                    "parse_effect_sentences:subject-verb-followup-post:{}",
                    rule.id
                )
                .as_str(),
                sentence_tokens,
            );
            return Ok(Some(result));
        }
    }
    Ok(None)
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
        state.effects.push(build_may_cast_tagged_effect(&spec));
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
    let reminder_facts = followup_shapes::token_reminder_followup_facts(reminder_tokens);
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
    if is_generic_token_reminder_sentence(reminder_tokens)
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
    if is_generic_token_reminder_sentence(reminder_tokens) {
        if !reminder_facts.delayed_pronoun_lifecycle && !reminder_facts.pronoun_trigger_prefix {
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                LexedClause::new(sentence_tokens).text()
            )));
        }
    }
    if let Some(effects) = parse_choose_target_prelude_sentence(sentence_tokens)? {
        state.effects.extend(effects);
        *state.carried_context = None;
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    if let Some(abilities) = parse_token_granted_ability_followup_sentence_lexed(reminder_tokens)? {
        if try_apply_token_granted_ability_followup(state.effects, &abilities)? {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: Some(
                    "subject-verb verb=Grant subject=implicit recognizer=created-token-ability-followup",
                ),
            }));
        }
    }
    if let Some(followup) = parse_token_copy_followup_sentence(sentence_tokens) {
        if try_apply_token_copy_followup(state.effects, followup)? {
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: Some(
                    "subject-verb verb=Exile subject=implicit recognizer=token-copy-delayed-followup",
                ),
            }));
        }
        let mut plan = SentenceParsePlan::new(sentence_tokens.to_vec());
        plan.direct_effects = Some(apply_unapplied_token_copy_followup(
            sentences[sentence_idx].lowered(),
            sentence_tokens,
            followup,
            state.effects.is_empty(),
        )?);
        return Ok(Some(PreParseFollowupResult::Plan(plan)));
    }
    Ok(None)
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
    let SubjectVerbActionAst::CreateTokenWithMods { count, .. } = &mut subject_verb.action else {
        return None;
    };
    *count = Value::Fixed(shape.count as i32);

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
    plan.wrap_if_result = Some(IfResultPredicate::DidNot);
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
                matches!(
                    effect,
                    EffectAst::SubjectVerb(SubjectVerbEffectAst {
                        action: SubjectVerbActionAst::Clash { .. },
                        ..
                    })
                )
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
        *sentence_effects = vec![EffectAst::DelayedUntilEndStepOfExtraTurn {
            player: extra_turn_player,
            effects: sentence_effects.clone(),
        }];
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
        | EffectAst::SourceSentence { effects }
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
        let Some(previous) = state.effects.pop() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous effect for 'instead' conditional rewrite".to_string(),
            ));
        };
        let previous_target = primary_target_from_effect(&previous);
        let previous_damage_target = primary_damage_target_from_effect(&previous);
        let previous_damage_source = primary_damage_source_from_effect(&previous);
        if let Some((predicate, mut if_true, mut if_false)) = sentence_effects
            .pop()
            .and_then(take_self_replacement_condition)
        {
            if has_trailing_unpreventable_damage_rider(sentence_tokens)
                && !mark_last_deal_damage_unpreventable(&mut if_true)
            {
                return Err(CardTextError::ParseError(format!(
                    "unpreventable-damage replacement rider has no damage effect (clause: '{}')",
                    LexedClause::new(sentence_tokens).text(),
                )));
            }
            let (default_effects, carried_player) =
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
            player: library_owner,
            ..
        } = action
            && *library_owner == PlayerAst::That
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
                | EffectAst::SourceSentence { effects: nested } => {
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
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. }
        | EffectAst::DelayedUntilNextMainPhase { effects, .. },
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
        match effect {
            EffectAst::SubjectVerb(subject_verb) => matches!(
                subject_verb.action,
                SubjectVerbActionAst::RetargetStackObject { .. }
            ),
            EffectAst::May { effects } | EffectAst::MayByPlayer { effects, .. } => {
                effects.iter().any(contains_retarget)
            }
            _ => false,
        }
    }

    effects.iter().any(contains_retarget)
}

fn trailing_delayed_trigger_effects_mut(effect: &mut EffectAst) -> Option<&mut Vec<EffectAst>> {
    match effect {
        EffectAst::DelayedTriggerThisTurn { effects, .. } => Some(effects),
        EffectAst::Conditional { if_true, .. } => if_true.iter_mut().rev().find_map(|effect| {
            if let EffectAst::DelayedTriggerThisTurn { effects, .. } = effect {
                Some(effects)
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn post_rule_delayed_trigger_copy_retarget_followup(
    state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    _sentence_tokens: &[OwnedLexToken],
    sentence_effects: &mut Vec<EffectAst>,
) -> Result<Option<PostParseFollowupResult>, CardTextError> {
    if !effects_are_copy_retarget_followup(sentence_effects) {
        return Ok(None);
    }
    let Some(previous) = state.effects.last_mut() else {
        return Ok(None);
    };
    let Some(delayed_effects) = trailing_delayed_trigger_effects_mut(previous) else {
        return Ok(None);
    };

    delayed_effects.extend(sentence_effects.drain(..));
    Ok(Some(PostParseFollowupResult::Handled {
        consumed_sentences: 1,
    }))
}

const PRE_PARSE_SUBJECT_VERB_FOLLOWUP_RULES: &[SubjectVerbFollowupRuleDef] = &[
    SubjectVerbFollowupRuleDef {
        id: "library-shuffle",
        priority: 10,
        heads: &["if", "then", "that"],
        run: pre_rule_library_shuffle_followups,
    },
    SubjectVerbFollowupRuleDef {
        id: "still-lands",
        priority: 20,
        heads: &["theyre", "they", "its", "it"],
        run: pre_rule_still_lands_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "cant-be-regenerated",
        priority: 30,
        heads: &["it", "they", "creature", "creatures", "a"],
        run: pre_rule_cant_be_regenerated_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "damage-cant-be-prevented",
        priority: 35,
        heads: &["the"],
        run: pre_rule_damage_cant_be_prevented_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "copy-and-cast",
        priority: 40,
        heads: &["copy", "that"],
        run: pre_rule_copy_and_cast_followups,
    },
    SubjectVerbFollowupRuleDef {
        id: "draw-count-demonstrative-gain",
        priority: 45,
        heads: &["that", "those", "each", "all"],
        run: pre_rule_draw_count_demonstrative_gain_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "token-followups",
        priority: 50,
        heads: &[],
        run: pre_rule_token_followups,
    },
    SubjectVerbFollowupRuleDef {
        id: "exile-this-way",
        priority: 55,
        heads: &["if"],
        run: pre_rule_exile_this_way_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "source-exiled-return-if-sacrificed",
        priority: 55,
        heads: &["if"],
        run: pre_rule_return_source_exiled_cards_if_source_sacrificed,
    },
    SubjectVerbFollowupRuleDef {
        id: "declined-tagged-battlefield-move",
        priority: 54,
        heads: &["if"],
        run: pre_rule_declined_tagged_battlefield_move_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "milled-this-way",
        priority: 55,
        heads: &["when"],
        run: pre_rule_when_milled_this_way_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "if-no-one-does",
        priority: 55,
        heads: &["if"],
        run: pre_rule_if_no_one_does_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "if-you-win",
        priority: 55,
        heads: &["if"],
        run: pre_rule_if_you_win_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "future-zone-replacement",
        priority: 56,
        heads: &["if"],
        run: pre_rule_future_zone_replacement_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "skip-tapped-source-turn-replacement",
        priority: 57,
        heads: &["if"],
        run: pre_rule_skip_tapped_source_turn_replacement,
    },
    SubjectVerbFollowupRuleDef {
        id: "damage-this-way-player-followup",
        priority: 58,
        heads: &["if", "players"],
        run: pre_rule_damage_this_way_player_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "tap-damage-this-way",
        priority: 58,
        heads: &["tap"],
        run: pre_rule_tap_damage_this_way_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "destroy-those-creatures",
        priority: 59,
        heads: &["destroy", "then"],
        run: pre_rule_destroy_those_creatures_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "otherwise",
        priority: 60,
        heads: &["otherwise"],
        run: pre_rule_otherwise_followup,
    },
];

const POST_PARSE_SUBJECT_VERB_FOLLOWUP_RULES: &[SubjectVerbPostParseRuleDef] = &[
    SubjectVerbPostParseRuleDef {
        id: "token-copy-and-extra-turn",
        priority: 10,
        heads: &[],
        run: post_rule_token_copy_and_extra_turn,
    },
    SubjectVerbPostParseRuleDef {
        id: "future-zone-and-self-replacement",
        priority: 20,
        heads: &[],
        run: post_rule_future_zone_and_self_replacement,
    },
    SubjectVerbPostParseRuleDef {
        id: "hand-reveal-choice-discard-followup",
        priority: 25,
        heads: &["that", "the"],
        run: post_rule_hand_reveal_choice_discard_followup,
    },
    SubjectVerbPostParseRuleDef {
        id: "reflexive-object-followup",
        priority: 27,
        heads: &[],
        run: post_rule_reflexive_object_followup,
    },
    SubjectVerbPostParseRuleDef {
        id: "delayed-trigger-result-followup",
        priority: 30,
        heads: &["if", "when"],
        run: post_rule_delayed_trigger_result_followup,
    },
    SubjectVerbPostParseRuleDef {
        id: "delayed-trigger-copy-retarget-followup",
        priority: 31,
        heads: &["you"],
        run: post_rule_delayed_trigger_copy_retarget_followup,
    },
];

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
                (PlayerAst::Implicit, PlayerAst::That, None),
            ],
            "the default branch carries the target-qualified library; the replacement resolves its demonstrative owner from the retained target declaration"
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
