use crate::cards::builders::SubjectVerbSubjectAst;
use crate::runtime_backend::grammar::effects::followup_shapes;
use crate::runtime_backend::grammar::structure::parse_trailing_if_predicate_lexed;

use super::*;

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
            let Value::Count(filter) = count else {
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
            },
        ..
    }) = effect
    else {
        return false;
    };
    *count == 1
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
                predicate: IfResultPredicate::Did,
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
    if is_still_lands_followup
        && (state.effects.iter().rev().any(|effect| {
            matches!(
                effect,
                EffectAst::SubjectVerb(SubjectVerbEffectAst {
                    action: SubjectVerbActionAst::BecomeBasePtCreature { .. }
                        | SubjectVerbActionAst::AddCardTypes { .. },
                    ..
                })
            )
        }) || previous_sentence_is_land_animation)
    {
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
    }
    Ok(None)
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
    if !is_cant_be_regenerated_followup_sentence(sentence_tokens) {
        return Ok(None);
    }
    if apply_cant_be_regenerated_to_last_destroy_effect(state.effects) {
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
    if let Some((mut copy_effects, spec)) =
        parse_same_sentence_copy_and_may_cast_copy(sentence_tokens)?
    {
        state.effects.append(&mut copy_effects);
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
            let mut effects = parse_effect_sentence_lexed(sentence_tokens)?;
            effects.push(build_may_cast_tagged_effect(&spec));
            return Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
                tokens: sentence_tokens.to_vec(),
                wrap_if_result: None,
                direct_effects: Some(effects),
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
    if let Some(effect) = parse_create_more_of_prior_tokens(sentence_tokens, state.effects) {
        state.effects.push(effect);
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
        if append_token_reminder_to_last_create_effect(state.effects, reminder_tokens) {
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

fn parse_create_more_of_prior_tokens(
    sentence_tokens: &[OwnedLexToken],
    prior_effects: &[EffectAst],
) -> Option<EffectAst> {
    let shape = followup_shapes::parse_create_more_prior_tokens(sentence_tokens)?;
    let predicate = parse_trailing_if_predicate_lexed(shape.predicate_tokens)?;
    let (name, definition, player) = last_created_token_info(prior_effects)?;

    let create = EffectAst::subject_verb(
        SubjectVerbRoleAst::Actor,
        player,
        SubjectVerbActionAst::CreateTokenWithMods {
            name,
            definition,
            count: Value::Fixed(shape.count as i32),
            dynamic_power_toughness: None,
            player,
            attached_to: None,
            tapped: false,
            attacking: false,
            exile_at_end_of_combat: false,
            sacrifice_at_end_of_combat: false,
            sacrifice_at_next_end_step: false,
            exile_at_next_end_step: false,
            next_end_step_player: PlayerFilter::Any,
            granted_abilities: Vec::new(),
        },
    );

    Some(EffectAst::Conditional {
        predicate,
        if_true: vec![create],
        if_false: Vec::new(),
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
            Some(EffectAst::Conditional { .. })
        )
    {
        let Some(previous) = state.effects.pop() else {
            return Err(CardTextError::InvariantViolation(
                "expected previous effect for 'instead' conditional rewrite".to_string(),
            ));
        };
        let previous_target = primary_target_from_effect(&previous);
        let previous_damage_target = primary_damage_target_from_effect(&previous);
        if let Some(EffectAst::Conditional {
            predicate,
            mut if_true,
            mut if_false,
        }) = sentence_effects.pop()
        {
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
            if let Some(target) = previous_target {
                replace_it_target_in_effects(&mut if_true, &target);
            }
            if let Some(target) = previous_damage_target {
                replace_it_damage_target_in_effects(&mut if_true, &target);
                replace_placeholder_damage_target_in_effects(&mut if_true, &target);
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
    let Some(CarryContext::Player(player)) = explicit_player_for_carry(effect) else {
        return None;
    };
    if matches!(player, PlayerAst::That | PlayerAst::Implicit) {
        None
    } else {
        Some(player)
    }
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
    if let EffectAst::SubjectVerb(SubjectVerbEffectAst { subject, .. }) = effect
        && subject.player == PlayerAst::That
    {
        subject.player = player;
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
        | EffectAst::DelayedUntilNextEndStep { effects, .. }
        | EffectAst::DelayedUntilNextUpkeep { effects, .. }
        | EffectAst::DelayedUntilNextDrawStep { effects, .. },
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
