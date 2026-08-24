use super::*;
use crate::ChoiceCount;
use crate::cards::builders::SubjectVerbSubjectAst;
use crate::grammar::effects::followup_shapes;
use crate::grammar::structure::parse_trailing_if_predicate_lexed;

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
        || crate::word_primitives::parse_any_sequence_prefix(
            &authored_words,
            &[&["each", "of", "those"], &["all", "of", "those"]],
        ))
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
#[path = "subject_verb_followups_inline_demonstrative_grant_surface_tests.rs"]
mod demonstrative_grant_surface_tests;

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
    let words = crate::util::non_article_token_word_refs(tokens);
    if words.len() < 7
        || !crate::word_primitives::parse_sequence_prefix(
            &words,
            &["if", "you", "do", "return", "those"],
        )
    {
        return false;
    }
    if words.get(5) != Some(&"cards") || words.get(6) != Some(&"to") {
        return false;
    }
    crate::word_primitives::sequence_occurs(&words, &["battlefield"])
        && words
            .iter()
            .any(|word| matches!(*word, "owner" | "owners" | "owner's" | "owners'"))
        && crate::word_primitives::sequence_occurs(&words, &["control"])
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
        .is_some_and(|head| heads.iter().any(|candidate| candidate == &head))
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
    let words = crate::lexer::parser_token_word_refs(sentence_tokens);
    let expected_head = ["you", "may", "exile", "it", "and", "collect", "evidence"];
    if words.len() != 8
        || !words[..7]
            .iter()
            .zip(expected_head)
            .all(|(word, expected)| word.eq_ignore_ascii_case(expected))
    {
        return Ok(None);
    }
    let Some(amount) = crate::util::parse_number_word_u32(words[7]) else {
        return Ok(None);
    };
    let Some(followup) = sentences.get(sentence_idx + 1) else {
        return Ok(None);
    };
    let followup_words = crate::lexer::token_word_refs(followup.lexed());
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

    let source_exiled_tag = crate::util::helper_tag_for_tokens(sentence_tokens, "exiled");
    let evidence_tag = crate::util::helper_tag_for_tokens(sentence_tokens, "evidence");
    let evidence_filter = ObjectFilter::default()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You)
        .match_tagged(
            crate::tag::CompilerReferenceTag::Triggering.key(),
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
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), None),
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
#[path = "subject_verb_followups_inline_collect_evidence_followup_tests_2.rs"]
mod collect_evidence_followup_tests;

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
                tag: crate::tag::CompilerReferenceTag::Damaged0.key(),
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
            crate::tag::CompilerReferenceTag::Damaged0.key(),
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
        crate::activation_and_restrictions::parse_copy_reference_cost_reduction_sentence(
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
    let Some(comma_idx) =
        crate::slice_primitives::select_position(sentence_tokens, OwnedLexToken::is_comma)
    else {
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
    let Some(comma_idx) =
        crate::slice_primitives::select_position(sentence_tokens, OwnedLexToken::is_comma)
    else {
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
    let predicate = crate::grammar::filters::parse_condition_predicate_lexed(&predicate_tokens)?;

    effects.pop();
    effects.push(EffectAst::Conditional {
        predicate,
        if_true: modified,
        if_false: vec![previous],
    });
    Ok(true)
}

#[cfg(test)]
#[path = "subject_verb_followups_inline_moved_object_entry_followup_tests_3.rs"]
mod moved_object_entry_followup_tests;

struct PriorTokenCreateFollowup {
    predicate: PredicateAst,
    create: EffectAst,
    instead: bool,
}

#[cfg(test)]
#[path = "subject_verb_followups_inline_choose_for_each_player_instead_tests_4.rs"]
mod choose_for_each_player_instead_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_numeric_result_branch_label_tests_5.rs"]
mod numeric_result_branch_label_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_each_player_coin_face_followup_tests_6.rs"]
mod each_player_coin_face_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_revealed_same_mana_value_iterator_tests_7.rs"]
mod revealed_same_mana_value_iterator_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_correlated_plural_sacrifice_result_tests_8.rs"]
mod correlated_plural_sacrifice_result_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_delayed_copy_retarget_followup_tests_9.rs"]
mod delayed_copy_retarget_followup_tests;

const PRE_PARSE_SUBJECT_VERB_FOLLOWUP_RULES: &[SubjectVerbFollowupRuleDef] = &[
    SubjectVerbFollowupRuleDef {
        id: "prepare-each-player-coin-face-followup",
        priority: 4,
        heads: &["each"],
        run: pre_rule_each_player_coin_face_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "optional-source-exile-and-collect-evidence",
        priority: 5,
        heads: &["you"],
        run: pre_rule_optional_source_exile_and_collect_evidence,
    },
    SubjectVerbFollowupRuleDef {
        id: "prepare-returned-permanent-enters-followup",
        priority: 5,
        heads: &["when"],
        run: pre_rule_returned_permanent_enters,
    },
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
        heads: &["it", "they", "those", "creature", "creatures", "a"],
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
        heads: &["copy", "that", "you", "the"],
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
        priority: 42,
        heads: &[],
        run: pre_rule_token_followups,
    },
    SubjectVerbFollowupRuleDef {
        id: "moved-object-entry-followup",
        priority: 43,
        heads: &["it"],
        run: pre_rule_moved_object_entry_followup,
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
        id: "choose-for-each-player-instead",
        priority: 55,
        heads: &["if"],
        run: pre_rule_choose_for_each_player_instead,
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
        id: "numeric-result-branch-label",
        priority: 5,
        heads: &[],
        run: post_rule_numeric_result_branch_label,
    },
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
        id: "each-player-coin-face-followup",
        priority: 22,
        heads: &["each"],
        run: post_rule_each_player_coin_face_followup,
    },
    SubjectVerbPostParseRuleDef {
        id: "typed-sacrificed-result-iterator",
        priority: 23,
        heads: &["for"],
        run: post_rule_typed_sacrificed_result_iterator,
    },
    SubjectVerbPostParseRuleDef {
        id: "revealed-same-mana-value-as-another-iterator",
        priority: 23,
        heads: &["for"],
        run: post_rule_revealed_same_mana_value_as_another_iterator,
    },
    SubjectVerbPostParseRuleDef {
        id: "correlated-plural-sacrifice-result",
        priority: 24,
        heads: &["those"],
        run: post_rule_correlated_plural_sacrifice_result,
    },
    SubjectVerbPostParseRuleDef {
        id: "hand-reveal-choice-discard-followup",
        priority: 25,
        heads: &["that", "the"],
        run: post_rule_hand_reveal_choice_discard_followup,
    },
    SubjectVerbPostParseRuleDef {
        id: "prior-exiled-card-reference",
        priority: 26,
        heads: &[],
        run: post_rule_prior_exiled_card_reference,
    },
    SubjectVerbPostParseRuleDef {
        id: "consult-remainder-reference",
        priority: 26,
        heads: &["put", "shuffle", "that", "then"],
        run: post_rule_consult_remainder_reference,
    },
    SubjectVerbPostParseRuleDef {
        id: "returned-permanent-enters",
        priority: 26,
        heads: &["when"],
        run: post_rule_returned_permanent_enters,
    },
    SubjectVerbPostParseRuleDef {
        id: "targeted-object-delayed-leave",
        priority: 26,
        heads: &["when", "whenever"],
        run: post_rule_targeted_object_delayed_leave,
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
    SubjectVerbPostParseRuleDef {
        id: "optional-copy-retarget-followup",
        priority: 32,
        heads: &["the"],
        run: post_rule_optional_copy_retarget_followup,
    },
    SubjectVerbPostParseRuleDef {
        id: "self-replacement-common-suffix",
        priority: 100,
        heads: &[],
        run: post_rule_self_replacement_common_suffix,
    },
];

#[cfg(test)]
#[path = "subject_verb_followups_inline_retained_land_followup_tests_10.rs"]
mod retained_land_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_copy_cast_followup_tests_11.rs"]
mod copy_cast_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_revealed_hand_actor_tests_12.rs"]
mod revealed_hand_actor_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_declined_move_followup_tests_13.rs"]
mod declined_move_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_damage_self_replacement_followup_tests_14.rs"]
mod damage_self_replacement_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_counter_self_replacement_followup_tests_15.rs"]
mod counter_self_replacement_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_prior_token_copy_self_replacement_tests_16.rs"]
mod prior_token_copy_self_replacement_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_targeted_delayed_leave_followup_tests_17.rs"]
mod targeted_delayed_leave_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_returned_permanent_enters_followup_tests_18.rs"]
mod returned_permanent_enters_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_conditional_target_self_replacement_followup_tests_19.rs"]
mod conditional_target_self_replacement_followup_tests;

#[cfg(test)]
#[path = "subject_verb_followups_inline_targeted_search_self_replacement_followup_tests_20.rs"]
mod targeted_search_self_replacement_followup_tests;

#[path = "subject_verb_followups/subject_verb_followups_reference_programs.rs"]
mod subject_verb_followups_reference_programs;
pub(super) use subject_verb_followups_reference_programs::transport_copy_retarget_into_trailing_optional_copy;
use subject_verb_followups_reference_programs::{
    append_copy_retarget_to_trailing_optional_copy,
    append_moved_object_entry_followup_to_optional_move,
    bind_self_replacement_condition_to_previous_target, bind_targeted_leaves_filter,
    bind_that_player_subjects, bind_that_player_subjects_in_effects, carried_player_from_effect,
    effect_has_that_player_subject, effects_are_copy_retarget_followup,
    effects_are_one_copy_retarget_followup, effects_copy_a_stack_object,
    last_remove_abilities_all_filter, post_rule_consult_remainder_reference,
    post_rule_each_player_coin_face_followup, post_rule_optional_copy_retarget_followup,
    pre_rule_declined_tagged_battlefield_move_followup, pre_rule_each_player_coin_face_followup,
    pre_rule_moved_object_entry_followup, rebind_source_match_to_target, tag_latest_prior_exile,
    tagged_may_battlefield_move, tagged_object_reference, target_is_explicitly_a_land,
};
#[path = "subject_verb_followups/subject_verb_followups_trigger_programs.rs"]
mod subject_verb_followups_trigger_programs;
pub(super) use subject_verb_followups_trigger_programs::transport_copy_retarget_into_trailing_delayed_trigger;
use subject_verb_followups_trigger_programs::{
    append_copy_retarget_to_trailing_delayed_trigger,
    bind_demonstrative_land_match_to_triggering_object,
    post_rule_delayed_trigger_copy_retarget_followup, post_rule_delayed_trigger_result_followup,
    post_rule_reflexive_object_followup, post_rule_targeted_object_delayed_leave,
    replace_event_amount_with_value, trailing_delayed_trigger_effects_mut,
};
#[path = "subject_verb_followups/subject_verb_followups_object_action_programs.rs"]
mod subject_verb_followups_object_action_programs;
use subject_verb_followups_object_action_programs::{
    parse_create_more_of_prior_tokens, post_rule_token_copy_and_extra_turn,
    pre_rule_token_followups, trailing_optional_copy_effects_mut,
};
#[path = "subject_verb_followups/subject_verb_followups_zone_programs.rs"]
mod subject_verb_followups_zone_programs;
use subject_verb_followups_zone_programs::{
    is_explicit_return_to_battlefield, is_singular_explicit_return_to_battlefield,
    post_rule_returned_permanent_enters, pre_rule_returned_permanent_enters,
};
#[path = "subject_verb_followups/subject_verb_followups_library_programs.rs"]
mod subject_verb_followups_library_programs;
use subject_verb_followups_library_programs::{
    bind_cast_tag_to_prior_exiled_card, bind_prior_exiled_card_to_source_link,
    bind_self_replacement_search_owner, chosen_card_tag_from_hand_choice_branch,
    effect_references_prior_exiled_card, first_library_search_shape, first_search_library_owner,
    is_dependent_that_player_discard, is_if_card_put_into_exile_this_way_sentence,
    materialize_search_count_self_replacement, mill_count_from_effect,
    post_rule_hand_reveal_choice_discard_followup, post_rule_prior_exiled_card_reference,
    post_rule_revealed_same_mana_value_as_another_iterator, pre_rule_when_milled_this_way_followup,
    preserve_search_owner_anaphor_in_self_replacement, replace_matching_library_search_count,
    replace_mill_event_amounts_with_value,
};
#[path = "subject_verb_followups/subject_verb_followups_resource_programs.rs"]
mod subject_verb_followups_resource_programs;
use subject_verb_followups_resource_programs::{
    bind_prior_exiled_mana_value, effects_contain_gain_life,
    post_rule_correlated_plural_sacrifice_result, post_rule_typed_sacrificed_result_iterator,
    pre_rule_draw_count_demonstrative_gain_followup,
};
#[path = "subject_verb_followups/subject_verb_followups_condition_programs.rs"]
mod subject_verb_followups_condition_programs;
pub(super) use subject_verb_followups_condition_programs::post_rule_future_zone_and_self_replacement;
use subject_verb_followups_condition_programs::{
    default_effects_for_self_replacement, post_rule_self_replacement_common_suffix,
    pre_rule_if_no_one_does_followup, pre_rule_if_you_win_followup,
    predicate_explicitly_says_that_land, take_self_replacement_condition,
};
#[path = "subject_verb_followups/subject_verb_followups_core_programs.rs"]
mod subject_verb_followups_core_programs;
use subject_verb_followups_core_programs::{
    is_destroy_those_creatures_sentence, post_rule_numeric_result_branch_label,
    pre_rule_destroy_those_creatures_followup, pre_rule_otherwise_followup,
};
#[path = "subject_verb_followups/subject_verb_followups_choice_programs.rs"]
mod subject_verb_followups_choice_programs;
use subject_verb_followups_choice_programs::{
    pre_rule_choose_for_each_player_instead, rewrite_each_player_choice_complement_chooser,
    target_is_explicitly_chosen,
};
#[path = "subject_verb_followups/subject_verb_followups_combat_programs.rs"]
mod subject_verb_followups_combat_programs;
use subject_verb_followups_combat_programs::{
    normalize_anaphoric_damage_self_replacement, primary_damage_source_from_effect,
    replace_anaphoric_damage_source_in_effects, sole_damage_payload,
};
#[path = "subject_verb_followups/subject_verb_followups_permission_programs.rs"]
mod subject_verb_followups_permission_programs;
use subject_verb_followups_permission_programs::pre_rule_exile_this_way_followup;
