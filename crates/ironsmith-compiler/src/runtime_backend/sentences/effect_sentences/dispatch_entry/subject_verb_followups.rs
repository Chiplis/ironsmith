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

fn effect_needs_followup_library_shuffle(effect: &EffectAst) -> bool {
    if matches!(
        effect,
        EffectAst::ChooseObjectsAcrossZones { zones, .. } if slice_contains(zones, &Zone::Library)
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

fn is_if_you_search_library_this_way_shuffle_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words: Vec<&str> = crate::runtime_backend::token_word_refs(tokens)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    words.as_slice()
        == [
            "if", "you", "search", "your", "library", "this", "way", "shuffle",
        ]
        || words.as_slice()
            == [
                "if", "you", "search", "your", "library", "this", "way", "shuffles",
            ]
}

fn is_then_that_player_shuffles_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    matches!(
        words.as_slice(),
        ["then", "that", "player", "shuffles"]
            | ["that", "player", "shuffles"]
            | ["then", "that", "player", "shuffle"]
            | ["that", "player", "shuffle"]
    )
}

fn rule_matches_sentence_head(heads: &[&str], tokens: &[OwnedLexToken]) -> bool {
    if heads.is_empty() {
        return true;
    }
    crate::runtime_backend::token_word_refs(tokens)
        .first()
        .is_some_and(|head| heads.iter().any(|candidate| head == candidate))
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
        "copy-and-cast" => "subject-verb verb=Copy subject=implicit recognizer=copy-cast-followup",
        "token-followups" => "subject-verb verb=Create subject=implicit recognizer=token-followup",
        "exile-this-way" => "subject-verb verb=Exile subject=implicit recognizer=this-way-followup",
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
            state.effects.push(EffectAst::subject_verb(
                SubjectVerbRoleAst::LibraryOwner,
                PlayerAst::You,
                SubjectVerbActionAst::ShuffleLibrary,
            ));
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route: None,
            }));
        }
        if state.effects.iter().any(effect_contains_search_library) {
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

fn pre_rule_future_zone_replacement_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let sentence_text = crate::runtime_backend::token_word_refs(sentence_tokens).join(" ");
    if !str_contains(&sentence_text, "would die this turn") {
        return Ok(None);
    }
    if !matches!(
        classify_instead_followup_text(&sentence_text),
        InsteadSemantics::FutureReplacement
    ) {
        return Ok(None);
    }
    let Some(replacement) = future_zone_replacement_from_sentence_text(&sentence_text) else {
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
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
    if words.as_slice()
        != [
            "if", "you", "would", "begin", "your", "turn", "while", "this", "artifact", "is",
            "tapped", "you", "may", "skip", "that", "turn", "instead",
        ]
    {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![EffectAst::Conditional {
            predicate: PredicateAst::SourceIsTapped,
            if_true: vec![EffectAst::subject_verb_skip_turn(PlayerAst::You)],
            if_false: Vec::new(),
        }]),
        consumed_sentences: 1,
    })))
}

fn pre_rule_damage_this_way_player_followup(
    _state: &mut SentenceDispatchState<'_>,
    _sentences: &[SentenceInput],
    _sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
    let words = crate::runtime_backend::token_word_refs(sentence_tokens);
    if !matches!(
        words.as_slice(),
        [
            "if", "a", "player", "is", "dealt", "damage", "this", "way", "they", "cant", "gain",
            "life", "for", "the", "rest", "of", "the", "game"
        ] | [
            "if", "player", "is", "dealt", "damage", "this", "way", "they", "cant", "gain", "life",
            "for", "the", "rest", "of", "the", "game"
        ]
    ) {
        return Ok(None);
    }
    Ok(Some(PreParseFollowupResult::Plan(SentenceParsePlan {
        tokens: sentence_tokens.to_vec(),
        wrap_if_result: None,
        direct_effects: Some(vec![EffectAst::IfResult {
            predicate: IfResultPredicate::Did,
            effects: vec![EffectAst::subject_verb_cant(
                crate::effect::Restriction::gain_life(PlayerFilter::DamagedPlayer),
                crate::effect::Until::Forever,
                None,
            )],
        }]),
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
    let sentence_words = TokenWordView::new(sentence_tokens).to_word_refs();
    matches!(
        sentence_words.as_slice(),
        ["theyre", "still", "land"]
            | ["theyre", "still", "lands"]
            | ["they", "re", "still", "land"]
            | ["they", "re", "still", "lands"]
            | ["its", "still", "a", "land"]
            | ["its", "still", "land"]
            | ["it", "s", "still", "a", "land"]
            | ["it", "s", "still", "land"]
    )
}

pub(super) fn previous_sentence_is_temporary_land_animation(
    sentences: &[SentenceInput],
    sentence_idx: usize,
) -> bool {
    sentence_idx
        .checked_sub(1)
        .and_then(|idx| sentences.get(idx))
        .is_some_and(|previous_sentence| {
            let previous_words =
                crate::runtime_backend::token_word_refs(previous_sentence.lowered());
            previous_words
                .iter()
                .any(|word| *word == "become" || *word == "becomes")
                && previous_words
                    .iter()
                    .any(|word| *word == "creature" || *word == "creatures")
                && previous_words
                    .windows(4)
                    .any(|window| window == ["until", "end", "of", "turn"])
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
        crate::runtime_backend::token_word_refs(sentence_tokens).join(" ")
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
            crate::runtime_backend::token_word_refs(sentence_tokens).join(" ")
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

fn pre_rule_token_followups(
    state: &mut SentenceDispatchState<'_>,
    sentences: &[SentenceInput],
    sentence_idx: usize,
    sentence_tokens: &[OwnedLexToken],
) -> Result<Option<PreParseFollowupResult>, CardTextError> {
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
            crate::runtime_backend::token_word_refs(sentence_tokens).join(" ")
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
    if is_generic_token_reminder_sentence(sentence_tokens)
        && state.effects.last().is_some_and(effect_creates_any_token)
    {
        if append_token_reminder_to_last_create_effect(state.effects, sentence_tokens) {
            let reminder_words = crate::runtime_backend::token_word_refs(sentence_tokens);
            let route = matches!(
                reminder_words.as_slice(),
                ["exile", ..] | ["sacrifice", ..]
            )
            .then_some(
                "subject-verb verb=Exile subject=implicit recognizer=token-copy-delayed-followup",
            );
            return Ok(Some(PreParseFollowupResult::Handled {
                consumed_sentences: 1,
                route,
            }));
        }
        return Err(CardTextError::ParseError(format!(
            "unsupported standalone token reminder clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(sentence_tokens).join(" ")
        )));
    }
    if is_generic_token_reminder_sentence(sentence_tokens) {
        let reminder_words = crate::runtime_backend::token_word_refs(sentence_tokens);
        let delayed_pronoun_lifecycle =
            matches!(reminder_words.first().copied(), Some("exile" | "sacrifice"))
                && (grammar::contains_word(sentence_tokens, "it")
                    || grammar::contains_word(sentence_tokens, "them"));
        let pronoun_followup_clause =
            grammar::words_match_any_prefix(sentence_tokens, PRONOUN_TRIGGER_PREFIXES).is_some();
        if !delayed_pronoun_lifecycle && !pronoun_followup_clause {
            return Err(CardTextError::ParseError(format!(
                "unsupported standalone token reminder clause (clause: '{}')",
                crate::runtime_backend::token_word_refs(sentence_tokens).join(" ")
            )));
        }
    }
    if let Some(effect) = parse_choose_target_prelude_sentence(sentence_tokens)? {
        state.effects.push(effect);
        *state.carried_context = None;
        return Ok(Some(PreParseFollowupResult::Handled {
            consumed_sentences: 1,
            route: None,
        }));
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
        )?);
        return Ok(Some(PreParseFollowupResult::Plan(plan)));
    }
    Ok(None)
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
    let has_expected_prefix = grammar::words_match_prefix(
        tokens,
        &[
            "if", "a", "card", "is", "put", "into", "exile", "this", "way",
        ],
    )
    .is_some()
        || grammar::words_match_prefix(
            tokens,
            &["if", "card", "is", "put", "into", "exile", "this", "way"],
        )
        .is_some()
        || grammar::words_match_prefix(
            tokens,
            &[
                "if", "a", "card", "was", "put", "into", "exile", "this", "way",
            ],
        )
        .is_some()
        || grammar::words_match_prefix(
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
            crate::runtime_backend::token_word_refs(sentence_tokens).join(" ")
        )));
    };

    let mut plan = SentenceParsePlan::new(trim_commas(after).to_vec());
    plan.wrap_if_result = Some(IfResultPredicate::Did);
    Ok(Some(PreParseFollowupResult::Plan(plan)))
}

fn is_destroy_those_creatures_sentence(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    matches!(
        words.as_slice(),
        ["destroy", "those", "creatures"] | ["then", "destroy", "those", "creatures"]
    )
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
    let sentence_text = crate::runtime_backend::token_word_refs(sentence_tokens).join(" ");
    maybe_rewrite_future_zone_replacement_sentence(sentence_effects, &sentence_text);
    if matches!(
        classify_instead_followup_text(&sentence_text),
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
            if let Some(target) = previous_target {
                replace_it_target_in_effects(&mut if_true, &target);
            }
            if let Some(target) = previous_damage_target {
                replace_it_damage_target_in_effects(&mut if_true, &target);
                replace_placeholder_damage_target_in_effects(&mut if_true, &target);
            }
            if_false.insert(0, previous);
            state.effects.push(EffectAst::SelfReplacement {
                predicate,
                if_true,
                if_false,
            });
            return Ok(Some(PostParseFollowupResult::Handled {
                consumed_sentences: 1,
            }));
        }
    }
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
    let Some(EffectAst::DelayedTriggerThisTurn { effects, .. }) = state.effects.last_mut() else {
        return Ok(None);
    };
    effects.extend(sentence_effects.drain(..));
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
        heads: &["it", "they"],
        run: pre_rule_cant_be_regenerated_followup,
    },
    SubjectVerbFollowupRuleDef {
        id: "copy-and-cast",
        priority: 40,
        heads: &["copy", "that"],
        run: pre_rule_copy_and_cast_followups,
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
        heads: &["if"],
        run: pre_rule_damage_this_way_player_followup,
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
];
