use super::*;

pub(in super::super) fn parse_object_filter_inner(
    tokens: &[OwnedLexToken],
    other: bool,
    strict: bool,
) -> Result<ObjectFilter, CardTextError> {
    let (tokens, vote_winners_only) = trim_vote_winner_suffix(tokens);
    let trailing_couldnt_attack_exception = tokens.len() >= 6
        && tokens[tokens.len() - 6].is_word("except")
        && tokens[tokens.len() - 5].is_word("for")
        && (tokens[tokens.len() - 4].is_word("creature")
            || tokens[tokens.len() - 4].is_word("creatures"))
        && tokens[tokens.len() - 3].is_word("that")
        && (tokens[tokens.len() - 2].is_word("couldn't")
            || tokens[tokens.len() - 2].is_word("couldnt"))
        && tokens[tokens.len() - 1].is_word("attack");
    let tokens = if trailing_couldnt_attack_exception {
        &tokens[..tokens.len() - 6]
    } else {
        tokens.as_slice()
    };
    // A terminal arity phrase can qualify the stack object without naming a
    // target class: "an instant or sorcery spell with a single target". The
    // relation parser below only sees `that target(s) ...`, so retain this
    // independent grammar fact before parsing the ordinary spell domain.
    let trailing_single_target = tokens.len() >= 4
        && tokens[tokens.len() - 4].is_word("with")
        && tokens[tokens.len() - 3].is_word("a")
        && tokens[tokens.len() - 2].is_word("single")
        && (tokens[tokens.len() - 1].is_word("target")
            || tokens[tokens.len() - 1].is_word("targets"));
    let tokens = if trailing_single_target {
        &tokens[..tokens.len() - 4]
    } else {
        tokens
    };
    let chosen_type_reference = parse_chosen_type_reference_tokens(tokens);
    let mut filter = ObjectFilter::default();
    filter.could_have_attacked_this_turn = trailing_couldnt_attack_exception;
    if other {
        filter.other = true;
    }

    let mut target_player: Option<PlayerFilter> = None;
    let mut target_object: Option<ObjectFilter> = None;
    let mut targets_only = false;
    let mut target_count = trailing_single_target.then_some(crate::effect::ChoiceCount::exactly(1));
    let mut base_tokens: Vec<OwnedLexToken> = tokens.to_vec();
    let mut targets_idx: Option<usize> = None;
    for (idx, token) in tokens.iter().enumerate() {
        if token
            .as_word()
            .is_some_and(|word| parse_word_choice(word, TARGET_OR_TARGETS_WORDS).is_some())
            && idx > 0
            && tokens[idx - 1]
                .as_word()
                .is_some_and(|word| word == THAT_WORD)
        {
            targets_idx = Some(idx);
            break;
        }
    }
    if let Some(targets_idx) = targets_idx {
        let that_idx = targets_idx - 1;
        base_tokens = tokens[..that_idx].to_vec();
        let mut target_tokens = &tokens[targets_idx + 1..];
        let mut relation_target_count = None;
        // A bare article is grammar, not arity: "that targets a permanent you
        // control" constrains what is targeted, never how many targets the
        // spell has. Only an explicit count ("that targets two ...") narrows
        // the relation's target count.
        let leading_article = target_tokens.first().is_some_and(|token| {
            token
                .as_word()
                .is_some_and(|word| word == "a" || word == "an")
        });
        if !leading_article
            && let Some((count, rest)) = primitives::parse_prefix(
                target_tokens,
                crate::grammar::leaf::parse_leaf_choice_count_prefix_lexed,
            )
        {
            relation_target_count = Some(count);
            target_tokens = rest;
        }
        let parse_target_fragment = |fragment_tokens: &[OwnedLexToken]| -> Result<
            (
                Option<PlayerFilter>,
                Option<ObjectFilter>,
                bool,
                Option<crate::effect::ChoiceCount>,
            ),
            CardTextError,
        > {
            let mut fragment_tokens = trim_commas(fragment_tokens);
            let mut only = false;
            let mut count = None;
            // The outer scan splits target fragments after the demonstrative
            // "that target(s)" marker, so a fragment never re-introduces a
            // leading "that"; strip one defensively to keep the fragment shape
            // stable if upstream splitting changes.
            if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == THAT_WORD))
            {
                fragment_tokens.drain(..1);
            }
            if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == ONLY_WORD))
            {
                only = true;
                fragment_tokens.drain(..1);
            }
            if fragment_tokens.len() >= 2
                && fragment_tokens[0].is_word("a")
                && fragment_tokens[1]
                    .as_word()
                    .is_some_and(|word| word == SINGLE_WORD)
            {
                count = Some(crate::effect::ChoiceCount::exactly(1));
                fragment_tokens.drain(..2);
            } else if fragment_tokens
                .first()
                .is_some_and(|token| token.as_word().is_some_and(|word| word == SINGLE_WORD))
            {
                count = Some(crate::effect::ChoiceCount::exactly(1));
                fragment_tokens.drain(..1);
            }

            if parse_phrase_at_head(
                &non_article_parser_word_refs(&fragment_tokens),
                YOU_TARGET_PREFIX,
            )
            .is_some()
            {
                return Ok((Some(PlayerFilter::You), None, only, count));
            }
            if parse_phrase_choice_at_head(
                &non_article_parser_word_refs(&fragment_tokens),
                OPPONENT_TARGET_PREFIXES,
            )
            .is_some()
            {
                return Ok((Some(PlayerFilter::Opponent), None, only, count));
            }
            if parse_phrase_choice_at_head(
                &non_article_parser_word_refs(&fragment_tokens),
                PLAYER_TARGET_PREFIXES,
            )
            .is_some()
            {
                return Ok((Some(PlayerFilter::Any), None, only, count));
            }

            let mut target_filter_tokens = fragment_tokens.as_slice();
            if target_filter_tokens.first().is_some_and(|token| {
                token
                    .as_word()
                    .is_some_and(|word| parse_word_choice(word, TARGET_OR_TARGETS_WORDS).is_some())
            }) {
                target_filter_tokens = &target_filter_tokens[1..];
            }
            if target_filter_tokens.is_empty() {
                return Ok((None, None, only, count));
            }
            let source_exclusion_surface =
                target_filter_tokens
                    .iter()
                    .enumerate()
                    .find_map(|(index, token)| {
                        (token.is_word("other")
                            && target_filter_tokens
                                .get(index + 1)
                                .is_some_and(|next| next.is_word("than")))
                        .then(|| source_reference_tail_prefix(&target_filter_tokens[index + 2..]))
                        .flatten()
                        .and_then(|(consumed, surface)| {
                            (consumed == target_filter_tokens.len() - index - 2).then_some(surface)
                        })
                    });
            let mut target_filter = parse_object_filter_permissive(target_filter_tokens, false)?;
            // The relation parser peels `that targets only a single ...`
            // away from the stack-spell filter before the ordinary source
            // exclusion stage is finalized. Recover the exact proper-name or
            // typed-source tail on the nested target filter: this remains the
            // executable source-identity predicate (`other`), while the
            // authored alias is presentation provenance only.
            if let Some(surface) = source_exclusion_surface {
                target_filter.other = true;
                target_filter.source_surface = Some(surface);
            }
            Ok((None, Some(target_filter), only, count))
        };

        if let Some(or_token_idx) = token_index_for_word(target_tokens, OR_WORD) {
            let left_tokens = trim_commas(&target_tokens[..or_token_idx]);
            let right_tokens = trim_commas(&target_tokens[or_token_idx + 1..]);
            let (left_player, left_object, left_only, left_count) =
                parse_target_fragment(&left_tokens)?;
            let (right_player, right_object, right_only, right_count) =
                parse_target_fragment(&right_tokens)?;
            let is_object_union = left_player.is_none()
                && right_player.is_none()
                && left_object.is_some()
                && right_object.is_some();
            target_player = left_player.or(right_player);
            target_object = if is_object_union {
                // Preserve an object-class union such as "creatures or
                // Vehicles you control" as one target relation. Picking one
                // side here silently broadened/narrowed both trigger matching
                // and event-derived target counts.
                Some(parse_object_filter_permissive(target_tokens, false)?)
            } else {
                left_object.or(right_object)
            };
            targets_only = left_only || right_only;
            target_count = relation_target_count.or(left_count).or(right_count);
            if target_player.is_some() && target_object.is_some() {
                filter.targets_any_of = true;
            }
        } else {
            let (parsed_player, parsed_object, parsed_only, parsed_count) =
                parse_target_fragment(target_tokens)?;
            target_player = parsed_player;
            target_object = parsed_object;
            targets_only = parsed_only;
            target_count = relation_target_count.or(parsed_count);
        }
    }

    // Object filters should not absorb trailing duration clauses such as
    // "... until this enchantment leaves the battlefield".
    if let Some(until_token_idx) = token_index_for_word(&base_tokens, UNTIL_WORD)
        && until_token_idx > 0
    {
        base_tokens.truncate(until_token_idx);
    }

    let not_on_battlefield = strip_not_on_battlefield_phrase(&mut base_tokens);

    // "<subject> with a <inner> attached to it" and "<subject> that's
    // enchanted by <inner>" both select the subject based on an attachment
    // it carries. Intercept before the attached-to-tail split claims the
    // attachment words as the subject's own attachment reference.
    let attachment_split = split_with_attached_object_filter(&base_tokens)
        .map(|(subject, inner)| (subject, inner, false))
        .or_else(|| {
            split_enchanted_by_object_filter(&base_tokens)
                .map(|(subject, inner)| (subject, inner, true))
        });
    if let Some((subject_tokens, inner_tokens, uses_enchanted_by_surface)) = attachment_split {
        let inner_other = inner_tokens
            .first()
            .is_some_and(|token| token.is_word("another") || token.is_word("other"));
        let mut inner = parse_object_filter_permissive(&inner_tokens, inner_other)?;
        inner.other |= inner_other;
        filter.with_attached_object = Some(Box::new(inner));
        if uses_enchanted_by_surface {
            filter.set_relative_attachment_state_surface(true);
        }
        base_tokens = subject_tokens;
    }

    if let Some((head_tokens, attached_to_tokens)) = split_attached_to_object_filter(&base_tokens) {
        let attached_to_words = non_article_parser_word_refs(&attached_to_tokens);
        if parse_phrase_whole(&attached_to_words, THAT_PLAYER_ATTACHMENT_TAIL).is_some() {
            filter.attached_to_player =
                Some(PlayerFilter::AliasedTarget(Box::new(PlayerFilter::Any)));
        } else {
            let attached_to = if crate::word_primitives::parse_any_sequence_complete(
                &attached_to_words,
                &[&["him"], &["her"]],
            ) {
                ObjectFilter::source_with_surface(crate::target::SourceReferenceSurface::FullName(
                    attached_to_words[0].to_string(),
                ))
            } else {
                let mut attached = parse_object_filter_permissive(&attached_to_tokens, false)?;
                attached.set_plural_object_noun_surface(has_plural_object_noun_surface(
                    &attached_to_tokens,
                ));
                // `this creature` is both a source identity reference and a
                // typed object selector. Preserve the noun on the nested
                // filter so attachment legality does not widen to every
                // source object.
                if attached.source
                    && attached.card_types.is_empty()
                    && attached_to_words.len() == 2
                    && let Some(card_type) = parse_card_type(attached_to_words[1])
                {
                    attached.card_types.push(card_type);
                }
                attached
            };
            filter.attached_to_object = Some(Box::new(attached_to));
        }
        base_tokens = head_tokens;
    }

    // A chosen-object exclusion is an identity relation to the preceding
    // choice. Do not let the generic "other than <type>" pass reinterpret
    // the final noun as an excluded card type (for example, as
    // "noncreature creature").
    let base_words = parser_token_word_refs(&base_tokens);
    if let Some(exclusion) =
        parse_phrase_choice_anywhere(&base_words, CHOSEN_OBJECT_EXCLUSION_PHRASES).filter(
            |exclusion| {
                exclusion.phrase.first() != Some(&"and")
                    || parse_phrase_anywhere(&base_words[..exclusion.span.start], OTHER_THAN_PREFIX)
                        .is_some()
            },
        )
    {
        let start = token_index_after_word_prefix(&base_tokens, exclusion.span.start)
            .unwrap_or(base_tokens.len());
        let end = token_index_after_word_prefix(&base_tokens, exclusion.span.end)
            .unwrap_or(base_tokens.len());
        if start < end {
            let chosen_kind = exclusion.phrase.last().copied().unwrap_or("object");
            filter.tagged_constraints.push(TaggedObjectConstraint {
                tag: crate::tag::CompilerReferenceTag::ChosenObjects.bind(),
                relation: TaggedOpbjectRelation::IsNotTaggedObject,
            });
            // For a direct `other than the chosen ...` exclusion this inert
            // surface preserves the chosen noun. In the coordinated
            // `other than <source> and the chosen ...` form, leave the slot
            // available for the independently parsed source identity.
            if exclusion.phrase.first() == Some(&"other") {
                filter.source_surface = Some(crate::target::SourceReferenceSurface::FullName(
                    format!("the chosen {chosen_kind}"),
                ));
            }
            base_tokens.drain(start..end);
        }
    }

    // "other than <source>" marks an exclusion, not an additional type
    // selector. Keep "other" and capture the source surface when available.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if parse_phrase_whole(
            &non_article_parser_word_refs(&base_tokens[idx..idx + 2]),
            OTHER_THAN_PREFIX,
        )
        .is_none()
        {
            idx += 1;
            continue;
        }

        let tail_tokens = &base_tokens[idx + 2..];
        let Some((tail_token_len, surface)) = source_reference_tail_prefix(tail_tokens) else {
            idx += 1;
            continue;
        };

        filter.other = true;
        filter.source_surface.get_or_insert(surface);
        base_tokens.drain(idx..idx + 2 + tail_token_len);
    }

    // "other than Werewolves and Wolves" is an exclusion on the described
    // object class, not the source-relative "other" predicate.
    let mut idx = 0usize;
    while idx + 2 < base_tokens.len() {
        if parse_phrase_whole(
            &non_article_parser_word_refs(&base_tokens[idx..idx + 2]),
            OTHER_THAN_PREFIX,
        )
        .is_none()
        {
            idx += 1;
            continue;
        }

        let mut base_card_types = Vec::new();
        for token in &base_tokens[..idx] {
            for piece in token.parser_word_pieces() {
                if let Some(card_type) = parse_card_type(piece.text.as_str()) {
                    push_unique(&mut base_card_types, card_type);
                }
            }
        }

        let tail_tokens = &base_tokens[idx + 2..];
        if parse_phrase_choice_at_head(
            &non_article_parser_word_refs(tail_tokens),
            EXCLUSION_RELATION_IGNORED_PREFIXES,
        )
        .is_some()
        {
            idx += 1;
            continue;
        }
        let mut excluded_card_types = Vec::new();
        let mut excluded_subtypes = Vec::new();
        let mut excluded_supertypes = Vec::new();
        let mut excluded_colors = ColorSet::new();
        for token in tail_tokens {
            for piece in token.parser_word_pieces() {
                let word = piece.text.as_str();
                if is_article(word) || parse_word_choice(word, AND_OR_WORDS).is_some() {
                    continue;
                }
                if let Some(card_type) = parse_card_type(word) {
                    push_unique(&mut excluded_card_types, card_type);
                }
                if let Some(subtype) = parse_subtype_flexible(word) {
                    push_unique(&mut excluded_subtypes, subtype);
                }
                if let Some(supertype) = parse_supertype_word(word) {
                    push_unique(&mut excluded_supertypes, supertype);
                }
                if let Some(color) = parse_color(word) {
                    excluded_colors = excluded_colors.union(color);
                }
            }
        }

        let has_specific_exclusion = !excluded_subtypes.is_empty()
            || !excluded_supertypes.is_empty()
            || !excluded_colors.is_empty();
        let saw_exclusion = !excluded_card_types.is_empty() || has_specific_exclusion;
        if !saw_exclusion {
            idx += 1;
            continue;
        }

        for card_type in excluded_card_types {
            if has_specific_exclusion && slice_has(&base_card_types, &card_type) {
                continue;
            }
            push_unique(&mut filter.excluded_card_types, card_type);
        }
        for subtype in excluded_subtypes {
            push_unique(&mut filter.excluded_subtypes, subtype);
        }
        for supertype in excluded_supertypes {
            push_unique(&mut filter.excluded_supertypes, supertype);
        }
        filter.excluded_colors = filter.excluded_colors.union(excluded_colors);
        base_tokens.truncate(idx);
        break;
    }

    if let Some(mut disjunction) = parse_attached_reference_or_another_disjunction(&base_tokens)? {
        disjunction.attached_to_object = filter.attached_to_object.take();
        disjunction.attached_to_player = filter.attached_to_player.take();
        if target_player.is_some() || target_object.is_some() {
            disjunction = if targets_only {
                disjunction.targeting_only(target_player.take(), target_object.take())
            } else {
                disjunction.targeting(target_player.take(), target_object.take())
            };
            if let Some(count) = target_count {
                disjunction = disjunction.with_target_count(count);
            } else if targets_only {
                disjunction = disjunction.target_count_exact(1);
            }
        }
        return Ok(disjunction);
    }
    let mut segment_tokens = base_tokens.clone();

    let raw_words_with_articles = parser_token_word_refs(&base_tokens);
    let all_words_with_articles = word_refs_except(&raw_words_with_articles, &["instead"]);

    let map_non_article_index = |non_article_idx: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_idx {
                return Some(idx);
            }
            seen += 1;
        }
        None
    };

    let map_non_article_end = |non_article_end: usize| -> Option<usize> {
        let mut seen = 0usize;
        for (idx, word) in all_words_with_articles.iter().enumerate() {
            if is_article(word) {
                continue;
            }
            if seen == non_article_end {
                return Some(idx);
            }
            seen += 1;
        }
        if seen == non_article_end {
            return Some(all_words_with_articles.len());
        }
        None
    };

    let mut all_words = non_article_word_refs(&all_words_with_articles);
    let has_tap_activated_ability = has_tap_activated_ability_phrase(&all_words);
    if parse_phrase_whole(
        &non_article_parser_word_refs(&base_tokens),
        ACTIVATED_ABILITY_WORDS,
    )
    .is_some()
    {
        return Ok(ObjectFilter::activated_ability());
    }
    if parse_phrase_whole(
        &non_article_parser_word_refs(&base_tokens),
        TRIGGERED_ABILITY_WORDS,
    )
    .is_some()
    {
        let mut filter = ObjectFilter::ability();
        filter.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        return Ok(filter);
    }
    if parse_phrase_choice_whole(
        &non_article_parser_word_refs(&base_tokens),
        ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES,
    )
    .is_some()
    {
        let mut triggered = ObjectFilter::ability();
        triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        let mut filter = ObjectFilter::default();
        filter.any_of = vec![ObjectFilter::activated_ability(), triggered];
        return Ok(filter);
    }

    // Qualified stack-ability sets (for example, "all other activated and
    // triggered abilities you control") must retain both their stack-object
    // identity and the outer controller/reference qualifiers. The exact-shape
    // branches above intentionally return early, but a qualified shape needs
    // to continue through the ordinary relation parser below.
    let ability_words = non_article_parser_word_refs(&base_tokens);
    if parse_phrase_choice_anywhere(&ability_words, SPELL_AND_ABILITY_PHRASES).is_some() {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        filter.has_mana_cost = false;
        filter.set_conjunctive_set_surface(true);
    } else if parse_phrase_choice_anywhere(&ability_words, ACTIVATED_OR_TRIGGERED_ABILITY_PHRASES)
        .is_some()
    {
        let mut triggered = ObjectFilter::ability();
        triggered.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
        filter.zone = Some(Zone::Stack);
        filter.any_of = vec![ObjectFilter::activated_ability(), triggered];
    } else if (parse_phrase_anywhere(&ability_words, &["activated", "ability"]).is_some()
        || parse_phrase_anywhere(&ability_words, &["activated", "abilities"]).is_some())
        && (!has_tap_activated_ability
            || crate::word_primitives::parse_any_sequence_prefix(
                &ability_words,
                &[&["activated", "ability"], &["activated", "abilities"]],
            ))
    {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::ActivatedAbility);
    } else if parse_phrase_anywhere(&ability_words, &["triggered", "ability"]).is_some()
        || parse_phrase_anywhere(&ability_words, &["triggered", "abilities"]).is_some()
    {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::TriggeredAbility);
    }
    if parse_phrase_choice_whole(
        &non_article_parser_word_refs(&base_tokens),
        REST_REVEALED_OBJECT_PHRASES,
    )
    .is_some()
    {
        return Ok(ObjectFilter::tagged(crate::tag::CompilerReferenceTag::Rest.bind()));
    }
    if let Some(filter) = parse_permanent_or_suspended_card_disjunction(&base_tokens) {
        return Ok(filter);
    }

    try_apply_distinct_powers_clause(&mut filter, &mut all_words);
    try_apply_distinct_mana_values_clause(&mut filter, &mut all_words);
    try_apply_distinct_creature_types_clause(&mut filter, &mut all_words);
    try_apply_no_shared_creature_type_with_your_creatures_or_graveyard_clause(
        &mut filter,
        &mut all_words,
    );
    try_apply_no_shared_creature_type_with_chosen_creature_clause(&mut filter, &mut all_words);
    try_apply_shared_creature_type_with_source_clause(&mut filter, &mut all_words);

    try_apply_could_be_targeted_by_that_spell_clause(&mut filter, &mut all_words);

    try_apply_blocked_or_was_blocked_by_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    )?;

    // "that were put there from the battlefield this turn" means the card entered
    // a graveyard from the battlefield this turn.
    try_apply_put_there_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "put there from their library this turn" is object-specific zone-change
    // history. Consume it before the ordinary zone parser can turn the
    // referenced library into a second current-zone union arm.
    try_apply_put_there_from_their_library_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "legendary or Rat card" (Nashi, Moon's Legacy) is a supertype/subtype disjunction.
    // We parse it by collecting both selectors and then expanding into an `any_of` filter
    // after the normal pass so other shared qualifiers (zone/owner/etc.) are preserved.
    let legendary_or_subtype = parse_phrase_anywhere(&all_words, LEGENDARY_OR_PREFIX)
        .and_then(|fact| all_words.get(fact.span.end).copied())
        .and_then(parse_subtype_word);

    // "in a graveyard that was put there from anywhere this turn" (Reenact the Crime)
    // means the card entered a graveyard this turn.
    try_apply_put_there_from_anywhere_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // A zone-qualified "put there this turn" clause has the same executable
    // graveyard-entry history as the explicit "from anywhere" surface.
    try_apply_put_there_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // "... graveyard from the battlefield this turn" means the card entered a graveyard
    // from the battlefield this turn.
    try_apply_graveyard_from_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // Preserve the source-relative history used by leaves-the-battlefield abilities such as
    // "a creature put onto the battlefield with this enchantment". This is an object-identity
    // relation, not a type clause; consume it before the ordinary noun pass can flatten the
    // source noun into the selected object's card types.
    try_apply_put_onto_battlefield_with_source_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // Token provenance is a source-instance relationship. Consume the source
    // noun before the ordinary type pass can misread "this enchantment" as a
    // requirement that the selected token itself be an enchantment.
    try_apply_created_with_source_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // Preserve negative turn history such as "creatures that didn't attack or
    // enter this turn" before the ordinary word pass can discard the
    // conjunctive predicate.
    try_apply_didnt_enter_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    // "... entered the battlefield ... this turn" marks a battlefield entry this turn.
    try_apply_entered_battlefield_this_turn_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_drawn_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    try_apply_counters_put_on_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    try_apply_ability_activated_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);
    try_apply_not_enchanted_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // Preserve damage history in ordinary object selectors such as "target
    // creature that was dealt damage this turn". This is a runtime legality
    // constraint, not disposable surface text.
    try_apply_was_dealt_damage_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);
    try_apply_dealt_damage_this_turn_clause(&mut filter, &mut all_words, &mut segment_tokens);

    // A prior player-or-planeswalker target can be referenced through either
    // the chosen player or the chosen planeswalker's controller. Remove that
    // relation before its `or planeswalker` surface is mistaken for a second
    // selected card type.
    try_apply_target_player_or_planeswalker_controller_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    if parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        BLOCKED_BY_TAGGED_OBJECT_PHRASES,
    )
    .is_some()
    {
        filter.blocked = true;
        filter.blocked_by = Some(crate::filter::ObjectRef::Tagged(
            crate::tag::CompilerReferenceTag::It.bind(),
        ));
    }

    // Avoid treating reference phrases like "... with mana value less than or equal to the number
    // of charge counters on this artifact" as additional type selectors on the filtered object.
    // (Aether Vial: "put a creature card with mana value equal to the number of charge counters
    // on this artifact from your hand onto the battlefield.")
    let _ = try_apply_mana_value_counters_on_source_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    );

    try_apply_attached_exclusion_phrases(&mut filter, &mut all_words);
    let exclude_basic_land_cards =
        strip_other_than_basic_land_cards_clause(&mut all_words, &mut segment_tokens);

    let _ = try_apply_pt_literal_prefix(&mut filter, &mut all_words);

    strip_object_filter_leading_prefixes(&mut all_words);

    let _ = try_apply_required_both_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_not_all_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_not_exactly_two_colors_clause(&mut filter, &mut all_words);

    let _ = try_apply_exactly_two_colors_clause(&mut filter, &mut all_words);

    strip_be_put_on_reference_prefix(&mut all_words, &segment_tokens);

    let _ = try_apply_leading_tagged_reference_prefix(&mut filter, &mut all_words);

    let _ = try_apply_target_choice_attribution_reference(&mut filter, &mut all_words);

    let _ = try_apply_entered_since_your_last_turn_ended_clause(&mut filter, &mut all_words);

    strip_object_filter_face_state_words(&mut filter, &mut all_words);

    if parse_phrase_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        ENTERED_THIS_TURN_UNSUPPORTED_PHRASE,
    )
    .is_some()
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported entered-this-turn object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    let has_counter_state_or_clause = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TAGGED_COUNTER_STATE_DISJUNCTION_PHRASES,
    )
    .is_some();
    let has_supported_suspended_disjunction = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        SUSPENDED_CARD_DISJUNCTION_PHRASES,
    )
    .is_some();
    if has_counter_state_or_clause && !has_supported_suspended_disjunction {
        return Err(CardTextError::ParseError(format!(
            "unsupported counter-state object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }
    strip_single_graveyard_phrase(&mut filter, &mut all_words);

    let _ = try_apply_not_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
        &base_tokens,
    )?;

    let _ = try_apply_named_clause(
        &mut filter,
        &mut all_words,
        &all_words_with_articles,
        &map_non_article_index,
        &map_non_article_end,
    )?;

    // "with the chosen name" — a runtime back-reference to a previously
    // chosen card name, not a literal name.
    if filter.name.is_none() {
        for phrase in [
            ["with", "chosen", "name"].as_slice(),
            ["of", "chosen", "name"].as_slice(),
        ] {
            if let Some(start) = crate::word_primitives::parse_sequence_start(&all_words, phrase) {
                filter.name = Some("{chosen name}".to_string());
                naming_and_reference::remove_word_range(
                    &mut all_words,
                    start,
                    start + phrase.len(),
                );
                break;
            }
        }
    }

    let _ = try_apply_color_count_phrase(&mut filter, &mut all_words)?;
    let _ = try_apply_sticker_filter_clause(&mut filter, &mut all_words);
    let has_power_or_toughness_clause = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        POWER_OR_TOUGHNESS_PHRASES,
    )
    .is_some();
    if has_power_or_toughness_clause
        && !all_words
            .iter()
            .any(|word| parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some())
    {
        return Err(CardTextError::ParseError(format!(
            "unsupported power-or-toughness object filter (clause: '{}')",
            all_words.join(" ")
        )));
    }

    // A sharing clause compares the candidate's characteristics with a
    // separately filtered object set. Parse and remove the entire relation
    // before the ordinary noun/controller passes can leak the comparison
    // object's identity into the candidate filter.
    let _ = try_apply_shared_characteristic_relation_clause(
        &mut filter,
        &mut all_words,
        &mut segment_tokens,
    )?;

    let explicit_card_rhs_ranges = filter_comparison_rhs_ranges(&all_words)?;
    if contains_explicit_card_noun(&all_words, &explicit_card_rhs_ranges) {
        filter.set_explicit_card_noun(true);
    }

    let reference_stage =
        apply_reference_and_tag_stage(&mut filter, &mut all_words, &mut segment_tokens);
    if reference_stage.early_return {
        return Ok(filter);
    }
    let source_linked_exile_reference = reference_stage.source_linked_exile_reference;

    let references_target_player = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TARGET_PLAYER_REFERENCE_PHRASES,
    )
    .is_some();
    let references_target_opponent = parse_phrase_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TARGET_OPPONENT_REFERENCE_PHRASES,
    )
    .is_some();
    let pronoun_player_filter = if references_target_opponent {
        PlayerFilter::target_opponent()
    } else if references_target_player {
        PlayerFilter::target_player()
    } else {
        PlayerFilter::IteratedPlayer
    };
    let comparison_rhs_ranges = filter_comparison_rhs_ranges(&all_words)?;

    let outer_filter_words = all_words
        .iter()
        .enumerate()
        .map(|(idx, word)| {
            if word_is_in_ranges(idx, &comparison_rhs_ranges) {
                "__comparison_rhs__"
            } else {
                *word
            }
        })
        .collect::<Vec<_>>();
    let has_attack_destination_planeswalker_clause =
        crate::slice_primitives::select_position(&all_words_with_articles, |word| {
            matches!(*word, "attacking" | "attacks")
        })
        .is_some_and(|attacking_idx| {
            all_words_with_articles[..attacking_idx]
                .iter()
                .any(|word| matches!(*word, "creature" | "creatures"))
                && all_words_with_articles[attacking_idx + 1..]
                    .iter()
                    .any(|word| matches!(*word, "planeswalker" | "planeswalkers"))
        });
    if let Some(attacking_filter) =
        attacking_player_filter_from_words(&outer_filter_words, &pronoun_player_filter)
    {
        filter.attacking_player_or_planeswalker_controlled_by = Some(attacking_filter);
        filter.attacking_player_only = !has_attack_destination_planeswalker_clause
            && !crate::word_primitives::sequence_occurs(&outer_filter_words, &["planeswalker"]);
    }

    let is_outer_tagged_spell_reference_at = |idx: usize| {
        outer_filter_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| parse_word_choice(prev, TAGGED_SPELL_REFERENCE_WORDS).is_some())
    };
    let contains_unqualified_spell_word =
        outer_filter_words.iter().enumerate().any(|(idx, word)| {
            parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some()
                && !is_outer_tagged_spell_reference_at(idx)
        });
    let is_tagged_spell_reference_at = |idx: usize| {
        all_words
            .get(idx.wrapping_sub(1))
            .is_some_and(|prev| parse_word_choice(prev, TAGGED_SPELL_REFERENCE_WORDS).is_some())
    };
    let mentions_ability_word = outer_filter_words
        .iter()
        .any(|word| parse_word_choice(word, ABILITY_OR_ABILITIES_WORDS).is_some());
    if contains_unqualified_spell_word && !mentions_ability_word {
        filter.has_mana_cost = true;
    }
    // Both current and older Oracle surfaces narrow a spell/permanent filter
    // to objects whose printed mana cost includes an {X} symbol.
    let has_x_in_cost_surface =
        parse_phrase_anywhere(&outer_filter_words, &["mana", "cost", "that", "contains"]).is_some()
            || [
                &["with", "x", "in", "its", "mana", "cost"][..],
                &["with", "x", "in", "their", "mana", "cost"][..],
                &["with", "x", "in", "its", "mana", "costs"][..],
                &["with", "x", "in", "their", "mana", "costs"][..],
            ]
            .iter()
            .any(|phrase| parse_phrase_anywhere(&outer_filter_words, phrase).is_some());
    if has_x_in_cost_surface {
        filter.has_x_in_cost = true;
    }

    if !all_words.is_empty() {
        let mut idx = 0usize;
        while idx < all_words.len() {
            if word_is_in_ranges(idx, &comparison_rhs_ranges) {
                idx += 1;
                continue;
            }
            let slice = &all_words[idx..];
            if relation_clause_is_inside_aggregate_scope(&all_words, idx) {
                idx += 1;
                continue;
            }
            if let Some(consumed) =
                try_apply_neither_owned_nor_controlled_clause(&mut filter, slice)
            {
                idx += consumed;
                continue;
            }
            if let Some(consumed) =
                try_apply_joint_owner_controller_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) = try_apply_chosen_player_graveyard_clause(&mut filter, slice) {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_negated_you_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_player_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            if let Some(consumed) =
                try_apply_passive_player_relation_clause(&mut filter, slice, &pronoun_player_filter)
            {
                idx += consumed.max(1);
                continue;
            }
            idx += 1;
        }
    }

    let mut with_idx = 0usize;
    while with_idx + 1 < all_words.len() {
        if word_is_in_ranges(with_idx, &comparison_rhs_ranges) {
            with_idx += 1;
            continue;
        }
        if all_words[with_idx] != WITH_WORD {
            with_idx += 1;
            continue;
        }

        if let Some(consumed) = try_apply_with_clause_tail(&mut filter, &all_words[with_idx + 1..])
        {
            with_idx += 1 + consumed;
            continue;
        }

        with_idx += 1;
    }

    let mut has_idx = 0usize;
    while has_idx + 1 < all_words.len() {
        if word_is_in_ranges(has_idx, &comparison_rhs_ranges) {
            has_idx += 1;
            continue;
        }
        if parse_word_choice(all_words[has_idx], HAS_HAVE_WORDS).is_none() {
            has_idx += 1;
            continue;
        }
        if filter.with_counter.is_none()
            && let Some((counter_constraint, consumed)) =
                parse_filter_counter_constraint_words(&all_words[has_idx + 1..])
        {
            filter.with_counter = Some(counter_constraint);
            has_idx += 1 + consumed;
            continue;
        }
        if let Some((constraints, connective, consumed)) =
            parse_filter_keyword_constraint_list_words(&all_words[has_idx + 1..])
        {
            // "that doesn't have <keywords>" — the negation word precedes the
            // has/have word and inverts every list item (has NONE of them).
            let negated = (has_idx > 0
                && matches!(
                    all_words[has_idx - 1],
                    "doesn't" | "doesnt" | "don't" | "dont"
                ))
                || (has_idx > 1
                    && all_words[has_idx - 1] == "not"
                    && matches!(all_words[has_idx - 2], "does" | "do"));
            if negated {
                for constraint in constraints {
                    apply_filter_keyword_constraint(&mut filter, constraint, true);
                }
            } else if constraints.len() > 1
                && filter.any_of.is_empty()
                && !matches!(connective, FilterKeywordListConnective::And)
            {
                // A disjunctive list ("first strike, double strike, and/or
                // haste") matches objects with AT LEAST ONE listed keyword.
                filter.any_of = constraints
                    .into_iter()
                    .map(|constraint| {
                        let mut branch = ObjectFilter::default();
                        apply_filter_keyword_constraint(&mut branch, constraint, false);
                        branch
                    })
                    .collect();
                if matches!(connective, FilterKeywordListConnective::AndOr) {
                    filter.set_union_connective(ObjectFilterUnionConnective::AndOr);
                }
            } else {
                for constraint in constraints {
                    apply_filter_keyword_constraint(&mut filter, constraint, false);
                }
            }
            has_idx += 1 + consumed;
            continue;
        }
        has_idx += 1;
    }

    let mut without_idx = 0usize;
    while without_idx + 1 < all_words.len() {
        if word_is_in_ranges(without_idx, &comparison_rhs_ranges) {
            without_idx += 1;
            continue;
        }
        if all_words[without_idx] != WITHOUT_WORD {
            without_idx += 1;
            continue;
        }

        if let Some(consumed) =
            try_apply_without_clause_tail(&mut filter, &all_words[without_idx + 1..])
        {
            without_idx += 1 + consumed;
            continue;
        }

        without_idx += 1;
    }

    if has_tap_activated_ability {
        filter.has_tap_activated_ability = true;
    }

    let mut referenced_zones = Vec::new();
    for idx in 0..all_words.len() {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if let Some(zone) = parse_zone_word(all_words[idx]) {
            if !slice_has(&referenced_zones, &zone) {
                referenced_zones.push(zone);
            }
            let is_reference_zone_for_spell = if contains_unqualified_spell_word {
                idx > 0
                    && matches!(
                        all_words[idx - 1],
                        "controller"
                            | "controllers"
                            | "owner"
                            | "owners"
                            | "its"
                            | "their"
                            | "that"
                            | "this"
                    )
            } else {
                false
            };
            if is_reference_zone_for_spell {
                continue;
            }
            if filter.zone.is_none() {
                filter.zone = Some(zone);
            }
            if idx > 0 {
                match all_words[idx - 1] {
                    "your" => {
                        filter.owner = Some(PlayerFilter::You);
                    }
                    "opponent" | "opponents" => {
                        filter.owner = Some(PlayerFilter::Opponent);
                    }
                    "their" => {
                        filter.owner = Some(pronoun_player_filter.clone());
                    }
                    _ => {}
                }
            }
            if idx > 1 {
                let owner_pair = (all_words[idx - 2], all_words[idx - 1]);
                match owner_pair {
                    ("defending", "player") | ("defending", "players") => {
                        filter.owner = Some(PlayerFilter::Defending);
                    }
                    ("target", "player") | ("target", "players") => {
                        filter.owner = Some(PlayerFilter::target_player());
                    }
                    ("target", "opponent") | ("target", "opponents") => {
                        filter.owner = Some(PlayerFilter::target_opponent());
                    }
                    ("that", "player") | ("that", "players") => {
                        filter.owner = Some(PlayerFilter::IteratedPlayer);
                    }
                    _ => {}
                }
            }
        }
    }
    if referenced_zones.len() > 1 && filter.any_of.is_empty() {
        filter.zone = None;
        filter.any_of = referenced_zones
            .into_iter()
            .map(|zone| ObjectFilter::default().in_zone(zone))
            .collect();
    }

    let clause_words = all_words.clone();
    for idx in 0..all_words.len() {
        let value_tokens = match all_words.get(idx..) {
            Some(["total", "power", "and", "toughness", rest @ ..])
            | Some(["power", "and", "toughness", "totaling", rest @ ..]) => rest,
            _ => continue,
        };
        let Some((cmp, _consumed)) =
            parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
        else {
            continue;
        };
        filter.total_power_toughness = Some(cmp);
        break;
    }

    for idx in 0..all_words.len() {
        let (is_base_reference, pt_word_idx) = if idx + 4 < all_words.len()
            && parse_phrase_at_head(&all_words[idx..], BASE_POWER_TOUGHNESS_PREFIX).is_some()
        {
            (true, idx + 4)
        } else if idx + 3 < all_words.len()
            && parse_phrase_at_head(&all_words[idx..], POWER_TOUGHNESS_PREFIX).is_some()
            && (idx == 0 || all_words[idx - 1] != BASE_WORD)
        {
            (false, idx + 3)
        } else {
            continue;
        };

        if let Ok((power, toughness)) = parse_pt_modifier(all_words[pt_word_idx]) {
            filter.power = Some(crate::filter::Comparison::Equal(power));
            filter.toughness = Some(crate::filter::Comparison::Equal(toughness));
            filter.power_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
            filter.toughness_reference = if is_base_reference {
                crate::filter::PtReference::Base
            } else {
                crate::filter::PtReference::Effective
            };
        }
    }

    let mut idx = 0usize;
    while idx < all_words.len() {
        let axis = if all_words[idx] == POWER_WORD {
            Some("power")
        } else if all_words[idx] == TOUGHNESS_WORD {
            Some("toughness")
        } else if idx + 1 < all_words.len()
            && parse_phrase_at_head(&all_words[idx..], MANA_VALUE_PREFIX).is_some()
        {
            Some("mana value")
        } else {
            None
        };
        let Some(axis) = axis else {
            idx += 1;
            continue;
        };
        let is_base_reference = idx > 0 && all_words[idx - 1] == BASE_WORD;

        let axis_word_count =
            usize::from(parse_phrase_at_head(&all_words[idx..], MANA_VALUE_PREFIX).is_some()) + 1;
        let value_tokens = if idx + axis_word_count < all_words.len() {
            &all_words[idx + axis_word_count..]
        } else {
            &[]
        };
        if axis == POWER_WORD && value_tokens.first().is_some_and(|word| *word == AND_WORD) {
            idx += 1;
            continue;
        }
        if axis == TOUGHNESS_WORD
            && idx >= 3
            && matches!(
                &all_words[idx - 3..idx],
                ["total", "power", "and"] | ["base", "power", "and"] | ["power", "and", "base"]
            )
        {
            idx += 1;
            continue;
        }
        if (axis == TOUGHNESS_WORD
            && parse_phrase_choice_at_head(&all_words[idx..], TOUGHNESS_GREATER_THAN_POWER_PHRASES)
                .is_some())
            || (axis == POWER_WORD
                && parse_phrase_choice_at_head(
                    &all_words[idx..],
                    POWER_GREATER_THAN_TOUGHNESS_PHRASES,
                )
                .is_some())
            || parse_phrase_choice_at_head(&all_words[idx..], POWER_TOUGHNESS_NOT_EQUAL_PHRASES)
                .is_some()
        {
            idx += 1;
            continue;
        }
        let Some((cmp, consumed)) =
            parse_filter_comparison_tokens(axis, value_tokens, &clause_words)?
        else {
            idx += 1;
            continue;
        };

        match axis {
            "power" => {
                filter.power = Some(cmp);
                filter.power_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "toughness" => {
                filter.toughness = Some(cmp);
                filter.toughness_reference = if is_base_reference {
                    crate::filter::PtReference::Base
                } else {
                    crate::filter::PtReference::Effective
                };
            }
            "mana value" => filter.mana_value = Some(cmp),
            _ => {}
        }
        idx += axis_word_count + consumed;
    }

    apply_parity_filter_phrases(&clause_words, &mut filter);

    if parse_phrase_anywhere(&clause_words, POWER_GREATER_THAN_BASE_POWER_PHRASE).is_some() {
        filter.power_greater_than_base_power = true;
    }
    if parse_phrase_choice_anywhere(&clause_words, TOUGHNESS_GREATER_THAN_POWER_PHRASES).is_some() {
        let relation = crate::filter::PowerToughnessRelation::ToughnessGreaterThanPower;
        filter.power_toughness_relation = Some(relation);
        clear_redundant_power_toughness_axis_filter(&mut filter, relation);
    } else if parse_phrase_choice_anywhere(&clause_words, POWER_GREATER_THAN_TOUGHNESS_PHRASES)
        .is_some()
    {
        let relation = crate::filter::PowerToughnessRelation::PowerGreaterThanToughness;
        filter.power_toughness_relation = Some(relation);
        clear_redundant_power_toughness_axis_filter(&mut filter, relation);
    } else if parse_phrase_choice_anywhere(&clause_words, POWER_TOUGHNESS_NOT_EQUAL_PHRASES)
        .is_some()
    {
        filter.power_toughness_relation = Some(crate::filter::PowerToughnessRelation::NotEqual);
    }

    let mut saw_permanent = false;
    let mut saw_spell = false;
    let mut saw_permanent_type = false;

    let mut saw_subtype = false;
    let mut negated_word_indices = std::collections::HashSet::new();
    let mut negated_historic_indices = std::collections::HashSet::new();
    let mut has_coordinated_negated_characteristic_list = false;
    let is_text_negation_word = |word: &str| parse_word_choice(word, TEXT_NEGATION_WORDS).is_some();
    for idx in 0..all_words.len().saturating_sub(1) {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if all_words[idx] != NON_WORD {
            continue;
        }
        let next = all_words[idx + 1];
        if is_outlaw_word(next) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(card_type) = parse_card_type(next)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(idx + 1);
        }
        if next == ATTACKING_WORD {
            filter.nonattacking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == BLOCKING_WORD {
            filter.nonblocking = true;
            negated_word_indices.insert(idx + 1);
        }
        if next == BLOCKED_WORD {
            filter.unblocked = true;
            negated_word_indices.insert(idx + 1);
        }
        if parse_word_choice(next, COMMANDER_OR_COMMANDERS_WORDS).is_some() {
            filter.noncommander = true;
            negated_word_indices.insert(idx + 1);
        }
        if let Some(color) = parse_color(next) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(idx + 1);
        }
        if let Some(subtype) = parse_subtype_flexible(next)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(idx + 1);
        }
    }
    for idx in 0..all_words.len() {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if !is_text_negation_word(all_words[idx]) {
            continue;
        }
        let mut target_idx = idx + 1;
        if target_idx >= all_words.len() {
            continue;
        }
        if is_article(all_words[target_idx]) {
            target_idx += 1;
            if target_idx >= all_words.len() {
                continue;
            }
        }

        let negated_word = all_words[target_idx];
        if negated_word == ATTACKING_WORD {
            filter.nonattacking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == BLOCKING_WORD {
            filter.nonblocking = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == BLOCKED_WORD {
            filter.unblocked = true;
            negated_word_indices.insert(target_idx);
        }
        if negated_word == HISTORIC_WORD {
            filter.nonhistoric = true;
            negated_historic_indices.insert(target_idx);
        }
        if parse_word_choice(negated_word, COMMANDER_OR_COMMANDERS_WORDS).is_some() {
            filter.noncommander = true;
            negated_word_indices.insert(target_idx);
        }
        if let Some(card_type) = parse_card_type(negated_word)
            && !slice_has(&filter.excluded_card_types, &card_type)
        {
            filter.excluded_card_types.push(card_type);
            negated_word_indices.insert(target_idx);
        }
        if let Some(supertype) = parse_supertype_word(negated_word)
            && !slice_has(&filter.excluded_supertypes, &supertype)
        {
            filter.excluded_supertypes.push(supertype);
            negated_word_indices.insert(target_idx);
        }
        if let Some(color) = parse_color(negated_word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            negated_word_indices.insert(target_idx);
        }
        if let Some(subtype) = parse_subtype_flexible(negated_word)
            && !slice_has(&filter.excluded_subtypes, &subtype)
        {
            filter.excluded_subtypes.push(subtype);
            negated_word_indices.insert(target_idx);
        }

        // A single negated copula scopes over the entire coordinated type
        // list: “isn't an Insect, Rat, Spider, or Squirrel” excludes every
        // listed subtype, not just the first one. Token punctuation has
        // already been removed here, so walk through conjunctions/articles
        // until the first word that is not another characteristic.
        let mut coordinated_characteristic_count = usize::from(
            parse_card_type(negated_word).is_some()
                || parse_supertype_word(negated_word).is_some()
                || parse_color(negated_word).is_some()
                || parse_subtype_flexible(negated_word).is_some(),
        );
        let mut list_idx = target_idx + 1;
        while list_idx < all_words.len() {
            let word = all_words[list_idx];
            if matches!(word, "and" | "or" | "and/or") || is_article(word) {
                list_idx += 1;
                continue;
            }
            let mut recognized = false;
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut filter.excluded_card_types, card_type);
                recognized = true;
            }
            if let Some(supertype) = parse_supertype_word(word) {
                push_unique(&mut filter.excluded_supertypes, supertype);
                recognized = true;
            }
            if let Some(color) = parse_color(word) {
                filter.excluded_colors = filter.excluded_colors.union(color);
                recognized = true;
            }
            if let Some(subtype) = parse_subtype_flexible(word) {
                push_unique(&mut filter.excluded_subtypes, subtype);
                recognized = true;
            }
            if !recognized {
                break;
            }
            coordinated_characteristic_count += 1;
            negated_word_indices.insert(list_idx);
            list_idx += 1;
        }
        has_coordinated_negated_characteristic_list |= coordinated_characteristic_count > 1;
    }
    for idx in 0..all_words.len().saturating_sub(1) {
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        if parse_phrase_whole(&all_words[idx..idx + 2], NOT_HISTORIC_PHRASE).is_some() {
            filter.nonhistoric = true;
            negated_historic_indices.insert(idx + 1);
        }
    }

    let excluded_chosen_type_indices: std::collections::HashSet<usize> =
        EXCLUDED_CHOSEN_TYPE_PHRASES
            .iter()
            .filter_map(|phrase| {
                parse_phrase_anywhere(&all_words, phrase)
                    .map(|fact| fact.span.end.saturating_sub(2))
            })
            .collect();
    let original_parser_words = parser_token_word_refs(tokens);
    if EXCLUDED_TYPE_CHOSEN_THIS_WAY_PHRASES
        .iter()
        .any(|phrase| parse_phrase_anywhere(&original_parser_words, phrase).is_some())
    {
        filter.excluded_any_chosen_creature_type = true;
        filter.set_chosen_type_this_way_surface(true);
    }

    if parse_phrase_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        ATTACKED_THIS_TURN_PHRASE,
    )
    .is_some()
    {
        filter.attacked_this_turn = true;
    }

    let blocked_this_turn_word_indices = all_words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| {
            (*word == "blocked"
                && all_words.get(idx + 1) == Some(&"this")
                && all_words.get(idx + 2) == Some(&"turn")
                && !idx
                    .checked_sub(1)
                    .and_then(|previous| all_words.get(previous))
                    .is_some_and(|word| matches!(*word, "was" | "became" | "is")))
            .then_some(idx)
        })
        .collect::<std::collections::HashSet<_>>();
    if !blocked_this_turn_word_indices.is_empty() {
        filter.blocked_this_turn = true;
    }

    for negated_phrase in [
        ["didn't", "attack", "this", "turn"],
        ["didnt", "attack", "this", "turn"],
    ] {
        if parse_phrase_anywhere(
            &non_article_parser_word_refs(&segment_tokens),
            &negated_phrase,
        )
        .is_some()
        {
            filter.didnt_attack_this_turn = true;
            filter.attacked_this_turn = false;
        }
    }

    let basic_land_type_basic_indices = all_words
        .iter()
        .enumerate()
        .filter_map(|(idx, word)| {
            (*word == "basic"
                && all_words.get(idx + 1) == Some(&"land")
                && all_words
                    .get(idx + 2)
                    .is_some_and(|kind| matches!(*kind, "type" | "types")))
            .then_some(idx)
        })
        .collect::<std::collections::HashSet<_>>();

    for (idx, word) in all_words.iter().enumerate() {
        let idx: usize = idx;
        if word_is_in_ranges(idx, &comparison_rhs_ranges) {
            continue;
        }
        let is_negated_word = set_has(&negated_word_indices, &idx);
        match *word {
            "permanent" | "permanents" => saw_permanent = true,
            "spell" | "spells" => {
                if !is_tagged_spell_reference_at(idx) {
                    saw_spell = true;
                }
            }
            word if word == CHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == COLOR_WORD) =>
            {
                filter.chosen_color = true;
            }
            word if word == THAT_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == COLOR_WORD) =>
            {
                // A demonstrative color after a color choice ("creatures of
                // that color") refers to the source program's chosen color.
                filter.chosen_color = true;
            }
            word if word == CHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                if set_has(&excluded_chosen_type_indices, &idx) {
                    filter.excluded_chosen_creature_type = true;
                } else {
                    filter.chosen_creature_type = true;
                }
            }
            word if word == THAT_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                filter.chosen_creature_type = true;
            }
            word if word == NONCHOSEN_WORD
                && all_words
                    .get(idx + 1)
                    .is_some_and(|next| *next == TYPE_WORD) =>
            {
                filter.excluded_chosen_creature_type = true;
            }
            "token" | "tokens" => filter.token = true,
            "nontoken" => filter.nontoken = true,
            "foretold" if !is_negated_word => filter.foretold = true,
            "other" => filter.other = true,
            "tapped" => filter.tapped = true,
            "untapped" => filter.untapped = true,
            "attacking" if !is_negated_word => filter.attacking = true,
            "nonattacking" => filter.nonattacking = true,
            // A bare "equipped" adjective not consumed by the attached-to
            // reference paths is the generic has-Equipment state.
            // NOTE(2026-07-25): a copula guard (skip when preceded by
            // is/are/was) was tried for Enkira's "As long as Enkira is
            // equipped, it must be blocked" and REVERTED — with the guard the
            // line HARD-FAILS ("parser does not yet support line family"),
            // meaning the predicate route that used to claim it is gone;
            // find that regression before re-adding the guard.
            "equipped" if !is_negated_word => {
                filter.tagged_constraints.push(TaggedObjectConstraint {
                    tag: crate::tag::CompilerReferenceTag::Equipped.bind(),
                    relation: TaggedOpbjectRelation::IsTaggedObject,
                });
            }
            "blocking" if !is_negated_word => filter.blocking = true,
            "nonblocking" => filter.nonblocking = true,
            "blocked" if !is_negated_word && !set_has(&blocked_this_turn_word_indices, &idx) => {
                filter.blocked = true;
            }
            "unblocked" if !is_negated_word => filter.unblocked = true,
            "commander" | "commanders" => {
                let prev = idx.checked_sub(1).and_then(|i| all_words.get(i)).copied();
                let prev2 = idx.checked_sub(2).and_then(|i| all_words.get(i)).copied();
                let negated_by_phrase = prev.is_some_and(is_text_negation_word)
                    || (prev.is_some_and(is_article) && prev2.is_some_and(is_text_negation_word));
                if is_negated_word || negated_by_phrase {
                    filter.noncommander = true;
                } else {
                    filter.is_commander = true;
                    match prev {
                        Some("your") => filter.owner = Some(PlayerFilter::You),
                        Some("opponent") | Some("opponents") => {
                            filter.owner = Some(PlayerFilter::Opponent);
                        }
                        Some("their") => filter.owner = Some(pronoun_player_filter.clone()),
                        _ => {}
                    }
                }
            }
            "noncommander" | "noncommanders" => filter.noncommander = true,
            "basic" if set_has(&basic_land_type_basic_indices, &idx) => {
                filter.has_basic_land_type = true;
            }
            "nonbasic" => {
                if all_words.get(idx + 1).is_some_and(|word| *word == "land")
                    && all_words.get(idx + 2).is_some_and(|word| *word == "type")
                {
                    filter.has_nonbasic_land_type = true;
                    continue;
                }
                filter = filter.without_supertype(Supertype::Basic);
            }
            "colorless" => filter.colorless = true,
            "multicolored" => filter.multicolored = true,
            "monocolored" => filter.monocolored = true,
            "nonhistoric" => filter.nonhistoric = true,
            "historic" if !set_has(&negated_historic_indices, &idx) => filter.historic = true,
            "modified" if !is_negated_word => filter.modified = true,
            "suspected" if !is_negated_word => filter.suspected = true,
            _ => {}
        }

        if is_non_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.excluded_subtypes);
            continue;
        }

        if set_has(&negated_word_indices, &idx) {
            continue;
        }

        if is_outlaw_word(word) {
            push_outlaw_subtypes(&mut filter.subtypes);
            saw_subtype = true;
            continue;
        }

        let mut parsed_explicit_exclusion = false;
        if let Some(card_type) = parse_non_type(word) {
            push_unique(&mut filter.excluded_card_types, card_type);
            parsed_explicit_exclusion = true;
        }

        if let Some(supertype) = parse_non_supertype(word) {
            if !slice_has(&filter.excluded_supertypes, &supertype) {
                filter.excluded_supertypes.push(supertype);
            }
            parsed_explicit_exclusion = true;
        }

        if let Some(color) = parse_non_color(word) {
            filter.excluded_colors = filter.excluded_colors.union(color);
            parsed_explicit_exclusion = true;
        }
        if let Some(subtype) = parse_non_subtype(word) {
            if !slice_has(&filter.excluded_subtypes, &subtype) {
                filter.excluded_subtypes.push(subtype);
            }
            parsed_explicit_exclusion = true;
        }

        // Flexible positive characteristic parsers deliberately accept some
        // prefixed surfaces. Once this word has been recognized as an
        // explicit `non-*` exclusion, do not feed the same word through those
        // positive parsers as well: `non-Equipment` must not require and
        // exclude Equipment simultaneously.
        if parsed_explicit_exclusion {
            continue;
        }

        if let Some(color) = parse_color(word) {
            let existing = filter.colors.unwrap_or(ColorSet::new());
            filter.colors = Some(existing.union(color));
        }

        if let Some(supertype) = parse_supertype_word(word)
            && !set_has(&basic_land_type_basic_indices, &idx)
            && !slice_has(&filter.supertypes, &supertype)
        {
            filter.supertypes.push(supertype);
        }

        if let Some(card_type) = parse_card_type(word) {
            filter.set_explicit_card_type_noun(Some(card_type));
            push_unique(&mut filter.card_types, card_type);
            if is_permanent_type(card_type) {
                saw_permanent_type = true;
            }
        }

        if let Some(subtype) = parse_compound_filter_subtype(&all_words, idx) {
            push_unique(&mut filter.subtypes, subtype);
            saw_subtype = true;
        }
    }
    if crate::word_primitives::sequence_occurs(&all_words_with_articles, &["attacking", "alone"]) {
        filter.attacking = true;
        filter.attacking_alone = true;
    }
    // In “shares a creature type with each creature tapped this way”, tapped
    // qualifies the cost objects on the right-hand side, not the candidate
    // card being filtered. Preserve an independent leading `tapped` qualifier
    // when one is also present on the candidate itself.
    if let Some(reference_tapped_idx) =
        crate::word_primitives::parse_sequence_start(&all_words, &["tapped", "this", "way"])
        && !all_words
            .iter()
            .enumerate()
            .any(|(idx, word)| *word == "tapped" && idx != reference_tapped_idx)
    {
        filter.tapped = false;
    }

    if saw_spell && source_linked_exile_reference {
        // "spell ... exiled with this" describes a stack spell with a relation
        // to source-linked exiled cards, not a spell object in exile.
        filter.zone = Some(Zone::Stack);
    }

    let segments = split_lexed_slices_on_or(&segment_tokens);
    let mut segment_types = Vec::new();
    let mut segment_subtypes = Vec::new();
    let mut segment_marker_counts = Vec::new();
    let mut segment_words_lists: Vec<Vec<String>> = Vec::new();

    for segment in &segments {
        let segment_words: Vec<String> = non_article_parser_word_refs(segment)
            .into_iter()
            .map(ToString::to_string)
            .collect();
        segment_words_lists.push(segment_words.clone());
        let segment_word_refs = segment_words.iter().map(String::as_str).collect::<Vec<_>>();
        let segment_comparison_rhs_ranges = filter_comparison_rhs_ranges(&segment_word_refs)?;
        // Everything after "named" is a card name, already claimed as one by
        // the name clause. Its words are not characteristics: "named Cleric of
        // the Forward Order" must not also constrain the filter to Clerics.
        let name_clause_start =
            crate::word_primitives::parse_sequence_start(&segment_word_refs, &["named"])
                .unwrap_or(segment_word_refs.len());
        let mut types = Vec::new();
        let mut subtypes = Vec::new();
        for (word_idx, word) in segment_words.iter().enumerate() {
            if word_idx >= name_clause_start {
                break;
            }
            if word_is_in_ranges(word_idx, &segment_comparison_rhs_ranges) {
                continue;
            }
            // The primary characteristic pass has already recorded explicit
            // `non-*` atoms as exclusions. Suffix recovery must not feed the
            // same atom through the permissive positive parsers and recreate
            // an impossible "has and does not have" filter.
            if parse_non_type(word).is_some()
                || parse_non_supertype(word).is_some()
                || parse_non_color(word).is_some()
                || parse_non_subtype(word).is_some()
            {
                continue;
            }
            // The lexer splits "non-Wall" into ["non", "wall"]; the earlier
            // characteristic pass recorded the exclusion against ITS index
            // space, which this per-segment scan does not share. Skip any
            // atom directly preceded by a negation word so the excluded
            // characteristic is not re-added positively.
            if word_idx > 0
                && (segment_word_refs[word_idx - 1] == NON_WORD
                    || parse_word_choice(segment_word_refs[word_idx - 1], TEXT_NEGATION_WORDS)
                        .is_some())
            {
                continue;
            }
            if let Some(card_type) = parse_card_type(word) {
                push_unique(&mut types, card_type);
            }
            if let Some(subtype) = parse_compound_filter_subtype(&segment_word_refs, word_idx) {
                push_unique(&mut subtypes, subtype);
            }
        }
        segment_marker_counts.push(types.len() + subtypes.len());
        if !types.is_empty() {
            segment_types.push(types);
        }
        if !subtypes.is_empty() {
            segment_subtypes.push(subtypes);
        }
    }

    if segments.len() > 1 {
        let qualifier_in_all_segments = |qualifier: &str| {
            segment_words_lists.iter().all(|segment| {
                let segment_refs = segment.iter().map(String::as_str).collect::<Vec<_>>();
                parse_word_choice_anywhere(&segment_refs, &[qualifier]).is_some()
            })
        };
        let shared_leading_qualifier = |qualifier: &str, opposite: &str| {
            if qualifier_in_all_segments(qualifier) {
                return true;
            }
            if parse_word_choice_anywhere(&all_words, &[opposite]).is_some() {
                return false;
            }
            let Some(first_segment) = segment_words_lists.first() else {
                return false;
            };
            let first_segment_refs = first_segment.iter().map(String::as_str).collect::<Vec<_>>();
            if parse_word_choice_anywhere(&first_segment_refs, &[qualifier]).is_none() {
                return false;
            }
            segment_words_lists.iter().skip(1).all(|segment| {
                let segment_refs = segment.iter().map(String::as_str).collect::<Vec<_>>();
                parse_word_choice_anywhere(&segment_refs, &[opposite]).is_none()
            })
        };

        if filter.tapped && !shared_leading_qualifier("tapped", "untapped") {
            filter.tapped = false;
        }
        if filter.untapped && !shared_leading_qualifier("untapped", "tapped") {
            filter.untapped = false;
        }
    }

    if segments.len() > 1 {
        if !has_coordinated_negated_characteristic_list {
            let type_list_candidate = !segment_marker_counts.is_empty()
                && segment_marker_counts.iter().all(|count| *count == 1);

            if type_list_candidate {
                let mut any_types = Vec::new();
                let mut any_subtypes = Vec::new();
                for types in segment_types {
                    let Some(card_type) = types.first().copied() else {
                        continue;
                    };
                    push_unique(&mut any_types, card_type);
                }
                for subtypes in segment_subtypes {
                    let Some(subtype) = subtypes.first().copied() else {
                        continue;
                    };
                    push_unique(&mut any_subtypes, subtype);
                }
                if !any_types.is_empty() {
                    filter.card_types = any_types;
                }
                if !any_subtypes.is_empty() {
                    filter.subtypes = any_subtypes;
                    filter.all_subtypes.clear();
                }
                if !filter.card_types.is_empty() && !filter.subtypes.is_empty() {
                    filter.type_or_subtype_union = true;
                }
            }
        }
    } else {
        let types = segment_types.into_iter().next().unwrap_or_default();
        let subtypes = segment_subtypes.into_iter().next().unwrap_or_default();
        let normalized_segment_words = non_article_parser_word_refs(&segment_tokens);
        // Only a connector between characteristic atoms makes those atoms an
        // inclusive list. A later suffix connector ("you own and control")
        // must not turn an adjacent compound type or subtype phrase into OR.
        let characteristic_word_indices = normalized_segment_words
            .iter()
            .enumerate()
            .filter_map(|(idx, word)| {
                (parse_card_type(word).is_some()
                    || parse_compound_filter_subtype(&normalized_segment_words, idx).is_some())
                .then_some(idx)
            })
            .collect::<Vec<_>>();
        let has_conjunction = characteristic_word_indices
            .first()
            .zip(characteristic_word_indices.last())
            .is_some_and(|(first, last)| {
                normalized_segment_words[*first..=*last].iter().any(|word| {
                    TYPE_LIST_CONJUNCTION_WORDS
                        .iter()
                        .any(|conjunction| conjunction == word)
                })
            });
        let has_and = parse_word_choice_anywhere(&normalized_segment_words, &["and"]).is_some();
        let has_or = parse_word_choice_anywhere(&normalized_segment_words, &["or"]).is_some();
        let has_and_or =
            parse_word_choice_anywhere(&normalized_segment_words, &["and/or"]).is_some();
        if types.len() > 1 {
            if has_conjunction {
                filter.card_types = types;
            } else {
                filter.all_card_types = types;
            }
        } else if types.len() == 1 {
            filter.card_types = types;
        }
        // The fast typed filter parser may already have recognized a compound
        // subtype phrase. Preserve that complete set: the flexible reference
        // scan intentionally recognizes fewer subtype spellings and must not
        // replace it with a partial subset.
        if filter.all_subtypes.is_empty() {
            if subtypes.len() > 1 {
                if has_conjunction {
                    filter.subtypes = subtypes;
                } else {
                    filter.all_subtypes = subtypes;
                    filter.subtypes.clear();
                }
            } else if subtypes.len() == 1 {
                filter.subtypes = subtypes;
            }
        }
        if (has_and_or || (has_and && has_or))
            && !filter.card_types.is_empty()
            && !filter.subtypes.is_empty()
        {
            filter.type_or_subtype_union = true;
        }
    }

    let permanent_type_defaults = vec![
        CardType::Artifact,
        CardType::Creature,
        CardType::Enchantment,
        CardType::Land,
        CardType::Planeswalker,
        CardType::Battle,
    ];
    let and_segments = split_lexed_slices_on_and(&segment_tokens);
    let and_segment_words_lists: Vec<Vec<String>> = and_segments
        .iter()
        .map(|segment| {
            non_article_parser_word_refs(segment)
                .into_iter()
                .map(ToString::to_string)
                .collect()
        })
        .collect();

    let segment_has_standalone_spell = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some());
        if !contains_spell {
            return false;
        }

        !segment.iter().any(|word| {
            parse_word_choice(word.as_str(), OBJECT_REFERENCE_NOUN_WORDS).is_some()
                || parse_card_type(word).is_some()
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_nonspell_permanent_head = |segment: &[String]| {
        let contains_spell = segment
            .iter()
            .any(|word| parse_word_choice(word, SPELL_OR_SPELLS_WORDS).is_some());
        if contains_spell {
            return false;
        }

        segment.iter().any(|word| {
            parse_word_choice(word, PERMANENT_OR_PERMANENTS_WORDS).is_some()
                || parse_card_type(word).is_some_and(is_permanent_type)
                || parse_subtype_flexible(word).is_some()
        })
    };
    let segment_has_permanent_spell_head = |segment: &[String]| {
        if segment.len() < 2 {
            return false;
        }
        let mut idx = 0usize;
        while idx + 1 < segment.len() {
            let permanent = &segment[idx];
            let spell = &segment[idx + 1];
            if parse_word_choice(permanent, PERMANENT_OR_PERMANENTS_WORDS).is_some()
                && parse_word_choice(spell, SPELL_OR_SPELLS_WORDS).is_some()
            {
                return true;
            }
            idx += 1;
        }
        false
    };
    let has_standalone_spell_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_standalone_spell(segment));
    let has_nonspell_permanent_segment = segment_words_lists
        .iter()
        .any(|segment| segment_has_nonspell_permanent_head(segment));
    let has_split_permanent_spell_segments = and_segment_words_lists.len() > 1
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_permanent_spell_head(segment))
        && and_segment_words_lists
            .iter()
            .any(|segment| segment_has_nonspell_permanent_head(segment));

    if saw_spell && has_standalone_spell_segment && has_nonspell_permanent_segment {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.card_types.clear();
        spell_filter.all_card_types.clear();
        spell_filter.subtypes.clear();
        spell_filter.all_subtypes.clear();
        spell_filter.type_or_subtype_union = false;

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
            && permanent_filter.all_subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent && has_split_permanent_spell_segments {
        let mut spell_filter = filter.clone();
        spell_filter.any_of.clear();
        spell_filter.zone = Some(Zone::Stack);
        spell_filter.has_mana_cost = false;
        if spell_filter.card_types.is_empty()
            && spell_filter.all_card_types.is_empty()
            && spell_filter.subtypes.is_empty()
            && spell_filter.all_subtypes.is_empty()
        {
            spell_filter.card_types = permanent_type_defaults.clone();
        }

        let mut permanent_filter = filter.clone();
        permanent_filter.any_of.clear();
        permanent_filter.zone = Some(Zone::Battlefield);
        permanent_filter.has_mana_cost = false;
        if permanent_filter.card_types.is_empty()
            && permanent_filter.all_card_types.is_empty()
            && permanent_filter.subtypes.is_empty()
            && permanent_filter.all_subtypes.is_empty()
        {
            permanent_filter.card_types = permanent_type_defaults.clone();
        }

        let mut combined_filter = ObjectFilter::default();
        combined_filter.any_of = vec![spell_filter, permanent_filter];
        filter = combined_filter;
    } else if saw_spell && saw_permanent {
        if filter.card_types.is_empty() && filter.all_card_types.is_empty() {
            filter.card_types = permanent_type_defaults.clone();
        }
        filter.zone = Some(Zone::Stack);
    } else if saw_permanent && filter.card_types.is_empty() && filter.all_card_types.is_empty() {
        filter.card_types = permanent_type_defaults.clone();
    }

    if filter.any_of.is_empty() {
        if let Some(zone) = filter.zone {
            if saw_spell && zone != Zone::Stack {
                let is_spell_origin_zone = matches!(
                    zone,
                    Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command
                );
                if !is_spell_origin_zone {
                    return Err(CardTextError::ParseError(
                        "spell targets must be on the stack".to_string(),
                    ));
                }
            }
        } else if saw_spell {
            filter.zone = Some(Zone::Stack);
        } else if saw_permanent || saw_permanent_type || saw_subtype {
            filter.zone = Some(Zone::Battlefield);
        }
    }

    if contains_unqualified_spell_word
        && filter.cast_by.is_some()
        && matches!(
            filter.zone,
            Some(Zone::Hand | Zone::Graveyard | Zone::Exile | Zone::Library | Zone::Command)
        )
    {
        filter.owner = None;
    }

    if target_player.is_some() || target_object.is_some() {
        filter = if targets_only {
            filter.targeting_only(target_player.take(), target_object.take())
        } else {
            filter.targeting(target_player.take(), target_object.take())
        };
        if let Some(count) = target_count {
            filter = filter.with_target_count(count);
        } else if targets_only {
            filter = filter.target_count_exact(1);
        }
    }

    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.is_empty()
        && slice_has(&filter.supertypes, &Supertype::Legendary)
        && slice_has(&filter.subtypes, &or_subtype)
    {
        // The zone, owner, target count, and other trailing qualifiers scope
        // the complete disjunction. Keep them on the outer filter rather than
        // cloning them into the two selector arms; reference consumers (for
        // example a subsequent graveyard-card copy) must be able to observe
        // that the selected object itself is in that shared domain.
        let mut disjunction = filter.clone();
        disjunction
            .supertypes
            .retain(|supertype| *supertype != Supertype::Legendary);
        disjunction
            .subtypes
            .retain(|subtype| *subtype != or_subtype);
        let legendary_branch = ObjectFilter {
            supertypes: vec![Supertype::Legendary],
            ..ObjectFilter::default()
        };
        let subtype_branch = ObjectFilter {
            subtypes: vec![or_subtype],
            ..ObjectFilter::default()
        };
        disjunction.any_of = vec![legendary_branch, subtype_branch];
        filter = disjunction;
    }
    if let Some(or_subtype) = legendary_or_subtype
        && filter.any_of.len() == 2
        && filter.any_of.iter().any(|branch| {
            branch.supertypes.len() == 1
                && branch.supertypes.first() == Some(&Supertype::Legendary)
                && branch.subtypes.is_empty()
        })
        && filter.any_of.iter().any(|branch| {
            branch.subtypes.len() == 1
                && branch.subtypes.first() == Some(&or_subtype)
                && branch.supertypes.is_empty()
        })
        && filter
            .any_of
            .iter()
            .all(|branch| branch.controller.is_none())
    {
        let shared_zone = filter.zone.or_else(|| {
            filter
                .any_of
                .iter()
                .find_map(|branch| branch.zone.filter(|zone| *zone != Zone::Battlefield))
        });
        let shared_owner = filter
            .owner
            .clone()
            .or_else(|| filter.any_of.iter().find_map(|branch| branch.owner.clone()));
        filter.zone = shared_zone;
        filter.owner = shared_owner;
        for branch in &mut filter.any_of {
            branch.zone = None;
            branch.owner = None;
        }
    }

    let owner_or_controller_player = all_words.iter().enumerate().find_map(|(idx, _)| {
        parse_owner_or_controller_disjunction_player(&all_words[idx..], &pronoun_player_filter)
            .map(|(player_filter, _)| player_filter)
    });
    if let Some(player_filter) = owner_or_controller_player
        && filter.any_of.is_empty()
    {
        let mut base = filter.clone();
        base.any_of.clear();
        base.owner = None;
        base.controller = None;

        let mut owner_branch = base.clone();
        owner_branch.owner = Some(player_filter.clone());

        let mut controller_branch = base;
        controller_branch.controller = Some(player_filter);

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = vec![owner_branch, controller_branch];
        filter = disjunction;
    }

    if has_power_or_toughness_clause && saw_spell {
        let mut power_or_toughness_cmp = None;
        for idx in 0..all_words.len() {
            let (_, value_tokens) = match all_words.get(idx..) {
                Some(["power", "or", "toughness", rest @ ..])
                | Some(["toughness", "or", "power", rest @ ..]) => {
                    (crate::filter::PtReference::Effective, rest)
                }
                _ => continue,
            };
            let Some((cmp, _)) =
                parse_filter_comparison_tokens("power", value_tokens, &clause_words)?
            else {
                continue;
            };
            power_or_toughness_cmp = Some(cmp);
            break;
        }
        if let Some(cmp) = power_or_toughness_cmp {
            let mut base = filter.clone();
            base.any_of.clear();
            base.power = None;
            base.toughness = None;

            let mut power_branch = base.clone();
            power_branch.power = Some(cmp.clone());

            let mut toughness_branch = base;
            toughness_branch.toughness = Some(cmp);

            let mut disjunction = ObjectFilter::default();
            disjunction.any_of = vec![power_branch, toughness_branch];
            filter = disjunction;
        }
    }

    // In "creature attacking you or a planeswalker you control", the
    // planeswalker and its controller describe the attack destination, not
    // another candidate object. Apply this cleanup after the ordinary
    // characteristic scan so those later passes cannot reintroduce the
    // destination as a candidate. The position check distinguishes this from
    // "creature or planeswalker attacking you", where both nouns genuinely
    // select candidates.
    if has_attack_destination_planeswalker_clause
        && filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
    {
        filter
            .card_types
            .retain(|card_type| *card_type != CardType::Planeswalker);
        filter
            .all_card_types
            .retain(|card_type| *card_type != CardType::Planeswalker);
        filter.controller = None;
    }

    if exclude_basic_land_cards {
        apply_basic_land_exception(&mut filter);
    }

    if chosen_type_reference.is_some() {
        let has_land_type = filter
            .card_types
            .iter()
            .chain(filter.all_card_types.iter())
            .any(|card_type| *card_type == CardType::Land);
        let has_nonland_type = filter
            .card_types
            .iter()
            .chain(filter.all_card_types.iter())
            .any(|card_type| *card_type != CardType::Land);
        if has_land_type && !has_nonland_type {
            filter.chosen_land_type = true;
            filter.chosen_creature_type = false;
        }
    }

    if parse_word_choice_anywhere(
        &non_article_parser_word_refs(&segment_tokens),
        TYPE_LIST_CONJUNCTION_WORDS,
    )
    .is_some()
        && !filter.card_types.is_empty()
    {
        filter.all_card_types.clear();
    }

    let has_constraints = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.all_subtypes.is_empty()
        || filter.zone.is_some()
        || filter.controller.is_some()
        || filter.owner.is_some()
        || filter.other
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.foretold
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.attacking_alone
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.color_count.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.has_basic_land_type
        || filter.has_nonbasic_land_type
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.power_toughness_relation.is_some()
        || filter.toughness.is_some()
        || filter.total_power_toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.characteristic_relations.is_empty()
        || !filter.any_of.is_empty();

    if !has_constraints {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase (clause: '{}')",
            all_words.join(" ")
        )));
    }

    let has_object_identity = !filter.card_types.is_empty()
        || !filter.all_card_types.is_empty()
        || !filter.supertypes.is_empty()
        || !filter.excluded_supertypes.is_empty()
        || !filter.excluded_card_types.is_empty()
        || !filter.excluded_subtypes.is_empty()
        || !filter.subtypes.is_empty()
        || !filter.all_subtypes.is_empty()
        || filter.zone.is_some()
        || filter.token
        || filter.nontoken
        || filter.face_down.is_some()
        || filter.foretold
        || filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.attacking_alone
        || filter
            .attacking_player_or_planeswalker_controlled_by
            .is_some()
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
        || filter.is_commander
        || filter.noncommander
        || filter.required_colors.is_some()
        || filter.sticker.is_some()
        || !filter.excluded_colors.is_empty()
        || filter.colorless
        || filter.multicolored
        || filter.monocolored
        || filter.all_colors.is_some()
        || filter.exactly_two_colors.is_some()
        || filter.color_count.is_some()
        || filter.historic
        || filter.nonhistoric
        || filter.power.is_some()
        || filter.power_parity.is_some()
        || filter.power_toughness_relation.is_some()
        || filter.toughness.is_some()
        || filter.total_power_toughness.is_some()
        || filter.mana_value.is_some()
        || filter.mana_value_parity.is_some()
        || filter.name.is_some()
        || filter.excluded_name.is_some()
        || filter.source
        || filter.with_counter.is_some()
        || filter.without_counter.is_some()
        || filter.total_counters_parity.is_some()
        || filter.alternative_cast.is_some()
        || !filter.static_abilities.is_empty()
        || !filter.excluded_static_abilities.is_empty()
        || !filter.ability_markers.is_empty()
        || !filter.excluded_ability_markers.is_empty()
        || !filter.no_shared_creature_types_with.is_empty()
        || !filter.characteristic_relations.is_empty()
        || filter.shares_creature_type_with_source
        || filter.chosen_color
        || filter.chosen_creature_type
        || filter.excluded_chosen_creature_type
        || filter.excluded_any_chosen_creature_type
        || filter.colors.is_some()
        || !filter.tagged_constraints.is_empty()
        || filter.targets_player.is_some()
        || filter.targets_object.is_some()
        || !filter.any_of.is_empty();
    if !has_object_identity {
        return Err(CardTextError::ParseError(format!(
            "unsupported target phrase lacking object selector (clause: '{}')",
            all_words.join(" ")
        )));
    }

    preserve_relative_characteristic_list_surface(&mut filter, tokens);
    preserve_branch_scoped_comparison_union(&mut filter, tokens);
    lift_shared_trailing_mana_value_from_type_union(&mut filter, tokens);

    if vote_winners_only {
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::VoteWinners.bind(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }

    if not_on_battlefield && filter.any_of.is_empty() && !matches!(filter.zone, Some(Zone::Stack)) {
        let mut base = filter.clone();
        base.any_of.clear();
        base.zone = None;

        let mut disjunction = ObjectFilter::default();
        disjunction.any_of = [
            Zone::Hand,
            Zone::Library,
            Zone::Graveyard,
            Zone::Exile,
            Zone::Command,
        ]
        .into_iter()
        .map(|zone| {
            let mut branch = base.clone();
            branch.zone = Some(zone);
            branch
        })
        .collect();
        filter = disjunction;
    }

    // Strict mode: detect structural patterns in the input that indicate
    // unconsumed compound content (e.g. "for each card in your hand AND EACH
    // foretold card you own in exile" where the second clause was silently
    // absorbed into the first filter).
    // This exact coordinated stack domain can be partially rewritten by
    // later noun/reference stages (most visibly back to Spell with a mana
    // cost). Reassert only the grammar-proven final domain after every stage
    // has run so public quantified clauses retain both members.
    let final_words = non_article_parser_word_refs(tokens);
    if parse_phrase_choice_anywhere(&final_words, SPELL_AND_ABILITY_PHRASES).is_some() {
        filter.zone = Some(Zone::Stack);
        filter.stack_kind = Some(crate::filter::StackObjectKind::SpellOrAbility);
        filter.has_mana_cost = false;
        filter.set_conjunctive_set_surface(true);
    }

    if strict {
        let input_words = non_article_parser_word_refs(tokens);
        let all_words = input_words.as_slice();

        // "and each" / "and every" signals a compound count source when
        // the word after "each"/"every" introduces a new filter (type word,
        // zone word, etc.) rather than qualifying the current subject
        // (e.g. "and each other creature" is a subject qualifier, but
        // "and each foretold card you own in exile" is a new clause).
        for (idx, _) in input_words.iter().enumerate() {
            if parse_phrase_choice_at_head(&input_words[idx..], STRICT_COMPOUND_COUNT_PREFIXES)
                .is_none()
            {
                continue;
            }
            // A "other than basic land card(s)" exception is stripped before
            // this point, so it never reaches the compound-clause check; guard
            // for it defensively to keep the strict scan stable.
            if parse_phrase_at_head(&all_words[idx..], OTHER_THAN_BASIC_LAND_PREFIX).is_some() {
                continue;
            }
            // "and each other" is typically a subject qualifier, allow it.
            let after_each = input_words.get(idx + 2).copied();
            if after_each.is_some_and(|w| parse_word_choice(w, OTHER_OR_ANOTHER_WORDS).is_some()) {
                continue;
            }
            return Err(CardTextError::ParseError(format!(
                "object filter has unconsumed compound clause '{}' (full input: '{}')",
                input_words[idx..].join(" "),
                input_words.join(" "),
            )));
        }

        // "for each" signals a trailing iteration clause that should have
        // been split out by the caller before passing to the filter parser.
        for (idx, _) in input_words.iter().enumerate() {
            if idx > 0
                && parse_phrase_at_head(&input_words[idx..], STRICT_FOR_EACH_TAIL_PREFIX).is_some()
            {
                return Err(CardTextError::ParseError(format!(
                    "object filter has unconsumed 'for each' clause '{}' (full input: '{}')",
                    input_words[idx..].join(" "),
                    input_words.join(" "),
                )));
            }
        }
    }

    Ok(filter)
}

pub(super) fn try_apply_could_be_targeted_by_that_spell_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        ["that", "spell", "could", "target"].as_slice(),
        ["this", "spell", "could", "target"].as_slice(),
        ["it", "could", "target"].as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;
        filter.could_be_targeted_by = Some(TargetabilityConstraint::by_stack_object(
            ObjectRef::tagged(crate::tag::CompilerReferenceTag::It.bind()),
        ));
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}

pub(super) fn try_apply_shared_creature_type_with_source_clause(
    filter: &mut ObjectFilter,
    all_words: &mut Vec<&str>,
) -> bool {
    for phrase in [
        [
            "that", "share", "creature", "type", "with", "this", "creature",
        ]
        .as_slice(),
        [
            "that", "shares", "creature", "type", "with", "this", "creature",
        ]
        .as_slice(),
        [
            "that",
            "share",
            "creature",
            "type",
            "with",
            "this",
            "permanent",
        ]
        .as_slice(),
        [
            "that",
            "shares",
            "creature",
            "type",
            "with",
            "this",
            "permanent",
        ]
        .as_slice(),
    ] {
        let Some(fact) = parse_phrase_anywhere(all_words, phrase) else {
            continue;
        };
        let idx = fact.span.start;

        filter.shares_creature_type_with_source = true;
        all_words.drain(idx..idx + phrase.len());
        return true;
    }
    false
}
