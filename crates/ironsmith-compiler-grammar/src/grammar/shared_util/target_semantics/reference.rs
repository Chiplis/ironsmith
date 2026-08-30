use super::*;

pub fn parse_target_phrase_inner(tokens: &[OwnedLexToken]) -> Result<TargetAst, CardTextError> {
    let mut tokens = tokens;
    while tokens.first().is_some_and(|token| token.is_word("then")) {
        tokens = &tokens[1..];
    }
    if tokens.is_empty() {
        return Err(CardTextError::ParseError(
            "missing target phrase".to_string(),
        ));
    }

    // `each` is a set quantifier rather than part of the object filter. Let
    // the ordinary target-head parser see a following `other` so it can retain
    // the source-exclusion bit (for example, "each other creature").
    if tokens.first().and_then(OwnedLexToken::as_word) == Some("each")
        && tokens.get(1).and_then(OwnedLexToken::as_word) == Some("other")
    {
        return parse_target_phrase_inner(&tokens[1..]);
    }

    if let Some(dynamic) = parse_dynamic_target_count_prefix(tokens) {
        let target = parse_target_phrase_inner(dynamic.target_tokens)?;
        return Ok(TargetAst::WithCountValue(
            Box::new(target),
            dynamic.count,
            dynamic.value,
        ));
    }

    let token_word_view = TokenWordView::new(tokens);
    let token_words = token_word_view.to_word_refs();
    if let Some(kind) = sacrificed_object_kind(&token_words) {
        let _ = kind;
        let span = token_slice_span(tokens);
        return Ok(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            span,
        ));
    }
    if matches_surface(token_words.as_slice(), YOUR_OPPONENTS_TARGET_PATTERN) {
        return Ok(TargetAst::Player(
            PlayerFilter::Opponent,
            token_slice_span(tokens),
        ));
    }
    if crate::word_primitives::parse_sequence_complete(
        &token_words,
        &["any", "target", "other", "than", "that", "permanent"],
    ) {
        let filter =
            ObjectFilter::permanent().not_tagged(crate::tag::CompilerReferenceTag::Damaged.key());
        return Ok(TargetAst::ObjectOrPlayer(
            filter,
            PlayerFilter::Any,
            token_slice_span(tokens),
        ));
    }
    if matches_surface(
        token_words.as_slice(),
        DEFENDING_PLAYER_CHOICE_TARGET_PATTERN,
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported defending player's choice target phrase '{}'",
            token_words.join(" ")
        )));
    }

    // Some non-targeting references use the same union grammar as target
    // phrases. Recognize them before the target-head parser, which correctly
    // rejects them as non-targets but would otherwise hide the typed union.
    let non_target_words = crate::lexer::parser_token_word_refs(tokens);
    if crate::word_primitives::parse_any_sequence_complete(
        &non_target_words,
        &[
            &["a", "permanent", "or", "player"],
            &["permanent", "or", "player"],
        ],
    ) {
        return Ok(TargetAst::ObjectOrPlayer(
            ObjectFilter::permanent(),
            PlayerFilter::Any,
            None,
        ));
    }
    if crate::word_primitives::parse_any_sequence_complete(
        &non_target_words,
        &[
            &["the", "player", "or", "planeswalker", "its", "attacking"],
            &[
                "the",
                "player",
                "or",
                "planeswalker",
                "it",
                "s",
                "attacking",
            ],
        ],
    ) {
        return Ok(TargetAst::AttackedPlayerOrPlaneswalker(None));
    }

    // Recognize an exact `this <permanent type>` source surface before the
    // generic target head consumes `this` as a demonstrative prefix.  Once
    // consumed, only the object noun remains and the phrase would otherwise
    // widen from the source permanent to every matching permanent.
    if let Some(surface) = this_source_surface_for_words(&token_words) {
        let span = token_slice_span(tokens);
        let _ = surface;
        return Ok(TargetAst::Source(span));
    }
    let authored_words = crate::lexer::token_word_refs(tokens);
    if matches_surface(&authored_words, REST_TARGET_PATTERN) {
        return Ok(TargetAst::Tagged(
            crate::tag::CompilerReferenceTag::Rest.key(),
            token_slice_span(tokens),
        ));
    }
    if let Some(surface) = source_reference_surface_for_possessive_words(&authored_words) {
        let span = token_slice_span(tokens);
        let _ = surface;
        return Ok(TargetAst::Source(span));
    }
    // A bare authored proper name in a target slot denotes the source card.
    // Ordinary object phrases are intentionally excluded: even when sentence
    // capitalization uppercases their first word, their lowercase rules
    // qualifiers prevent this surface predicate from claiming them.
    if crate::lexer::is_authored_proper_name_phrase(tokens)
        && parse_object_filter(tokens, false).is_err()
    {
        return Ok(TargetAst::Source(token_slice_span(tokens)));
    }

    let target_head_outcome = leaf::recognize_target_head(tokens);
    if matches!(
        target_head_outcome,
        crate::recognition::ParseOutcome::NoMatch
    ) && let Ok(filter) = parse_object_filter(tokens, false)
        && (filter.has_plural_object_noun_surface()
            || tokens.first().is_some_and(|token| {
                let word = token.parser_text();
                crate::word_primitives::strip_word_suffix(word, "s").is_some()
                    && parse_subtype_flexible(word).is_some()
            }))
    {
        // A plural subtype can be the complete subject of an effect without
        // an article or `target` marker (`Vehicles you control become ...`).
        // The leaf target-head recognizer deliberately owns only structural
        // selector heads, so retain this grammar-proven bare object set here.
        return Ok(TargetAst::Object(filter, None, token_slice_span(tokens)));
    }
    let target_head = match target_head_outcome {
        crate::recognition::ParseOutcome::Match(matched) => matched.value,
        crate::recognition::ParseOutcome::Error(diagnostic) => {
            return Err(diagnostic.into_card_text_error());
        }
        crate::recognition::ParseOutcome::NoMatch => {
            return Err(CardTextError::ParseError(format!(
                "unrecognized target or selection phrase '{}'",
                TokenWordView::new(tokens).join(" ")
            )));
        }
    };
    tokens = target_head.tokens();
    let random_choice = target_head.prefix.random.is_some();
    let span = target_head.prefix.phrase_span;
    let target_count: Option<ChoiceCount> = None;

    let all_words = crate::lexer::token_word_refs(tokens);
    if matches_surface(&all_words, ANY_TARGET_PATTERN) {
        return Ok(TargetAst::AnyTarget(span));
    }
    if matches_surface(&all_words, ANY_OTHER_TARGET_PATTERN) {
        return Ok(TargetAst::AnyOtherTarget(span));
    }
    if let Some(reference) = parse_referenced_target_prefix(tokens) {
        let mut filter = parse_object_filter(reference.object_tokens, reference.other)?;
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::It.key(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        let mut count = ChoiceCount::exactly(reference.count as usize);
        if random_choice {
            count = count.at_random();
        }
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            Some(count),
        ));
    }
    if matches_surface(&all_words, IT_OR_THEM_WITH_PREFIX_PATTERN)
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&all_words[2..])
        && consumed == all_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            target_count,
        ));
    }
    if matches_surface(&all_words, ALL_REFERENCED_WITH_THAT_NAME_PATTERN) {
        let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::ChosenName.key(),
            TaggedOpbjectRelation::SameNameAsTagged,
        );
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, span),
            target_count,
        ));
    }
    if matches_surface(&all_words, TAGGED_OBJECT_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), span),
            target_count,
        ));
    }
    if matches_surface(&all_words, REST_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Rest.key(), span),
            target_count,
        ));
    }

    let remaining_words: Vec<&str> = all_words
        .iter()
        .copied()
        .filter(|word| !is_article(word))
        .collect();
    if let Some(chosen) = parse_chosen_object_target(tokens) {
        let filter_tokens = chosen.filter_tokens;
        let filter_words = crate::lexer::token_word_refs(filter_tokens);
        let mut filter = if matches_surface(&filter_words, CARDS_TARGET_SHORTHAND_PATTERN) {
            ObjectFilter::default()
        } else {
            parse_object_filter(filter_tokens, false)?
        };
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::ChosenObjects.key(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
        return Ok(wrap_target_count(
            TargetAst::Object(filter, None, None),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, EQUIPPED_OBJECT_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Equipped.key(), span),
            target_count,
        ));
    }
    if let Some(enchanted) = parse_enchanted_object_target_kind(&remaining_words) {
        if enchanted == EnchantedObjectTargetKind::Creature {
            let mut filter =
                ObjectFilter::tagged(crate::tag::CompilerReferenceTag::Enchanted.key());
            filter.card_types.push(CardType::Creature);
            return Ok(wrap_target_count(
                TargetAst::Object(filter, None, span),
                target_count,
            ));
        }
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Enchanted.key(), span),
            target_count,
        ));
    }
    if matches_surface(
        &remaining_words,
        CREATURE_TAPPED_FOR_THIS_SPELL_COST_PATTERN,
    ) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::TapCost0.key(), span),
            target_count,
        ));
    }

    let target_count = target_head.prefix.count;
    let idx = target_head.prefix.consumed;
    let other = target_head.prefix.other;
    let explicit_target = target_head.prefix.explicit_target_span.is_some();
    let saw_top_prefix = target_head.prefix.top.is_some();

    let words_all = crate::lexer::token_word_refs(&tokens[idx..]);
    if matches_surface(&words_all, ANY_TARGET_PATTERN) {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }
    if matches_surface(&words_all, ANY_OTHER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }

    let remaining = &tokens[idx..];
    let remaining_words: Vec<&str> = crate::lexer::token_word_refs(remaining)
        .into_iter()
        .filter(|word| !is_article(word))
        .collect();
    let target_span = if explicit_target { span } else { None };

    if remaining_words.is_empty() && explicit_target {
        return Ok(wrap_target_count(
            if other {
                TargetAst::AnyOtherTarget(span)
            } else {
                TargetAst::AnyTarget(span)
            },
            target_count,
        ));
    }
    if other && matches_surface(&remaining_words, TARGET_OR_TARGETS_WORD_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::AnyOtherTarget(span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TARGET_OR_TARGETS_WORD_PATTERN) {
        return Ok(wrap_target_count(TargetAst::AnyTarget(span), target_count));
    }

    let bare_top_library_shorthand = saw_top_prefix
        && !remaining_words
            .iter()
            .any(|word| matches_surface_word(word, LIBRARY_WORD_PATTERN))
        && (matches_surface(&remaining_words, TOP_CARD_TARGET_SHORTHAND_PATTERN)
            || (target_count.is_some()
                && matches_surface(&remaining_words, CARDS_TARGET_SHORTHAND_PATTERN)));
    if bare_top_library_shorthand {
        let mut filter = ObjectFilter::default().in_zone(Zone::Library);
        filter.owner = Some(PlayerFilter::You);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    if crate::word_primitives::parse_sequence_complete(
        &remaining_words,
        &["player", "who", "lost", "life", "this", "turn"],
    ) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::lost_life_this_turn(PlayerFilter::Any),
                target_span,
            ),
            target_count,
        ));
    }

    if let Some(filter) = reference_shapes::parse_hand_advantage_player(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }

    if let Some(filter) = reference_shapes::parse_life_advantage_player(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, PLAYER_ON_YOUR_TEAM_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }
    if let Some(filter) = explicit_player_exclusion(&remaining_words) {
        return Ok(wrap_target_count(
            TargetAst::Player(filter, target_span),
            target_count,
        ));
    }
    if other && matches_surface(&remaining_words, ANY_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                contextual_other_player_filter(PlayerFilter::Any),
                target_span,
            ),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ANY_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Any, target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ENCHANTED_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::TaggedPlayer(crate::tag::CompilerReferenceTag::Enchanted.key()),
                target_span,
            ),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, THAT_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, CHOSEN_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::ChosenPlayer, target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, THAT_OPPONENT_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_opponent(), target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, DEFENDING_PLAYER_EDGE_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::Defending, target_span),
            target_count,
        ));
    }
    let second_word_is_object_head = remaining_words.get(1).is_some_and(|word| {
        let normalized = strip_possessive_suffix(word);
        leaf::parse_leaf_object_reference_head_complete(normalized).is_ok()
    });
    if remaining_words.len() >= 3
        && matches_surface_word(remaining_words[0], THAT_OR_THE_WORD_PATTERN)
        && second_word_is_object_head
        && matches_surface_word(remaining_words[2], CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN)
    {
        let player = tagged_it_owner_or_controller_player_filter(remaining_words[2]);
        return Ok(wrap_target_count(
            // The referenced object may have been targeted earlier, but its
            // controller/owner is an ordinary resolution-time reference. The
            // possessive phrase does not create another target requirement.
            TargetAst::Player(player, None),
            target_count,
        ));
    }
    if remaining_words.len() >= 5
        && matches_surface_word(remaining_words[0], THAT_WORD_PATTERN)
        && second_word_is_object_head
        && matches_surface_word(remaining_words[2], OR_WORD_PATTERN)
        && is_demonstrative_object_head(remaining_words[3])
        && matches_surface_word(remaining_words[4], CONTROLLER_OR_OWNER_PLURAL_WORD_PATTERN)
    {
        let player = tagged_it_owner_or_controller_player_filter(remaining_words[4]);
        return Ok(wrap_target_count(
            TargetAst::Player(player, None),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ITS_OR_THEIR_CONTROLLER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::ControllerOf(crate::filter::ObjectRef::tagged(
                    crate::tag::CompilerReferenceTag::It.key(),
                )),
                None,
            ),
            target_count,
        ));
    }
    if remaining_words.len() >= 2 {
        let object_head = strip_possessive_suffix(remaining_words[0]);
        if matches!(
            remaining_words[1],
            "controller" | "controllers" | "owner" | "owners"
        ) && leaf::parse_leaf_object_reference_head_complete(object_head).is_ok()
        {
            let player = tagged_it_owner_or_controller_player_filter(remaining_words[1]);
            return Ok(wrap_target_count(
                TargetAst::Player(player, None),
                target_count,
            ));
        }
    }
    // "enchanted artifact's controller" — the attachment host's controller,
    // resolved through the source's attachment at runtime.
    if remaining_words.len() == 3
        && matches!(remaining_words[0], "enchanted" | "equipped")
        && matches!(remaining_words[2], "controller" | "owner")
        && leaf::parse_leaf_object_reference_head_complete(strip_possessive_suffix(
            remaining_words[1],
        ))
        .is_ok()
    {
        let object_ref = crate::filter::ObjectRef::tagged(remaining_words[0]);
        let player = if remaining_words[2] == "controller" {
            PlayerFilter::ControllerOf(object_ref)
        } else {
            PlayerFilter::OwnerOf(object_ref)
        };
        return Ok(wrap_target_count(
            TargetAst::Player(player, None),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ITS_OR_THEIR_OWNER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                PlayerFilter::OwnerOf(crate::filter::ObjectRef::tagged(
                    crate::tag::CompilerReferenceTag::It.key(),
                )),
                None,
            ),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, YOU_OR_YOUR_PREFIX_PATTERN) && remaining_words.len() == 1 {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::You, target_span),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, ONE_OF_YOUR_OPPONENTS_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                if other {
                    contextual_other_player_filter(PlayerFilter::Opponent)
                } else {
                    PlayerFilter::Opponent
                },
                target_span,
            ),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, OPPONENT_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(
                if other {
                    contextual_other_player_filter(PlayerFilter::Opponent)
                } else {
                    PlayerFilter::Opponent
                },
                target_span,
            ),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, SPELL_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Spell(target_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TRIGGERING_SPELL_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::Triggering.key(), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TRIGGERING_SPELL_OR_ABILITY_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(
                crate::tag::CompilerReferenceTag::TriggeringSource.key(),
                span,
            ),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, IT_OR_THEM_WITH_PREFIX_PATTERN)
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::It.key());
        filter.with_counter = Some(counter_constraint);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, span),
            target_count,
        ));
    }

    if reference_shapes::is_source_from_your_graveyard(&remaining_words) {
        let mut source_filter = ObjectFilter::source().in_zone(Zone::Graveyard);
        source_filter.owner = Some(PlayerFilter::You);
        if let Some(surface) = source_reference_surface_for_words(&remaining_words)
            .or_else(|| this_source_surface_for_words(&remaining_words))
        {
            source_filter = source_filter.with_source_surface(surface);
        }
        return Ok(wrap_target_count(
            TargetAst::Object(source_filter, target_span, None),
            target_count,
        ));
    }
    if reference_shapes::is_source_from_exile(&remaining_words) {
        let mut source_filter = ObjectFilter::source().in_zone(Zone::Exile);
        if let Some(surface) = source_reference_surface_for_words(&remaining_words)
            .or_else(|| this_source_surface_for_words(&remaining_words))
        {
            source_filter = source_filter.with_source_surface(surface);
        }
        return Ok(wrap_target_count(
            TargetAst::Object(source_filter, target_span, None),
            target_count,
        ));
    }
    if let Some(surface) = source_reference_surface_for_words(&remaining_words)
        .or_else(|| this_source_surface_for_words(&remaining_words))
    {
        let source_span = target_span.or(span);
        let _ = surface;
        return Ok(wrap_target_count(
            TargetAst::Source(source_span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, SOURCE_PT_REFERENCE_PREFIX_PATTERN)
        || matches_surface(&remaining_words, SOURCE_PT_REFERENCE_TARGET_PATTERN)
    {
        let source_span = target_span.or(span);
        return Ok(wrap_target_count(
            TargetAst::Source(source_span),
            target_count,
        ));
    }

    if matches_surface(&remaining_words, IT_INSTEAD_THIS_WAY_PREFIX_PATTERN)
        && remaining_words
            .iter()
            .skip(1)
            .all(|word| matches_surface_word(word, INSTEAD_THIS_WAY_WORD_PATTERN))
    {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, TOKEN_CREATED_THIS_WAY_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, ITSELF_TARGET_PATTERN) {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if matches_surface(&remaining_words, HIM_OR_HER_TARGET_PATTERN) {
        return Ok(wrap_target_count(TargetAst::Source(span), target_count));
    }
    if matches_surface(&remaining_words, THEM_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Tagged(crate::tag::CompilerReferenceTag::It.key(), span),
            target_count,
        ));
    }
    if matches_surface(&remaining_words, THAT_PLAYER_TARGET_PATTERN) {
        return Ok(wrap_target_count(
            TargetAst::Player(PlayerFilter::target_player(), target_span),
            target_count,
        ));
    }

    let attacking_you_or_your_planeswalker = [
        &[
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ][..],
        &[
            "creature",
            "thats",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ][..],
        &[
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ][..],
        &[
            "creature",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ][..],
        &[
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "control",
        ][..],
        &[
            "creature",
            "that",
            "is",
            "attacking",
            "you",
            "or",
            "planeswalker",
            "you",
            "controls",
        ][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&remaining_words, expected).is_some());
    if attacking_you_or_your_planeswalker {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.card_types.push(CardType::Creature);
        filter.attacking = true;
        filter.controller = Some(PlayerFilter::Opponent);
        return Ok(wrap_target_count(
            TargetAst::Object(filter, target_span, None),
            target_count,
        ));
    }

    let opponent_or_planeswalker = [
        &["opponent", "or", "planeswalker"][..],
        &["opponents", "or", "planeswalkers"][..],
        &["planeswalker", "or", "opponent"][..],
        &["planeswalkers", "or", "opponents"][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&remaining_words, expected).is_some());
    if opponent_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let prior_player_or_planeswalker = matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::PriorPlayerOrPlaneswalker)
    );
    if prior_player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(
                PlayerFilter::TargetPlayerOrControllerOfTarget,
                target_span,
            ),
            target_count,
        ));
    }

    let player_or_planeswalker_its_attacking = matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::AttackedPlayerOrPlaneswalker)
    );
    if player_or_planeswalker_its_attacking {
        return Ok(wrap_target_count(
            TargetAst::AttackedPlayerOrPlaneswalker(target_span),
            target_count,
        ));
    }

    let player_or_planeswalker = [
        &["player", "or", "planeswalker"][..],
        &["players", "or", "planeswalkers"][..],
        &["planeswalker", "or", "player"][..],
        &["planeswalkers", "or", "players"][..],
    ]
    .iter()
    .any(|expected| primitives::parse_word_sequence_complete(&remaining_words, expected).is_some());
    if player_or_planeswalker {
        return Ok(wrap_target_count(
            TargetAst::PlayerOrPlaneswalker(PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if let Some(union) = parse_object_or_player_union_target(remaining)
        && let Ok(mut filter) = parse_object_filter(union.object_tokens, other)
    {
        filter.other = other;
        let player_filter = match union.player_kind {
            TrailingPlayerTargetKind::Any => PlayerFilter::Any,
            TrailingPlayerTargetKind::Opponent => PlayerFilter::Opponent,
        };
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, player_filter, target_span),
            target_count,
        ));
    }

    if matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::BattleOrOpponent)
    ) {
        let mut filter = ObjectFilter::default().in_zone(Zone::Battlefield);
        filter.card_types.push(CardType::Battle);
        filter.other = other;
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, PlayerFilter::Opponent, target_span),
            target_count,
        ));
    }

    let creature_or_player = matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::CreatureOrPlayer)
    );
    if creature_or_player {
        let mut filter = ObjectFilter::creature();
        filter.other = other;
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    if matches!(
        parse_target_union_shape(&remaining_words),
        Some(TargetUnionShape::PermanentOrPlayer)
    ) {
        let mut filter = ObjectFilter::permanent();
        filter.other = other;
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, PlayerFilter::Any, target_span),
            target_count,
        ));
    }

    let mixed_object_player_target =
        matches_surface(&remaining_words, MIXED_PLAYER_PLANESWALKER_TOKEN_PATTERN);
    if mixed_object_player_target {
        return Err(CardTextError::ParseError(format!(
            "unsupported creature-token/player/planeswalker target phrase (clause: '{}')",
            remaining_words.join(" ")
        )));
    }

    let controller_set = parse_target_controller_set_suffix(remaining);
    let target_set_same_controller = matches!(
        controller_set.constraint,
        TargetControllerSetConstraint::SameController
    );
    let target_set_different_controllers = matches!(
        controller_set.constraint,
        TargetControllerSetConstraint::DifferentControllers
    );
    let remaining = controller_set.core_tokens.as_slice();
    if target_count.is_none_or(|count| count.is_single())
        && let Some(for_each) = parse_target_for_each_suffix(remaining)
        && let Some((count_value, used_words)) =
            parse_for_each_count_value_words(&for_each.count_words)
        && used_words == for_each.count_words.len()
    {
        let object_tokens = for_each.object_tokens;
        if !object_tokens.is_empty() {
            let mut filter = parse_object_filter(object_tokens, other)?;
            filter.target_set_same_controller = target_set_same_controller;
            filter.target_set_different_controllers = target_set_different_controllers;
            return Ok(TargetAst::WithCountValue(
                Box::new(TargetAst::Object(filter, target_span, None)),
                ChoiceCount::dynamic_x(),
                count_value,
            ));
        }
    }

    let mut filter = parse_object_filter(remaining, other)?;
    // Definite combat-role noun phrases identify the concrete participant in
    // the triggering block relationship. Keep the ordinary role predicate as
    // well, both for structural rendering and as a legality guard.
    if crate::word_primitives::parse_sequence_prefix(&token_words, &["the", "blocking"])
        && filter.blocking
    {
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::Blocking.key(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    } else if crate::word_primitives::parse_sequence_prefix(&token_words, &["the", "attacking"])
        && filter.attacking
    {
        filter = filter.match_tagged(
            crate::tag::CompilerReferenceTag::Blocked.key(),
            TaggedOpbjectRelation::IsTaggedObject,
        );
    }
    apply_target_preparation_facts(
        &mut filter,
        parse_target_preparation_facts(remaining, explicit_target),
    );
    filter.target_set_same_controller = target_set_same_controller;
    filter.target_set_different_controllers = target_set_different_controllers;
    filter.target_set_aggregate_constraint =
        lift_total_mana_value_choice_constraint(remaining, &mut filter).map(Box::new);
    if filter.with_counter.is_none()
        && remaining_words
            .first()
            .is_some_and(|word| matches_surface_word(word, IT_OR_THEM_WORD_PATTERN))
        && remaining_words
            .get(1)
            .is_some_and(|word| matches_surface_word(word, WITH_WORD_PATTERN))
        && let Some((counter_constraint, consumed)) =
            parse_filter_counter_constraint_words(&remaining_words[2..])
        && consumed == remaining_words.len().saturating_sub(2)
    {
        filter.with_counter = Some(counter_constraint);
    }
    let reference_span =
        if let Some(surface) = typed_demonstrative_reference_surface(remaining) {
            filter = filter.match_tagged(
                crate::tag::CompilerReferenceTag::It.key(),
                TaggedOpbjectRelation::IsTaggedObject,
            );
            filter.source_surface = Some(surface);
            let span = token_slice_span(remaining);
            span
        } else if filter.tagged_constraints.iter().any(|constraint| {
            constraint.tag.as_str() == crate::tag::CompilerReferenceTag::It.as_str()
        }) {
            let mut idx = tokens.len();
            let mut found_span = None;
            while idx > 0 {
                idx -= 1;
                if token_matches_surface(&tokens[idx], IT_WORD_PATTERN) {
                    found_span = Some(tokens[idx].span());
                    break;
                }
            }
            found_span
        } else if !explicit_target
            && token_words
                .first()
                .is_some_and(|word| word.eq_ignore_ascii_case("the"))
        {
            // A definite description is a typed object reference even when it
            // carries enough qualifiers to identify one of several earlier
            // target declarations. Keep only its span here; reference resolution
            // owns the semantic binding and replaces it with the stable tag.
            token_slice_span(tokens)
        } else {
            None
        };
    let qualified_any_target_excluding_subtype =
        crate::word_primitives::parse_sequence_prefix(&token_words, &["any", "target", "that"])
            && token_words
                .get(3)
                .is_some_and(|word| matches!(*word, "isnt" | "isn't"))
            && !filter.excluded_subtypes.is_empty();
    if qualified_any_target_excluding_subtype {
        return Ok(wrap_target_count(
            TargetAst::ObjectOrPlayer(filter, PlayerFilter::Any, target_span.or(span)),
            target_count,
        ));
    }

    Ok(wrap_target_count(
        TargetAst::Object(filter, target_span, reference_span),
        target_count,
    ))
}
