use super::*;
use crate::runtime_backend::front_end::grammar::effects::choice_damage_shapes as choice_shapes;

fn is_explicit_target_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    choice_shapes::first_choice_damage_word_is(&clause.word_refs(), "target")
        || parse_choice_count_before_target_prefix(clause.tokens()).is_some()
}

pub(crate) fn parse_sentence_each_opponent_loses_x_and_you_gain_x(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_opponent_drain_sentence_shape(clause.tokens()) else {
        return Ok(None);
    };
    let where_value = parse_value_binding_clause(shape.where_tokens).ok_or_else(|| {
        CardTextError::ParseError(format!(
            "unsupported where-x value in opponent life-drain clause (clause: '{}')",
            clause.text()
        ))
    })?;

    Ok(Some(vec![
        EffectAst::ForEachOpponent {
            effects: vec![EffectAst::subject_verb(
                SubjectVerbRoleAst::AffectedPlayer,
                PlayerAst::Implicit,
                SubjectVerbActionAst::LoseLife {
                    amount: where_value.clone(),
                },
            )],
        },
        EffectAst::subject_verb(
            SubjectVerbRoleAst::AffectedPlayer,
            PlayerAst::You,
            SubjectVerbActionAst::GainLife {
                amount: where_value,
            },
        ),
    ]))
}

pub(crate) fn parse_sentence_relative_opponent_damage_difference(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) =
        choice_shapes::parse_relative_opponent_damage_difference_shape(clause.tokens())
    else {
        return Ok(None);
    };
    let mut base_filter = parse_object_filter(shape.filter_tokens, false)?;
    if base_filter.controller.is_some() || base_filter.owner.is_some() {
        return Ok(None);
    }

    let mut iterated_filter = base_filter.clone();
    iterated_filter.controller = Some(PlayerFilter::IteratedPlayer);
    let mut your_filter = base_filter.clone();
    your_filter.controller = Some(PlayerFilter::You);
    let amount = Value::Add(
        Box::new(Value::Count(iterated_filter)),
        Box::new(Value::Scaled(Box::new(Value::Count(your_filter)), -1)),
    )
    .with_surface_hint(ironsmith_core::ValueSurfaceHint::Difference);

    // The authored source phrase is deliberately validated by the grammar
    // shape but lowered to the ordinary source reference. This keeps named
    // self-references and "this spell/source" reusable without embedding a
    // card name in the AST.
    let _source_surface = shape.source_tokens;
    base_filter.zone = None;
    Ok(Some(vec![EffectAst::ForEachOpponent {
        effects: vec![EffectAst::Conditional {
            predicate: PredicateAst::PlayerControlsMoreThanYou {
                player: PlayerAst::That,
                filter: base_filter,
            },
            if_true: vec![EffectAst::subject_verb_damage_with_source(
                TargetAst::Source(None),
                amount,
                TargetAst::Player(PlayerFilter::IteratedPlayer, None),
            )],
            if_false: Vec::new(),
        }],
    }]))
}

pub(crate) fn parse_sentence_same_name_target_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_same_name_target_fanout_sentence)
}

pub(crate) fn parse_sentence_shared_color_target_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_shared_color_target_fanout_sentence)
}

pub(crate) fn parse_sentence_compound_damage_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_compound_damage_fanout_sentence)
}

pub(crate) fn parse_sentence_serial_target_pt_modifiers(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_serial_target_pt_modifiers_sentence)
}

pub(crate) fn parse_sentence_same_name_gets_fanout(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_same_name_gets_fanout_sentence)
}

pub(crate) fn parse_sentence_delayed_until_next_end_step(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_delayed_until_next_end_step_sentence)
}

pub(crate) fn parse_sentence_destroy_or_exile_all_split(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_destroy_or_exile_all_split_sentence)
}

pub(crate) fn parse_sentence_exile_up_to_one_each_target_type(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    clause.parse_with_lexed(parse_exile_up_to_one_each_target_type_sentence)
}

pub(crate) fn parse_sentence_exile_multi_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    if !choice_shapes::first_choice_damage_word_is(&clause.word_refs(), "exile")
        || choice_shapes::has_unless_shape(&clause.word_refs())
    {
        return Ok(None);
    }

    let Some(and_idx) = clause.find_token_word_where("and", |idx, tail_clause| {
        idx > 0 && !tail_clause.is_empty() && is_explicit_target_clause(tail_clause)
    }) else {
        return Ok(None);
    };

    let first_clause = clause.between(1, and_idx).trimmed();
    let second_clause = clause.from(and_idx + 1).trimmed();
    if first_clause.is_empty() || second_clause.is_empty() {
        return Ok(None);
    }

    let first_words = first_clause.word_refs();
    let first_is_explicit_target = is_explicit_target_clause(first_clause);
    let second_is_explicit_target = is_explicit_target_clause(second_clause);

    let mut first_target = if !first_is_explicit_target
        && choice_shapes::is_likely_named_or_source_reference_shape(&first_words)
    {
        if let Some(surface) = crate::runtime_backend::util::source_reference_surface_for_words(
            &first_words,
        )
        .or_else(|| crate::runtime_backend::util::this_source_surface_for_words(&first_words))
        {
            crate::runtime_backend::util::record_source_reference_surface(
                first_clause.span(),
                surface,
            );
        }
        TargetAst::Source(first_clause.span())
    } else {
        match parse_target_phrase(first_clause.tokens()) {
            Ok(target) => target,
            Err(err) => return Err(err),
        }
    };
    let mut second_target = parse_target_phrase(second_clause.tokens())?;

    if first_is_explicit_target
        && second_is_explicit_target
        && let (Some((mut first_filter, first_count)), Some((mut second_filter, second_count))) = (
            object_target_with_count(&first_target),
            object_target_with_count(&second_target),
        )
        && first_filter.zone == Some(Zone::Graveyard)
        && second_filter.zone == Some(Zone::Graveyard)
    {
        if first_filter.controller.is_none() {
            first_filter.controller = Some(PlayerFilter::Any);
        }
        if second_filter.controller.is_none() {
            second_filter.controller = Some(PlayerFilter::Any);
        }
        let tag = helper_tag_for_tokens(clause.tokens(), "exiled");
        return Ok(Some(vec![
            EffectAst::ChooseObjects {
                filter: first_filter,
                count: first_count,
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::ChooseObjects {
                filter: second_filter,
                count: second_count,
                count_value: None,
                player: PlayerAst::You,
                tag: tag.clone(),
            },
            EffectAst::subject_verb_exile(TargetAst::Tagged(tag, None), false),
        ]));
    }

    apply_exile_subject_hand_owner_context(&mut first_target, None);
    apply_exile_subject_hand_owner_context(&mut second_target, None);
    Ok(Some(vec![EffectAst::Coordinated {
        effects: vec![
            EffectAst::subject_verb_exile(first_target, false),
            EffectAst::subject_verb_exile(second_target, false),
        ],
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub(crate) fn split_destroy_target_segments(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Vec<SubjectVerbPrimitiveClause<'_>> {
    let mut segments = Vec::new();
    for segment_clause in clause.trimmed_and_comma_segments() {
        let split_starts = choice_shapes::up_to_one_target_word_starts(&segment_clause.word_refs());

        if split_starts.len() <= 1 {
            segments.push(segment_clause);
            continue;
        }

        for (idx, start) in split_starts.iter().enumerate() {
            let end = split_starts
                .get(idx + 1)
                .copied()
                .unwrap_or(segment_clause.len());
            let segment = segment_clause.between(*start, end).trimmed();
            if !segment.is_empty() {
                segments.push(segment);
            }
        }
    }

    segments
}

pub(crate) fn parse_sentence_destroy_multi_target(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_destroy_multi_target_shape(clause.tokens()) else {
        return Ok(None);
    };

    let target_clause = clause.from(shape.target_start_word).trimmed();
    if target_clause.is_empty() {
        return Ok(None);
    }
    if !shape.repeated_target_words
        && !shape.has_followup_tail
        && let Ok(target) = parse_target_phrase(target_clause.tokens())
        && let Some((filter, _)) = object_target_with_count(&target)
        && (filter.type_or_subtype_union
            || filter.card_types.len() > 1
            || filter.subtypes.len() > 1
            || filter.any_of.len() > 1)
    {
        return Ok(Some(vec![EffectAst::subject_verb_destroy(target)]));
    }

    let segments = split_destroy_target_segments(target_clause);
    if segments.len() < 2 {
        return Ok(None);
    }

    let mut effects = Vec::new();
    for segment_clause in segments {
        let segment_words = segment_clause.word_refs();
        if choice_shapes::has_choice_damage_condition_boundary(&segment_words) {
            return Ok(None);
        }
        let is_explicit_target =
            choice_shapes::first_choice_damage_word_is(&segment_words, "target")
                || parse_choice_count_before_target_prefix(segment_clause.tokens()).is_some();
        if !is_explicit_target
            && !choice_shapes::is_likely_named_or_source_reference_shape(&segment_words)
        {
            return Ok(None);
        }
        if let Some(choice) =
            crate::runtime_backend::front_end::grammar::choices::parse_possessive_object_choice_tokens(
                segment_clause.tokens(),
            )
            && choice.actor
                == crate::runtime_backend::front_end::grammar::choices::PossessiveObjectChoiceActor::Opponent
        {
            let target = parse_target_phrase(&choice.object_tokens)?;
            effects.push(EffectAst::Sequence {
                effects: vec![
                    EffectAst::subject_verb_explicit_target_only_for_chooser(
                        target,
                        PlayerAst::Opponent,
                    ),
                    EffectAst::subject_verb_destroy(TargetAst::Tagged(
                        TagKey::from(IT_TAG),
                        segment_clause.span(),
                    )),
                ],
            });
            continue;
        }
        let target = match parse_target_phrase(segment_clause.tokens()) {
            Ok(target) => target,
            Err(_)
                if !is_explicit_target
                    && choice_shapes::is_likely_named_or_source_reference_shape(&segment_words) =>
            {
                TargetAst::Source(segment_clause.span())
            }
            Err(err) => return Err(err),
        };
        effects.push(EffectAst::subject_verb_destroy(target));
    }

    if effects.len() < 2 {
        return Ok(None);
    }
    Ok(Some(vec![EffectAst::Coordinated {
        effects,
        leading_duration: false,
        result_conjunction: false,
    }]))
}

pub(crate) fn parse_sentence_reveal_selected_cards_in_your_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_reveal_selected_hand_shape(clause.tokens()) else {
        return Ok(None);
    };
    let clause_text = clause.text();
    let clause_words = clause.word_refs();
    if clause_words.first() != Some(&"reveal") {
        return Ok(None);
    }
    if clause_words.iter().any(|word| {
        matches!(
            *word,
            "then" | "if" | "unless" | "where" | "when" | "whenever"
        )
    }) {
        return Ok(None);
    }

    let mut descriptor_clause = SubjectVerbPrimitiveClause::new(shape.descriptor_tokens).trimmed();
    if descriptor_clause.is_empty() {
        return Ok(None);
    }

    let mut count = ChoiceCount::exactly(1);
    if let Some((parsed_count, used)) =
        crate::runtime_backend::util::parse_choice_count_token_prefix_consumed(
            descriptor_clause.tokens(),
        )
    {
        count = if parsed_count.dynamic_x {
            ChoiceCount::any_number()
        } else {
            parsed_count
        };
        descriptor_clause = descriptor_clause.from(used).trimmed();
        if choice_shapes::first_choice_damage_word_is(&descriptor_clause.word_refs(), "of") {
            descriptor_clause = descriptor_clause.from(1).trimmed();
        }
    } else if descriptor_clause
        .first_word()
        .is_some_and(choice_shapes::is_reveal_article_word)
    {
        descriptor_clause = descriptor_clause.from(1).trimmed();
    } else if choice_shapes::has_all_or_each_at(&descriptor_clause.word_refs(), 0) {
        return Ok(None);
    }

    if descriptor_clause.is_empty() {
        return Ok(None);
    }

    let mut filter = match parse_object_filter(descriptor_clause.tokens(), false) {
        Ok(filter) => filter,
        Err(_) => {
            let descriptor_words = descriptor_clause.word_refs();
            let mut filter = ObjectFilter::default();
            let mut idx = 0usize;
            if let Some(color) = descriptor_words.get(idx).and_then(|word| parse_color(word)) {
                filter.colors = Some(color.into());
                idx += 1;
            }
            if !choice_shapes::is_card_noun_at(&descriptor_words, idx) {
                return Err(CardTextError::ParseError(format!(
                    "unsupported reveal-hand clause (clause: '{}')",
                    clause_text
                )));
            }
            filter
        }
    };
    filter.zone = Some(Zone::Hand);
    filter.owner = Some(PlayerFilter::You);

    let tag = helper_tag_for_tokens(clause.tokens(), "revealed");
    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count,
            count_value: None,
            player: PlayerAst::You,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_reveal_tagged(tag),
    ]))
}

pub(crate) fn parse_sentence_target_player_reveals_random_card_from_hand(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_random_hand_reveal_shape(clause.tokens()) else {
        return Ok(None);
    };
    let subject_clause = SubjectVerbPrimitiveClause::new(shape.subject_tokens);
    if subject_clause.is_empty() {
        return Ok(None);
    }

    let subject_tokens = subject_clause.trim();
    let SubjectAst::Player(player) = parse_subject(&subject_tokens) else {
        return Ok(None);
    };
    if !matches!(
        player,
        PlayerAst::You
            | PlayerAst::Target
            | PlayerAst::TargetOpponent
            | PlayerAst::Opponent
            | PlayerAst::That
    ) {
        return Ok(None);
    }

    let descriptor_clause = SubjectVerbPrimitiveClause::new(shape.descriptor_tokens);
    if descriptor_clause.is_empty()
        || !choice_shapes::is_random_card_descriptor_shape(&descriptor_clause.word_refs())
    {
        return Ok(None);
    }

    let hand_clause = SubjectVerbPrimitiveClause::new(shape.hand_tokens);
    if !is_hand_reference_clause(hand_clause) {
        return Ok(None);
    }

    let filter = ObjectFilter {
        zone: Some(Zone::Hand),
        owner: Some(match player {
            PlayerAst::You => PlayerFilter::You,
            PlayerAst::Target => PlayerFilter::target_player(),
            PlayerAst::TargetOpponent => PlayerFilter::target_opponent(),
            PlayerAst::Opponent => PlayerFilter::Opponent,
            PlayerAst::That => PlayerFilter::IteratedPlayer,
            _ => return Ok(None),
        }),
        ..ObjectFilter::default()
    };
    let tag = helper_tag_for_tokens(clause.tokens(), "revealed");

    Ok(Some(vec![
        EffectAst::ChooseObjects {
            filter,
            count: ChoiceCount::exactly(1).at_random(),
            count_value: None,
            player,
            tag: tag.clone(),
        },
        EffectAst::subject_verb_reveal_tagged(tag),
    ]))
}

fn is_hand_reference_clause(clause: SubjectVerbPrimitiveClause<'_>) -> bool {
    choice_shapes::is_hand_reference_shape(&clause.word_refs())
}

pub(crate) fn object_target_with_count(target: &TargetAst) -> Option<(ObjectFilter, ChoiceCount)> {
    match target {
        TargetAst::Object(filter, _, _) => Some((filter.clone(), ChoiceCount::exactly(1))),
        TargetAst::WithCount(inner, count) => match inner.as_ref() {
            TargetAst::Object(filter, _, _) => Some((filter.clone(), count.clone())),
            _ => None,
        },
        _ => None,
    }
}

pub(crate) fn parse_sentence_damage_unless_controller_has_source_deal_damage(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_damage_unless_shape(clause.tokens()) else {
        return Ok(None);
    };
    let before_clause = SubjectVerbPrimitiveClause::new(shape.damage_tokens).trimmed();
    if before_clause.is_empty() {
        return Ok(None);
    }
    let effects = parse_effect_chain(before_clause.tokens())?;
    if effects.len() != 1 {
        return Ok(None);
    }
    let Some(main_effect) = effects.first() else {
        return Ok(None);
    };
    let main_target = match main_effect {
        EffectAst::SubjectVerb(subject_verb) => match &subject_verb.action {
            SubjectVerbActionAst::DealDamage { target, .. }
            | SubjectVerbActionAst::Destroy { target, .. } => target,
            _ => return Ok(None),
        },
        _ => return Ok(None),
    };
    if !matches!(
        main_target,
        TargetAst::Object(_, _, _) | TargetAst::WithCount(_, _)
    ) {
        return Ok(None);
    }

    let after_unless_clause = SubjectVerbPrimitiveClause::new(shape.condition_tokens).trimmed();
    let has_controller_clause =
        choice_shapes::is_that_controller_has_shape(&after_unless_clause.word_refs());
    if !has_controller_clause {
        return Ok(None);
    }
    let Some((_controller_clause, alt_clause)) =
        after_unless_clause.split_once_on_word_any(&["has", "have"])
    else {
        return Ok(None);
    };
    if alt_clause.is_empty() {
        return Ok(None);
    }

    let Some((_before_deal, deal_tail_clause)) =
        alt_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };
    let deal_tail = deal_tail_clause.tokens();
    let deal_words = deal_tail_clause.word_refs();
    let alt_amount = if deal_words.starts_with(&["damage", "to", "them", "equal", "to"]) {
        let amount_tokens = deal_tail.get(5..).unwrap_or_default();
        let Some((amount, used)) = parse_value(amount_tokens) else {
            return Ok(None);
        };
        if used != amount_tokens.len() {
            return Ok(None);
        }
        amount.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo)
    } else {
        let Some((amount, used)) = parse_value(deal_tail) else {
            return Ok(None);
        };
        if !deal_tail
            .get(used)
            .and_then(|token| token.as_word())
            .is_some_and(choice_shapes::is_damage_word)
        {
            return Ok(None);
        }

        let mut alt_target_clause = deal_tail_clause.from(used + 1).trimmed();
        if choice_shapes::has_leading_to_shape(&alt_target_clause.word_refs()) {
            alt_target_clause = alt_target_clause.from(1).trimmed();
        }
        if choice_shapes::parse_alternate_damage_target_shape(&alt_target_clause.word_refs())
            .is_none()
        {
            return Ok(None);
        }
        amount
    };

    let alternative = EffectAst::subject_verb_damage(
        alt_amount,
        TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    );
    let unless = EffectAst::UnlessAction {
        effects,
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    };
    Ok(Some(vec![unless]))
}

pub(crate) fn parse_sentence_damage_to_that_player_unless_enchanted_attacked(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    let Some(shape) = choice_shapes::parse_enchanted_attacked_damage_shape(clause.tokens()) else {
        return Ok(None);
    };
    let before_clause = SubjectVerbPrimitiveClause::new(shape.damage_tokens).trimmed();
    if before_clause.is_empty() {
        return Ok(None);
    }

    let Some((subject_clause, damage_clause)) =
        before_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };

    if !choice_shapes::is_choice_damage_source_subject_shape(&subject_clause.word_refs()) {
        return Ok(None);
    }
    let damage_tokens = damage_clause.tokens();
    let Some((amount, used)) = parse_value(damage_tokens) else {
        return Ok(None);
    };
    if !damage_tokens
        .get(used)
        .and_then(|token| token.as_word())
        .is_some_and(choice_shapes::is_damage_word)
    {
        return Ok(None);
    }

    let mut target_clause = damage_clause.from(used + 1).trimmed();
    if choice_shapes::first_choice_damage_word_is(&target_clause.word_refs(), "to") {
        target_clause = target_clause.from(1).trimmed();
    }
    if !choice_shapes::is_that_player_target_shape(&target_clause.word_refs()) {
        return Ok(None);
    }

    Ok(Some(vec![EffectAst::TrailingUnless {
        predicate: PredicateAst::EnchantedPermanentAttackedThisTurn,
        effects: vec![EffectAst::subject_verb_damage(
            amount,
            TargetAst::Player(PlayerFilter::IteratedPlayer, None),
        )],
    }]))
}

pub(crate) fn parse_sentence_unless_pays(
    clause: SubjectVerbPrimitiveClause<'_>,
) -> Result<Option<Vec<EffectAst>>, CardTextError> {
    // This causative alternative is an action choice, not a payment. Keep it
    // ahead of the broad unless parser even when a caller reaches this rule
    // through a generic conditional-dispatch path.
    if let Some(effects) = parse_sentence_damage_unless_controller_has_source_deal_damage(clause)? {
        return Ok(Some(effects));
    }
    let Some(shape) = choice_shapes::parse_unless_sentence_shape(clause.tokens()) else {
        return Ok(None);
    };
    let unless_idx = shape.unless_token;

    if unless_idx == 0 {
        let Some((unless_clause, effect_clause)) = clause.split_once_on_comma() else {
            return Ok(None);
        };
        if effect_clause.is_empty() {
            return Ok(None);
        }

        let effects = parse_effect_chain(effect_clause.tokens())?;
        if effects.is_empty() {
            return Ok(None);
        }

        if let Some(unless_effect) = try_build_unless(effects, unless_clause, 0)? {
            return Ok(Some(vec![unless_effect]));
        }
        return Ok(None);
    }

    let before_unless_clause = SubjectVerbPrimitiveClause::new(shape.action_tokens);
    let before_words = before_unless_clause.word_refs();

    if choice_shapes::first_choice_damage_word_is(&before_words, "counter") {
        return Ok(None);
    }
    if choice_shapes::is_create_token_sacrifice_counter_shape(&before_unless_clause.word_refs()) {
        return Ok(None);
    }

    // In `A, then B unless you pay C`, only the final action B is replaced
    // by the payment. Parsing the entire prefix as the UnlessPays body both
    // weakens the temporal boundary and lets a prefix-tolerant parser claim A
    // while silently dropping B. Split only on the grammar-proven comma/then
    // boundary, retain every earlier action, and wrap the final action in the
    // payment choice.
    let comma_then_segments =
        super::super::lex_chain_helpers::split_segments_on_comma_then_lexed(vec![
            shape.action_tokens,
        ]);
    if comma_then_segments.len() > 1 {
        let (last, leading) = comma_then_segments
            .split_last()
            .expect("comma/then split has at least two segments");
        let mut effects = Vec::new();
        for segment in leading {
            effects.extend(parse_effect_chain(*segment)?);
        }
        let final_effects = parse_effect_chain(*last)?;
        if effects.is_empty() || final_effects.is_empty() {
            return Ok(None);
        }
        let Some(unless_effect) = try_build_unless(final_effects, clause, unless_idx)? else {
            return Ok(None);
        };
        effects.push(unless_effect);
        return Ok(Some(effects));
    }

    let sentence_words = clause.word_refs();
    if let Some(special) =
        choice_shapes::parse_each_opponent_return_unless_draw_shape(&sentence_words)
    {
        let Some(target_clause) = clause
            .after_words(special.target_start_word)
            .and_then(|tail| {
                tail.before_word(
                    special
                        .target_end_word
                        .saturating_sub(special.target_start_word),
                )
            })
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        let target = parse_target_phrase(target_clause.tokens())?;
        return Ok(Some(vec![EffectAst::ForEachOpponent {
            effects: vec![
                EffectAst::subject_verb_target_only(target),
                EffectAst::UnlessAction {
                    effects: vec![EffectAst::subject_verb_return_to_hand(
                        TargetAst::Tagged(TagKey::from(IT_TAG), None),
                        false,
                    )],
                    alternative: vec![EffectAst::subject_verb(
                        SubjectVerbRoleAst::AffectedPlayer,
                        PlayerAst::You,
                        SubjectVerbActionAst::Draw {
                            count: Value::Fixed(1),
                        },
                    )],
                    player: PlayerAst::ItsController,
                },
            ],
        }]));
    }

    let each_prefix = choice_shapes::parse_choice_damage_scope(&before_unless_clause.word_refs());
    if let Some(prefix_kind) = each_prefix {
        let inner_clause = before_unless_clause
            .after_words(2)
            .unwrap_or_else(|| before_unless_clause.from(2));
        if let Ok(inner_effects) = parse_effect_chain(inner_clause.tokens()) {
            if !inner_effects.is_empty() {
                if let Some(unless_effect) = try_build_unless(inner_effects, clause, unless_idx)? {
                    let wrapper = match prefix_kind {
                        choice_shapes::ChoiceDamageScope::Opponent => EffectAst::ForEachOpponent {
                            effects: vec![unless_effect],
                        },
                        choice_shapes::ChoiceDamageScope::Player => EffectAst::ForEachPlayer {
                            effects: vec![unless_effect],
                        },
                    };
                    return Ok(Some(vec![wrapper]));
                }
            }
        }
        return Ok(None);
    }

    let effect_clause = before_unless_clause;
    if let Some((timing_start_word, _timing_end_word, step, player)) =
        delayed_next_step_marker(effect_clause)
    {
        let Some(delayed_effect_clause) = effect_clause
            .before_word(timing_start_word)
            .map(SubjectVerbPrimitiveClause::trimmed)
        else {
            return Ok(None);
        };
        if delayed_effect_clause.is_empty() {
            return Ok(None);
        }
        let delayed_effects = parse_effect_chain(delayed_effect_clause.tokens())?;
        if delayed_effects.is_empty() {
            return Ok(None);
        }
        if let Some(unless_effect) = try_build_unless(delayed_effects, clause, unless_idx)? {
            return Ok(Some(vec![wrap_delayed_next_step_unless_pays(
                step,
                player,
                vec![unless_effect],
            )]));
        }
    }

    let effects = parse_effect_chain(effect_clause.tokens())?;
    if effects.is_empty() {
        return Ok(None);
    }

    if let Some(unless_effect) = try_build_unless(effects, clause, unless_idx)? {
        return Ok(Some(vec![unless_effect]));
    }
    Ok(None)
}

#[cfg(test)]
mod opponent_choice_target_tests {
    use super::*;
    use crate::runtime_backend::front_end::lexer::lex_line;

    #[test]
    fn multi_target_destroy_keeps_opponent_chooser_on_second_target() {
        let tokens = lex_line(
            "Destroy target nonbasic land you don't control and target nonbasic land of an opponent's choice you don't control.",
            0,
        )
        .expect("destroy pair should lex");
        let parsed = parse_sentence_destroy_multi_target(SubjectVerbPrimitiveClause::new(&tokens))
            .expect("destroy pair should parse")
            .expect("multi-target destroy rule should claim the sentence");
        let [EffectAst::Coordinated { effects, .. }] = parsed.as_slice() else {
            panic!("expected one coordinated destroy pair: {parsed:#?}");
        };
        let [_, EffectAst::Sequence { effects: chosen }] = effects.as_slice() else {
            panic!("the second destroy must retain its delegated choice: {effects:#?}");
        };
        let [
            EffectAst::SubjectVerb(target_only),
            EffectAst::SubjectVerb(destroy),
        ] = chosen.as_slice()
        else {
            panic!("expected target declaration followed by destroy: {chosen:#?}");
        };
        assert_eq!(target_only.subject.role, SubjectVerbRoleAst::Chooser);
        assert_eq!(target_only.subject.player, PlayerAst::Opponent);
        assert!(matches!(
            target_only.action,
            SubjectVerbActionAst::TargetOnly {
                explicit_declaration: true,
                ..
            }
        ));
        assert!(matches!(
            destroy.action,
            SubjectVerbActionAst::Destroy {
                target: TargetAst::Tagged(_, _),
                ..
            }
        ));
    }
}
