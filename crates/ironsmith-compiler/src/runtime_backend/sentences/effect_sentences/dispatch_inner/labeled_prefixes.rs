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
    if word_slice_starts_with(sentence_words.as_slice(), &["round", "up", "each", "time"]) {
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
    if word_slice_first_is_any(&sentence_words, &["prevent", "take", "monstrosity"])
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
    if word_slice_starts_with(sentence_words.as_slice(), &["you", "may", "cast"])
        && word_slice_contains_phrase(&sentence_words, &["from", "among", "them"])
        && word_slice_contains_phrase(
            &sentence_words,
            &["without", "paying", "its", "mana", "cost"],
        )
    {
        let mut filter = ObjectFilter::tagged(TagKey::from(IT_TAG));
        filter.card_types.push(CardType::Instant);
        filter.card_types.push(CardType::Sorcery);
        filter.card_types.push(CardType::Artifact);
        filter.card_types.push(CardType::Creature);
        filter.card_types.push(CardType::Enchantment);
        filter.card_types.push(CardType::Planeswalker);
        filter.card_types.push(CardType::Battle);
        filter.type_or_subtype_union = true;
        if word_slice_contains_phrase(&sentence_words, &["mana", "value", "3", "or", "less"]) {
            filter.mana_value = Some(crate::filter::Comparison::LessThanOrEqual(3));
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
    if word_slice_starts_with(
        sentence_words.as_slice(),
        &["you", "may", "cast", "a", "spell", "from", "your", "hand"],
    ) && word_slice_contains_phrase(
        &sentence_words,
        &["without", "paying", "its", "mana", "cost"],
    )
    {
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
    if word_slice_starts_with(
        sentence_words.as_slice(),
        &["exile", "all", "cards", "from"],
    ) && word_slice_contains_any_word(sentence_words.as_slice(), &["hand", "hands"])
        && word_slice_contains_any_word(sentence_words.as_slice(), &["graveyard", "graveyards"])
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_first_is(&sentence_words, "enchant")
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_first_is(&sentence_words, "earthbend")
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_contains_word(sentence_words.as_slice(), "unless")
        && let Some(mut effects) =
            super::parse_sentence_unless_pays(super::SubjectVerbPrimitiveClause::new(tokens))?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_contains_word(sentence_words.as_slice(), "unless")
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if sentence_words
        .iter()
        .any(|word| matches!(*word, "gain" | "gains" | "lose" | "loses"))
    {
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
    if sentence_words
        .iter()
        .any(|word| *word == "vote" || *word == "votes")
        && let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_first_is(&sentence_words, "return")
        && word_slice_contains_word(sentence_words.as_slice(), "rounded")
        && word_slice_contains_word(sentence_words.as_slice(), "up")
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_first_is(&sentence_words, "choose")
        && crate::runtime_backend::lexer::word_slice_contains_phrase(
            sentence_words.as_slice(),
            &["do", "the", "same", "for"],
        )
        && let Some(mut effects) = run_subject_verb_primitives_lexed(
            tokens,
            POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVES,
            &POST_CONDITIONAL_SUBJECT_VERB_PRIMITIVE_INDEX,
        )?
    {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if word_slice_starts_with_any(
        sentence_words.as_slice(),
        &[
            &["each", "player", "choose"],
            &["each", "player", "chooses"],
        ],
    ) {
        if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
            apply_where_x_to_damage_amounts(tokens, &mut effects)?;
            return Ok(effects);
        }
    }
    if word_slice_starts_with(
        sentence_words.as_slice(),
        &["cast", "any", "number", "of"],
    ) && crate::runtime_backend::lexer::word_slice_contains_phrase(
        sentence_words.as_slice(),
        &["from", "your", "graveyard"],
    ) && crate::runtime_backend::lexer::word_slice_contains_phrase(
        sentence_words.as_slice(),
        &["without", "paying", "their", "mana", "costs"],
    )
    {
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
    if word_slice_first_is(&sentence_words, "sacrifice") && !sacrifice_counted_prefix {
        let mut effects = parse_effect_chain_lexed(tokens)?;
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }
    if let Some(mut effects) = parse_subject_verb_extension_sentence(tokens)? {
        apply_where_x_to_damage_amounts(tokens, &mut effects)?;
        return Ok(effects);
    }

    let (_, effects) = super::sentence_registry::run_sentence_parse_rules_lexed(tokens)?;
    Ok(effects)
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
    let Some(is_word_idx) = crate::runtime_backend::lexer::word_slice_find_word_where(
        &words,
        |word| matches!(word, "is" | "are"),
    ) else {
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
        if matches!(word, "a" | "an" | "and") {
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
    let Some(gain_idx) = find_index(words.as_slice(), |word| matches!(*word, "gain" | "gains"))
    else {
        return false;
    };
    gain_idx > 0
        && is_source_reference_words(&words[..gain_idx])
        && !words[gain_idx + 1..]
            .iter()
            .any(|word| matches!(*word, "and" | "then" | "if"))
}

fn is_simple_gain_ability_candidate(tokens: &[OwnedLexToken]) -> bool {
    let words = crate::runtime_backend::token_word_refs(tokens);
    let Some(gain_idx) = find_index(words.as_slice(), |word| {
        matches!(*word, "gain" | "gains" | "has" | "have" | "lose" | "loses")
    }) else {
        return false;
    };

    let ability_words = &words[gain_idx + 1..];
    if matches!(words[gain_idx], "has" | "have")
        && !ability_words
            .iter()
            .any(|word| matches!(*word, "when" | "whenever"))
    {
        return false;
    }
    if words[..gain_idx]
        .iter()
        .any(|word| matches!(*word, "shares" | "choice"))
        || (crate::runtime_backend::lexer::word_slice_contains_word(&words[..gain_idx], "another")
            && crate::runtime_backend::lexer::word_slice_contains_word(ability_words, "haste"))
    {
        return false;
    }
    !ability_words.is_empty()
        && !crate::runtime_backend::lexer::word_slice_contains_word(ability_words, "life")
        && (ability_words.iter().any(|word| {
            matches!(
                *word,
                "indestructible"
                    | "haste"
                    | "flying"
                    | "vigilance"
                    | "lifelink"
                    | "trample"
                    | "reach"
                    | "menace"
                    | "fear"
                    | "deathtouch"
                    | "horsemanship"
                    | "hexproof"
                    | "shroud"
                    | "shadow"
                    | "strike"
                    | "protection"
                    | "blocked"
                    | "abilities"
                    | "when"
                    | "whenever"
            )
        }) || contains_token_kind(tokens, TokenKind::Quote)
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

    let is_gain_haste_until_eot = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered.as_slice(),
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
        filtered.as_slice(),
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

    if word_slice_starts_with_any(
        filtered.as_slice(),
        &[
            &["sacrifice", "it"],
            &["sacrifice", "them"],
            &["sacrifice", "that", "token"],
            &["sacrifice", "those", "tokens"],
        ],
    ) {
        let has_next_end_step = crate::runtime_backend::lexer::word_slice_contains_phrase(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
        }
    }
    if word_slice_starts_with_any(filtered.as_slice(), &[&["exile", "it"], &["exile", "them"]]) {
        let has_next_end_step = crate::runtime_backend::lexer::word_slice_contains_phrase(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::ExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice = word_slice_starts_with_any(
        filtered.as_slice(),
        &[
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
        ],
    );
    if starts_delayed_end_step_sacrifice {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile = word_slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "exile",
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
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "exile",
            ],
        ],
    );
    if starts_delayed_end_step_exile {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}

pub(crate) fn parse_token_copy_modifier_sentence_lexed(
    tokens: &[OwnedLexToken],
) -> Option<TokenCopyFollowup> {
    let filtered = crate::runtime_backend::util::non_article_token_word_refs(tokens);

    let is_gain_haste_until_eot = matches!(
        filtered.as_slice(),
        ["it", "gains", "haste", "until", "end", "of", "turn"]
            | ["they", "gain", "haste", "until", "end", "of", "turn"]
    );
    if is_gain_haste_until_eot {
        return Some(TokenCopyFollowup::GainHasteUntilEndOfTurn);
    }

    let is_has_haste = matches!(
        filtered.as_slice(),
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
        filtered.as_slice(),
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

    if word_slice_starts_with_any(
        filtered.as_slice(),
        &[
            &["sacrifice", "it"],
            &["sacrifice", "them"],
            &["sacrifice", "that", "token"],
            &["sacrifice", "those", "tokens"],
        ],
    ) {
        let has_next_end_step = crate::runtime_backend::lexer::word_slice_contains_phrase(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
        }
    }
    if word_slice_starts_with_any(filtered.as_slice(), &[&["exile", "it"], &["exile", "them"]]) {
        let has_next_end_step = crate::runtime_backend::lexer::word_slice_contains_phrase(
            filtered.as_slice(),
            &["at", "beginning", "of", "next", "end", "step"],
        );
        if has_next_end_step {
            return Some(TokenCopyFollowup::ExileAtNextEndStep);
        }
    }

    let starts_delayed_end_step_sacrifice = word_slice_starts_with_any(
        filtered.as_slice(),
        &[
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
        ],
    );
    if starts_delayed_end_step_sacrifice {
        return Some(TokenCopyFollowup::SacrificeAtNextEndStep);
    }
    let starts_delayed_end_step_exile = word_slice_starts_with_any(
        filtered.as_slice(),
        &[
            &[
                "at",
                "the",
                "beginning",
                "of",
                "the",
                "end",
                "step",
                "exile",
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
                "exile",
            ],
            &[
                "at",
                "the",
                "beginning",
                "of",
                "next",
                "end",
                "step",
                "exile",
            ],
        ],
    );
    if starts_delayed_end_step_exile {
        return Some(TokenCopyFollowup::ExileAtNextEndStep);
    }

    None
}
