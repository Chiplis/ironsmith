const LABELED_ROUND_UP_EACH_TIME_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["round", "up", "each", "time"]);
const LABELED_THE_NEXT_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["the", "next"]);
const LABELED_CAST_FROM_AMONG_FREE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["you", "may", "cast"];
    contains_phrases & [
        &["from", "among", "them"],
        &["without", "paying", "its", "mana", "cost"],
    ]
);
const LABELED_CAST_HAND_FREE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["you", "may", "cast", "a", "spell", "from", "your", "hand"];
    contains_phrases & [&["without", "paying", "its", "mana", "cost"]]
);
const LABELED_EXILE_ALL_CARDS_FROM_HAND_GRAVEYARD_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["exile", "all", "cards", "from"];
    contains_any_words & [&["hand", "hands"], &["graveyard", "graveyards"]]
);
const LABELED_UNLESS_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["unless"]);
const LABELED_GAIN_LOSE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["gain", "gains", "lose", "loses"]]);
const LABELED_VOTE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["vote", "votes"]]);
const LABELED_RETURN_ROUNDED_UP_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["return"]; contains_words & ["rounded", "up"]);
const LABELED_CHOOSE_DO_SAME_FOR_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix & ["choose"]; contains_phrases & [&["do", "the", "same", "for"]]);
const LABELED_EACH_PLAYER_CHOOSE_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["each", "player", "choose"], &["each", "player", "chooses"]]);
const LABELED_CAST_ANY_NUMBER_GRAVEYARD_FREE_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix & ["cast", "any", "number", "of"];
    contains_phrases & [
        &["from", "your", "graveyard"],
        &["without", "paying", "their", "mana", "costs"],
    ]
);
const LABELED_TAP_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(prefix & ["tap"]);
const LABELED_PREVENT_TAKE_MONSTROSITY_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["prevent"], &["take"], &["monstrosity"]]);
const LABELED_ENCHANT_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["enchant"]);
const LABELED_EARTHBEND_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["earthbend"]);
const LABELED_SACRIFICE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(exact & ["sacrifice"]);
const LABELED_ALL_OR_EACH_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["all"], &["each"]]);
const LABELED_IS_OR_ARE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["is"], &["are"]]);
const LABELED_ARTICLE_AND_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["a"], &["an"], &["and"]]);
const LABELED_GAIN_OR_GAINS_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"]]);
const LABELED_GAIN_HAS_LOSE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["gain"], &["gains"], &["has"], &["have"], &["lose"], &["loses"]]);
const LABELED_HAS_HAVE_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["has"], &["have"]]);
const LABELED_SIMPLE_ABILITY_TAIL_STOP_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["and"], &["then"], &["if"]]);
const LABELED_TRIGGER_WORD_PATTERN: ClauseShape<'static> =
    clause_shape!(exact_any & [&["when"], &["whenever"]]);
const LABELED_OR_UNTAP_ALL_EACH_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_phrases & [&[&["or", "untap", "all"], &["or", "untap", "each"]]]);
const LABELED_SIMPLE_ABILITY_EXCLUSION_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_any_words & [&["shares", "choice"]]);
const LABELED_ANOTHER_HASTE_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_words & ["another", "haste"]);
const LABELED_LIFE_WORD_PATTERN: ClauseShape<'static> = clause_shape!(contains_words & ["life"]);

fn labeled_mana_value_or_less_bound(tokens: &[OwnedLexToken]) -> Option<u32> {
    let mut search_start = 0usize;
    while search_start < tokens.len() {
        let relative_idx =
            find_token_word_sequence(&tokens[search_start..], &["mana", "value"])?;
        let phrase_start = search_start + relative_idx;
        let tail_start = phrase_start + 2;
        let tail = tokens.get(tail_start..)?;
        let Some((count, _used)) =
            crate::runtime_backend::util::parse_less_than_or_equal_quantity_prefix(
                tail,
                false,
                false,
                "mana value bound",
            )
            .ok()
            .flatten()
        else {
            search_start = tail_start;
            continue;
        };
        return Some(count);
    }
    None
}

const LABELED_SIMPLE_ABILITY_WORD_PATTERN: ClauseShape<'static> = clause_shape!(
    contains_any_words
        & [&[
            "indestructible",
            "haste",
            "flying",
            "vigilance",
            "lifelink",
            "trample",
            "reach",
            "menace",
            "fear",
            "deathtouch",
            "horsemanship",
            "hexproof",
            "shroud",
            "shadow",
            "strike",
            "protection",
            "blocked",
            "abilities",
            "when",
            "whenever",
        ]]
);
const LABELED_TOKEN_SACRIFICE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["sacrifice", "it"],
            &["sacrifice", "them"],
            &["sacrifice", "that", "token"],
            &["sacrifice", "those", "tokens"],
        ]
);
const LABELED_TOKEN_EXILE_PREFIX_PATTERN: ClauseShape<'static> =
    clause_shape!(prefix_any & [&["exile", "it"], &["exile", "them"]]);
const LABELED_NEXT_END_STEP_PATTERN: ClauseShape<'static> =
    clause_shape!(contains_phrases & [&["at", "beginning", "of", "next", "end", "step"]]);
const LABELED_DELAYED_END_STEP_SACRIFICE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "sacrifice",
            ],
        ]
);
const LABELED_DELAYED_END_STEP_EXILE_PREFIX_PATTERN: ClauseShape<'static> = clause_shape!(
    prefix_any
        & [
            &["at", "the", "beginning", "of", "the", "end", "step", "exile"],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "next",
                "end",
                "step",
                "exile",
            ],
            &["at", "the", "beginning", "of", "next", "end", "step", "exile"],
        ]
);


pub(crate) fn parse_effect_sentence_inner_lexed(
    tokens: &[OwnedLexToken],
) -> Result<Vec<EffectAst>, CardTextError> {
    fn contains_unquoted_word(tokens: &[OwnedLexToken], words: &[&str]) -> bool {
        let mut inside_quotes = false;
        for token in tokens {
            if token.is_quote() {
                inside_quotes = !inside_quotes;
                continue;
            }
            if !inside_quotes
                && token
                    .as_word()
                    .is_some_and(|word| crate::word_primitives::contains_word(words, word))
            {
                return true;
            }
        }
        false
    }

    let word_view = LexClauseView::from_tokens(tokens);
    let sentence_words = word_view.words.to_word_refs();
    if is_activate_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if is_trigger_only_restriction_sentence_lexed(tokens) {
        return Ok(Vec::new());
    }
    if LABELED_ROUND_UP_EACH_TIME_PATTERN.matches_words(sentence_words.as_slice()) {
        return Ok(Vec::new());
    }

    if let Some(stripped) = split_labeled_effect_prefix_lexed(tokens) {
        return parse_effect_sentence_lexed(stripped);
    }
    if token_slice_first_is(tokens, "if")
        && let Some(mut effects) = parse_exile_replacement_subject_verb_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if token_slice_first_is(tokens, "if")
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &PRE_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(effects) = parse_next_spell_grant_sentence_lexed(tokens)? {
        return Ok(effects);
    }
    if let Some(effect) = parse_matching_spell_cost_reduction_this_turn_sentence_lexed(tokens) {
        return Ok(vec![effect]);
    }
    if LABELED_PREVENT_TAKE_MONSTROSITY_WORD_PATTERN.matches_word_at(&sentence_words, 0)
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(effects) =
        parse_conditional_sentence_family_lexed(tokens, parse_effect_chain_lexed)?
    {
        return Ok(effects);
    }
    if token_slice_first_is(tokens, "exile")
        && grammar::contains_word(tokens, "then")
    {
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
        if let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if token_slice_first_is(tokens, "then") && tokens.len() > 1 {
        return parse_effect_sentence_lexed(&tokens[1..]);
    }
    if let Some(prefix) = split_leading_result_prefix_lexed(tokens) {
        return Ok(vec![match prefix.kind {
            LeadingResultPrefixKind::If => EffectAst::IfResult {
                predicate: prefix.predicate,
                effects: super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
            },
            LeadingResultPrefixKind::When => EffectAst::WhenResult {
                predicate: prefix.predicate,
                effects: super::parse_effect_chain_inner_lexed(prefix.trailing_tokens)?,
            },
        }]);
    }
    if LABELED_CAST_FROM_AMONG_FREE_PATTERN.matches_words(sentence_words.as_slice()) {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.card_types.push(CardType::Instant);
        filter.card_types.push(CardType::Sorcery);
        filter.card_types.push(CardType::Artifact);
        filter.card_types.push(CardType::Creature);
        filter.card_types.push(CardType::Enchantment);
        filter.card_types.push(CardType::Planeswalker);
        filter.card_types.push(CardType::Battle);
        filter.type_or_subtype_union = true;
        if let Some(bound) = labeled_mana_value_or_less_bound(tokens) {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(bound as i32));
        }
        let chosen = TagKey::from("__chosen_cast_from_among");
        return Ok(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                },
            }),
        ]);
    }
    if LABELED_CAST_HAND_FREE_PATTERN.matches_words(sentence_words.as_slice()) {
        let chosen = TagKey::from("__chosen_hand_spell_to_cast");
        let filter = ObjectFilter::nonland()
            .in_zone(Zone::Hand)
            .owned_by(PlayerFilter::You);
        return Ok(vec![
            EffectAst::ChooseObjects {
                filter,
                count: ChoiceCount::exactly(1),
                count_value: None,
                player: PlayerAst::You,
                tag: chosen.clone(),
            },
            EffectAst::SubjectVerb(SubjectVerbEffectAst {
                subject: crate::runtime_backend::ast::SubjectVerbSubjectAst {
                    role: SubjectVerbRoleAst::Actor,
                    player: PlayerAst::You,
                },
                action: SubjectVerbActionAst::CastTagged {
                    tag: chosen,
                    player: PlayerAst::You,
                    allow_land: false,
                    as_copy: false,
                    without_paying_mana_cost: true,
                    cost_reduction: None,
                },
            }),
        ]);
    }
    if contains_unquoted_word(tokens, &["search", "searches"])
        && let Some(mut effects) = parse_search_library_sentence_lexed(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_EXILE_ALL_CARDS_FROM_HAND_GRAVEYARD_PATTERN
        .matches_words(sentence_words.as_slice())
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_ENCHANT_WORD_PATTERN.matches_word_at(&sentence_words, 0)
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_EARTHBEND_WORD_PATTERN.matches_word_at(&sentence_words, 0)
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_UNLESS_PATTERN.matches_words(sentence_words.as_slice())
        && let Some(mut effects) =
            super::parse_sentence_unless_pays(super::SubjectVerbPrimitiveClause::new(tokens))?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_UNLESS_PATTERN.matches_words(sentence_words.as_slice())
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_GAIN_LOSE_WORD_PATTERN.matches_words(sentence_words.as_slice()) {
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
        if let Ok(mut effects) = parse_effect_chain_lexed(tokens) {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
        if let Ok(mut effect) = super::parse_effect_clause_with_trailing_if(tokens) {
            apply_where_x_to_damage_amounts(tokens, std::slice::from_mut(&mut effect))?;
            return Ok(vec![effect]);
        }
        if let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if LABELED_VOTE_WORD_PATTERN.matches_words(sentence_words.as_slice())
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_RETURN_ROUNDED_UP_PATTERN.matches_words(sentence_words.as_slice())
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_CHOOSE_DO_SAME_FOR_PATTERN.matches_words(sentence_words.as_slice())
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_EACH_PLAYER_CHOOSE_PATTERN.matches_words(sentence_words.as_slice()) {
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if LABELED_CAST_ANY_NUMBER_GRAVEYARD_FREE_PATTERN.matches_words(sentence_words.as_slice()) {
        let mut filter = ObjectFilter::default();
        filter.card_types.push(CardType::Instant);
        filter.card_types.push(CardType::Sorcery);
        filter.type_or_subtype_union = true;
        filter.colors = Some(crate::color::ColorSet::from(crate::color::Color::Red));
        let tag = TagKey::from("__chosen_cast_from_graveyard");
        return Ok(vec![
            EffectAst::ChooseObjectsAcrossZones {
                filter,
                count: ChoiceCount::any_number(),
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
                zones: vec![Zone::Graveyard],
                search_mode: Some(crate::effect::SearchSelectionMode::Optional),
            },
            EffectAst::subject_verb_cast_tagged(
                tag,
                PlayerAst::You,
                false,
                false,
                true,
                None,
            ),
        ]);
    }
    if let Some(diag) = super::sentence_unsupported::diagnose_sentence_unsupported_lexed(tokens) {
        return Err(diag);
    }
    if super::parse_leading_player_may_lexed(tokens).is_some() {
        return parse_effect_chain_lexed(tokens);
    }
    if super::looks_like_multi_create_chain_lexed(tokens) {
        if let Some(unless_action) = super::parse_or_action_clause_lexed(tokens)? {
            return Ok(vec![unless_action]);
        }
        let mut effects = super::parse_effect_chain_inner_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    let sacrifice_counted_prefix = matches!(
        sentence_words.as_slice(),
        ["sacrifice", "any", "number", ..] | ["sacrifice", "one", "or", "more", ..]
    );
    if LABELED_SACRIFICE_WORD_PATTERN.matches_word_at(&sentence_words, 0)
        && !sacrifice_counted_prefix
    {
        let mut effects = parse_effect_chain_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if LABELED_TAP_PREFIX_PATTERN.matches_words(sentence_words.as_slice())
        && LABELED_ALL_OR_EACH_WORD_PATTERN.matches_word_at(&sentence_words, 1)
        && LABELED_OR_UNTAP_ALL_EACH_PATTERN.matches_words(sentence_words.as_slice())
    {
        let mut effects = super::parse_effect_chain_with_subject_verb_primitives_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }

    let (_, effects) = super::sentence_registry::run_sentence_parse_rules_lexed(tokens)?;
    Ok(effects)
}

fn parse_matching_spell_cost_reduction_this_turn_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<EffectAst> {
    let words = TokenWordView::new(tokens);
    let clause_words = words.to_word_refs();
    let spell_idx = word_slice_find_word(clause_words.as_slice(), "spells")
        .or_else(|| word_slice_find_word(clause_words.as_slice(), "spell"))?;
    let cost_idx = word_slice_find_word(clause_words.as_slice(), "cost")
        .or_else(|| word_slice_find_word(clause_words.as_slice(), "costs"))?;
    let less_idx = word_slice_find_word(clause_words.as_slice(), "less")?;

    let has_you_cast = word_slice_contains_phrase(&clause_words, &["you", "cast"]);
    let has_that_player_casts = word_slice_contains_phrase(&clause_words, &["that", "player", "casts"]);
    let has_chosen_name = word_slice_contains_phrase(&clause_words, &["with", "chosen", "name"])
        || word_slice_contains_phrase(&clause_words, &["with", "the", "chosen", "name"]);
    let has_this_turn_duration = word_slice_contains_phrase(&clause_words, &["this", "turn"]);
    let has_until_your_next_turn_duration = clause_words.starts_with(&["until", "your", "next", "turn"]);

    if cost_idx <= spell_idx
        || less_idx <= cost_idx
        || (!has_you_cast && !has_that_player_casts && !has_chosen_name)
        || (!has_this_turn_duration && !has_until_your_next_turn_duration)
        || clause_words.get(less_idx + 1).copied() != Some("to")
        || clause_words.get(less_idx + 2).copied() != Some("cast")
    {
        return None;
    }

    let spell_token_idx = words.token_index_for_word_index(spell_idx)?;
    let cost_token_idx = words.token_index_for_word_index(cost_idx)?;
    let less_token_idx = words.token_index_for_word_index(less_idx)?;
    let subject_start_token_idx = if has_until_your_next_turn_duration {
        words.token_index_for_word_index(4)?
    } else {
        0
    };
    let subject_tokens = trim_edge_punctuation(&tokens[subject_start_token_idx..=spell_token_idx]);
    let reduction_tokens = trim_edge_punctuation(&tokens[cost_token_idx + 1..less_token_idx]);
    let (mut reduction, used) = parse_value(&reduction_tokens)?;
    if used != reduction_tokens.len() {
        return None;
    }
    if matches!(reduction, Value::X)
        && clause_words.get(less_idx + 3).copied() == Some("where")
    {
        let where_token_idx = words.token_index_for_word_index(less_idx + 3)?;
        if let Some(where_value) = parse_value_binding_clause(&tokens[where_token_idx..]) {
            reduction = where_value;
        }
    }

    let mut filter = crate::runtime_backend::parse_spell_filter_lexed(&subject_tokens);
    let player = if has_you_cast {
        filter.cast_by = Some(PlayerFilter::You);
        PlayerAst::You
    } else if has_that_player_casts {
        filter.cast_by = Some(PlayerFilter::IteratedPlayer);
        PlayerAst::That
    } else {
        PlayerAst::Any
    };

    let between_words = &clause_words[spell_idx + 1..cost_idx];
    if has_chosen_name {
        filter.name = Some("{chosen name}".to_string());
    }
    if word_slice_contains_phrase(between_words, &["from", "exile"]) {
        filter.zone = Some(Zone::Exile);
    } else if word_slice_contains_phrase(between_words, &["from", "your", "graveyard"]) {
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
    }

    if LABELED_THE_NEXT_PREFIX_PATTERN.matches_words(&clause_words)
        && let Some((mana_reduction, used)) = parse_cost_modifier_mana_cost(&reduction_tokens)
        && used == reduction_tokens.len()
    {
        Some(EffectAst::subject_verb_reduce_next_spell_cost_this_turn(
            player,
            filter,
            mana_reduction,
        ))
    } else {
        let duration = if has_until_your_next_turn_duration {
            Until::YourNextTurn
        } else {
            Until::EndOfTurn
        };
        if duration == Until::EndOfTurn {
            Some(EffectAst::subject_verb_reduce_matching_spell_cost_this_turn(
                player, filter, reduction,
            ))
        } else {
            Some(EffectAst::subject_verb_reduce_matching_spell_cost(
                player, filter, reduction, duration,
            ))
        }
    }
}

fn parse_exile_replacement_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(effect) = parse_zone_replacement_subject_verb(tokens)? else {
        return Ok(None);
    };
    crate::parse_trace::event(
        "effect-route: subject-verb verb=Exile subject=implicit recognizer=instead-replacement",
    );
    Ok(Some(vec![effect]))
}

fn passive_addition_tail_len(words: &[&str]) -> Option<(usize, bool)> {
    for (tail, adds_colors) in [
        (
            &["in", "addition", "to", "its", "other", "colors", "and", "types"][..],
            true,
        ),
        (
            &["in", "addition", "to", "their", "other", "colors", "and", "types"][..],
            true,
        ),
        (
            &[
                "in", "addition", "to", "its", "other", "colors", "and", "creature", "types",
            ][..],
            true,
        ),
        (
            &[
                "in", "addition", "to", "their", "other", "colors", "and", "creature", "types",
            ][..],
            true,
        ),
        (
            &["in", "addition", "to", "its", "other", "types"][..],
            false,
        ),
        (
            &["in", "addition", "to", "their", "other", "types"][..],
            false,
        ),
        (
            &["in", "addition", "to", "its", "other", "creature", "types"][..],
            false,
        ),
        (
            &[
                "in", "addition", "to", "their", "other", "creature", "types",
            ][..],
            false,
        ),
    ] {
        if slice_ends_with(words, tail) {
            return Some((tail.len(), adds_colors));
        }
    }
    None
}

fn parse_passive_color_type_addition_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let word_view = TokenWordView::new(tokens);
    let words = word_view.to_word_refs();
    let Some((tail_len, adds_colors)) = passive_addition_tail_len(&words) else {
        return Ok(None);
    };
    let Some(is_word_idx) = LABELED_IS_OR_ARE_WORD_PATTERN.find_word(&words) else {
        return Ok(None);
    };
    if is_word_idx + 1 >= words.len().saturating_sub(tail_len) {
        return Ok(None);
    }

    let descriptor_end_word_idx = words.len().saturating_sub(tail_len);
    let clause = LexedClause::new(tokens);
    let subject_tokens = clause
        .before_word(is_word_idx)
        .unwrap_or_else(|| clause.before(0))
        .tokens();
    let descriptor_tokens = clause
        .between_word_range(is_word_idx + 1, descriptor_end_word_idx)
        .unwrap_or_else(|| clause.between(tokens.len(), tokens.len()))
        .tokens();
    let subject_words = TokenWordView::new(subject_tokens).to_word_refs();

    let target = if matches!(
        subject_words.as_slice(),
        ["it"]
            | ["that", "card"]
            | ["that", "creature"]
            | ["that", "permanent"]
            | ["those", "cards"]
            | ["each", "of", "them"]
    ) {
        TargetAst::Tagged(TagKey::from(IT_TAG), Some(TextSpan::synthetic()))
    } else {
        parse_target_phrase(subject_tokens)?
    };

    let mut colors = crate::color::ColorSet::new();
    let mut card_types = Vec::new();
    let mut subtypes = Vec::<Subtype>::new();
    for word in TokenWordView::new(descriptor_tokens).to_word_refs() {
        if LABELED_ARTICLE_AND_WORD_PATTERN.matches_word(word) {
            continue;
        }
        if let Some(color) = parse_color(word) {
            colors = colors.union(color);
            continue;
        }
        if let Some(card_type) = parse_card_type(word) {
            crate::slice_primitives::push_unique(&mut card_types, card_type);
            continue;
        }
        if let Some(subtype) = super::parse_subtype_word(word) {
            crate::slice_primitives::push_unique(&mut subtypes, subtype);
            continue;
        }
        return Ok(None);
    }

    let mut effects = Vec::new();
    if !colors.is_empty() {
        let color_effect = if adds_colors {
            EffectAst::subject_verb_add_colors(target.clone(), colors, Until::Forever)
        } else {
            EffectAst::subject_verb_set_colors(target.clone(), colors, Until::Forever)
        };
        effects.push(color_effect);
    }
    if !card_types.is_empty() {
        effects.push(EffectAst::subject_verb_add_card_types(
            target.clone(),
            card_types,
            Until::Forever,
        ));
    }
    if !subtypes.is_empty() {
        effects.push(EffectAst::subject_verb_add_subtypes(
            target,
            subtypes,
            Until::Forever,
        ));
    }

    Ok((!effects.is_empty()).then_some(effects))
}

pub(crate) fn parse_subject_verb_extension_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    macro_rules! one {
        ($route:literal, $parser:expr) => {{
            if let Some(effect) = $parser? {
                crate::parse_trace::event(concat!("effect-route: subject-verb ", $route));
                return Ok(Some(vec![effect]));
            }
        }};
    }
    macro_rules! many {
        ($route:literal, $parser:expr) => {{
            if let Some(effects) = $parser? {
                crate::parse_trace::event(concat!("effect-route: subject-verb ", $route));
                return Ok(Some(effects));
            }
        }};
    }

    one!(
        "verb=Take subject=implicit recognizer=extra-turn-after-anchor",
        parse_take_extra_turn_sentence(tokens)
    );
    one!(
        "verb=Prevent subject=implicit recognizer=damage-prevention",
        parse_prevent_damage_sentence(tokens)
    );
    one!(
        "verb=Monstrosity subject=implicit recognizer=keyword-action",
        parse_monstrosity_sentence(tokens)
    );
    many!(
        "verb=Earthbend subject=implicit recognizer=keyword-action",
        parse_earthbend_subject_verb_sentence(tokens)
    );
    one!(
        "verb=Enchant subject=implicit recognizer=aura-attachment",
        super::search_library::parse_enchant_sentence(tokens)
    );
    one!(
        "verb=Play subject=explicit recognizer=zone-permission",
        parse_play_permission_subject_verb(tokens)
    );
    one!(
        "verb=Exile subject=implicit recognizer=instead-replacement",
        parse_zone_replacement_subject_verb(tokens)
    );
    many!(
        "verb=Is subject=explicit recognizer=passive-color-type-addition",
        parse_passive_color_type_addition_sentence(tokens)
    );
    many!(
        "verb=When subject=implicit recognizer=delayed-trigger-this-turn",
        parse_sentence_delayed_trigger_this_turn(tokens)
    );
    one!(
        "verb=Choose subject=explicit recognizer=choice-complement-sacrifice",
        parse_choice_complement_subject_verb(tokens)
    );
    many!(
        "verb=Gain subject=explicit recognizer=life-equal-stat",
        parse_gain_life_equal_to_power_sentence(tokens)
    );
    one!(
        "verb=Get subject=explicit recognizer=last-effect-counter-loop",
        parse_for_each_counter_removed_sentence(tokens)
    );
    many!(
        "verb=Exile subject=explicit recognizer=exile-return-same-object",
        parse_exile_then_return_same_object_sentence(tokens)
    );
    if is_simple_source_gain_ability_candidate(tokens) {
        many!(
            "verb=Gain subject=implicit recognizer=source-ability-grant",
            parse_gain_ability_to_source_subject_verb_sentence(tokens)
        );
    }
    if is_simple_gain_ability_candidate(tokens) {
        many!(
            "verb=Gain subject=explicit recognizer=ability-grant",
            parse_gain_ability_subject_verb_sentence(tokens)
        );
    }
    many!(
        "verb=Choose subject=explicit recognizer=opponent-decline-loop",
        parse_for_each_opponent_doesnt_subject_verb_sentence(tokens)
    );
    many!(
        "verb=Vote subject=explicit recognizer=vote-affinity",
        parse_vote_affinity_subject_verb(tokens)
    );
    one!(
        "verb=Vote subject=explicit recognizer=vote-procedure",
        parse_vote_subject_verb(tokens)
    );

    Ok(None)
}

fn parse_earthbend_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(earthbend) = super::search_library::parse_earthbend_sentence(tokens)? else {
        return Ok(None);
    };

    let Some((_, used)) = parse_number(&tokens[1..]) else {
        return Ok(Some(vec![earthbend]));
    };
    let mut tail = trim_commas(&tokens[1 + used..]).to_vec();
    while token_slice_first_is(&tail, "then") {
        tail.remove(0);
    }
    if tail.is_empty() {
        return Ok(Some(vec![earthbend]));
    }

    let mut effects = vec![earthbend];
    if token_slice_first_is(&tail, "earthbend") {
        if let Some(mut tail_effects) = parse_earthbend_subject_verb_sentence(&tail)? {
            effects.append(&mut tail_effects);
            return Ok(Some(effects));
        }
    }
    effects.extend(parse_effect_chain_lexed(&tail)?);
    Ok(Some(effects))
}

fn parse_gain_ability_to_source_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(super::gain_ability::parse_gain_ability_to_source_sentence(tokens)?
        .map(|effect| vec![effect]))
}

fn parse_gain_ability_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    super::gain_ability::parse_gain_ability_sentence(tokens)
}

fn is_simple_source_gain_ability_candidate(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(gain_idx) = LABELED_GAIN_OR_GAINS_WORD_PATTERN.find_word(&words)
    else {
        return false;
    };
    gain_idx > 0
        && is_source_reference_words(&words[..gain_idx])
        && !words[gain_idx + 1..]
            .iter()
            .any(|word| LABELED_SIMPLE_ABILITY_TAIL_STOP_WORD_PATTERN.matches_word(word))
}

fn is_simple_gain_ability_candidate(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(gain_idx) = LABELED_GAIN_HAS_LOSE_WORD_PATTERN.find_word(&words)
    else {
        return false;
    };

    let ability_words = &words[gain_idx + 1..];
    if LABELED_HAS_HAVE_WORD_PATTERN.matches_word(words[gain_idx])
        && !ability_words
            .iter()
            .any(|word| LABELED_TRIGGER_WORD_PATTERN.matches_word(word))
    {
        return false;
    }
    if LABELED_SIMPLE_ABILITY_EXCLUSION_PATTERN.matches_words(&words[..gain_idx])
        || LABELED_ANOTHER_HASTE_PATTERN.matches_words(&words)
    {
        return false;
    }
    !ability_words.is_empty()
        && !LABELED_LIFE_WORD_PATTERN.matches_words(ability_words)
        && (LABELED_SIMPLE_ABILITY_WORD_PATTERN.matches_words(ability_words)
            || contains_token_kind(tokens, TokenKind::Quote)
            || contains_token_kind(tokens, TokenKind::Colon))
}

fn parse_for_each_opponent_doesnt_subject_verb_sentence(
    tokens: &[OwnedLexToken],
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    Ok(super::conditionals::parse_for_each_opponent_doesnt(tokens)?.map(|effect| vec![effect]))
}

pub(crate) fn is_negated_untap_clause(words: &[&str]) -> bool {
    effect_grammar::is_negated_untap_clause_words(words)
}

pub(crate) fn parse_token_copy_modifier_sentence(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    parse_token_copy_modifier_words(filtered.as_slice())
}

pub(crate) fn parse_token_copy_modifier_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered = crate::runtime_backend::util::non_article_token_word_refs(tokens);
    parse_token_copy_modifier_words(filtered.as_slice())
}

fn parse_token_copy_modifier_words(filtered: &[&str]) -> Option<TokenCopyFollowup> {
    let is_gain_haste_until_eot = matches!(
        filtered,
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered,
        ["it", "has", "haste"]
            | ["they", "have", "haste"]
            | ["token", "created", "this", "way", "has", "haste"]
            | ["tokens", "created", "this", "way", "have", "haste"]
            | ["token", "created", "this", "way", "gains", "haste"]
            | ["tokens", "created", "this", "way", "gain", "haste"]
    );
    if is_has_haste {
        return Some(TokenCopyFollowup::HasHaste);
    }

    let enters_tapped_and_attacking = matches!(
        filtered,
        ["it", "enters", "tapped", "and", "attacking"]
            | ["they", "enter", "tapped", "and", "attacking"]
            | ["token", "enters", "tapped", "and", "attacking"]
            | ["tokens", "enter", "tapped", "and", "attacking"]
            | [
                "token",
                "created",
                "this",
                "way",
                "enters",
                "tapped",
                "and",
                "attacking"
            ]
            | [
                "tokens",
                "created",
                "this",
                "way",
                "enter",
                "tapped",
                "and",
                "attacking"
            ]
    );
    if enters_tapped_and_attacking {
        return Some(TokenCopyFollowup::EnterTappedAndAttacking);
    }

    if LABELED_TOKEN_SACRIFICE_PREFIX_PATTERN.matches_words(filtered)
        && LABELED_NEXT_END_STEP_PATTERN.matches_words(filtered)
    {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    if LABELED_TOKEN_EXILE_PREFIX_PATTERN.matches_words(filtered)
        && LABELED_NEXT_END_STEP_PATTERN.matches_words(filtered)
    {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    if LABELED_DELAYED_END_STEP_SACRIFICE_PREFIX_PATTERN.matches_words(filtered) {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    if LABELED_DELAYED_END_STEP_EXILE_PREFIX_PATTERN.matches_words(filtered) {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}

#[cfg(test)]
mod labeled_prefix_tests {
    use super::*;

    #[test]
    fn labeled_mana_value_bound_uses_lexed_tail_tokens() {
        let tokens = crate::runtime_backend::lex_line(
            "You may cast any number of spells with mana value 5 or less from among them without paying their mana costs.",
            0,
        )
        .expect("labeled mana-value text should lex");

        assert_eq!(labeled_mana_value_or_less_bound(&tokens), Some(5));
    }
}
