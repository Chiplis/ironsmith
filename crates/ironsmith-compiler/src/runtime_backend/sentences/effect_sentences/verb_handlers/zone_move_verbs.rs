use super::super::grammar::effects::zone_move_shapes as zone_move_grammar;

fn mana_cost_is_x_only(mana: &[ManaSymbol]) -> bool {
    mana.len() == 1 && matches!(mana.first(), Some(ManaSymbol::X))
}

fn mana_cost_single_generic(mana: &[ManaSymbol]) -> Option<u8> {
    match mana {
        [ManaSymbol::Generic(value)] => Some(*value),
        _ => None,
    }
}

pub(crate) fn parse_move(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use winnow::Parser as _;

    // "all counters from <source> onto/to <destination>"
    // "a counter from <source> onto/to <destination>"
    let (after_prefix, move_all) = if let Some(rest) =
        grammar::strip_lexed_prefix_phrase(tokens, &["all", "counters", "from"])
    {
        (rest, true)
    } else if let Some(rest) = grammar::strip_lexed_prefix_phrase(tokens, &["a", "counter", "from"])
    {
        (rest, false)
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    let split = grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("onto").void())
        .or_else(|| {
            grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("to").void())
        });
    let Some((from_tokens, to_tokens)) = split else {
        return Err(CardTextError::ParseError(format!(
            "missing move destination (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    };

    let from = parse_target_phrase(from_tokens)?;
    let to = parse_target_phrase(to_tokens)?;

    Ok(if move_all {
        EffectAst::subject_verb_move_all_counters(from, to)
    } else {
        EffectAst::subject_verb_move_one_counter(from, to)
    })
}

fn draw_count_with_surface(count: Value, additional: bool) -> Value {
    if additional {
        count.with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
    } else {
        count
    }
}

pub(crate) fn parse_draw(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let head = zone_move_grammar::parse_draw_head_shape(tokens).map_err(|error| match error {
        zone_move_grammar::DrawHeadShapeError::MissingCount => CardTextError::ParseError(format!(
            "missing draw count (clause: '{}')",
            clause_words.join(" ")
        )),
        zone_move_grammar::DrawHeadShapeError::MissingCardKeyword => {
            CardTextError::ParseError("missing card keyword".to_string())
        }
        zone_move_grammar::DrawHeadShapeError::UnsupportedTrailingClause => {
            CardTextError::ParseError(format!(
                "unsupported trailing draw clause (clause: '{}')",
                clause_words.join(" ")
            ))
        }
    })?;
    let mut count = match head.count {
        zone_move_grammar::DrawHeadCountShape::Resolved(value) => value,
        zone_move_grammar::DrawHeadCountShape::CardPrefixed { count_tokens } => {
            parse_draw_card_prefixed_count_value(count_tokens)?.ok_or_else(|| {
                CardTextError::ParseError(format!(
                    "missing draw count (clause: '{}')",
                    clause_words.join(" ")
                ))
            })?
        }
    };
    let tail = head.tail_tokens;
    let player = extract_subject_player(subject).unwrap_or(PlayerAst::Implicit);
    let mut effect = subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::Draw {
            count: draw_count_with_surface(count.clone(), head.additional),
        },
    );

    if !tail.is_empty() && head.parsed_offset.is_none() {
        if let Some(parsed) = parse_draw_for_each_player_condition(&tail, effect.clone())? {
            effect = parsed;
        } else {
            let has_for_each = zone_move_grammar::contains_draw_for_each_shape(tail);
            if has_for_each {
                let dynamic = if let Some(value) = parse_draw_for_each_object_filter_value(&tail)? {
                    value
                } else {
                    parse_dynamic_cost_modifier_value(&tail)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported draw for-each clause (clause: '{}')",
                            crate::runtime_backend::token_word_refs(tokens).join(" ")
                        ))
                    })?
                };
                match count {
                    Value::Fixed(1) => count = dynamic,
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported multiplied draw count (clause: '{}')",
                            crate::runtime_backend::token_word_refs(tokens).join(" ")
                        )));
                    }
                }
                effect = subject_verb_player_resource_effect(
                    SubjectVerbRoleAst::AffectedPlayer,
                    player,
                    SubjectVerbActionAst::Draw {
                        count: draw_count_with_surface(count.clone(), head.additional),
                    },
                );
            } else if let Some(parsed) = parse_draw_trailing_clause(&tail, effect.clone())? {
                effect = parsed;
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing draw clause (clause: '{}')",
                    clause_words.join(" ")
                )));
            }
        }
    }
    Ok(effect)
}

fn parse_draw_for_each_player_condition(
    tokens: &[OwnedLexToken],
    draw_effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    fn bind_loop_player_predicate(predicate: PredicateAst) -> PredicateAst {
        match predicate {
            PredicateAst::And(left, right) => PredicateAst::And(
                Box::new(bind_loop_player_predicate(*left)),
                Box::new(bind_loop_player_predicate(*right)),
            ),
            PredicateAst::Or(left, right) => PredicateAst::Or(
                Box::new(bind_loop_player_predicate(*left)),
                Box::new(bind_loop_player_predicate(*right)),
            ),
            PredicateAst::Not(inner) => {
                PredicateAst::Not(Box::new(bind_loop_player_predicate(*inner)))
            }
            PredicateAst::PlayerControls { player, filter } if player == PlayerAst::That => {
                PredicateAst::PlayerControls {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerHasAtLeast {
                player,
                filter,
                count,
            } if player == PlayerAst::That => PredicateAst::PlayerHasAtLeast {
                player: PlayerAst::Implicit,
                filter,
                count,
            },
            PredicateAst::PlayerControlsExactly {
                player,
                filter,
                count,
            } if player == PlayerAst::That => PredicateAst::PlayerControlsExactly {
                player: PlayerAst::Implicit,
                filter,
                count,
            },
            PredicateAst::PlayerControlsMost { player, filter } if player == PlayerAst::That => {
                PredicateAst::PlayerControlsMost {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerControlsMoreThanEachOtherPlayer { player, filter }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerControlsMoreThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerControlsMoreThanYou { player, filter }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerControlsMoreThanYou {
                    player: PlayerAst::Implicit,
                    filter,
                }
            }
            PredicateAst::PlayerHasLessLifeThanYou { player } if player == PlayerAst::That => {
                PredicateAst::PlayerHasLessLifeThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreLifeThanYou { player } if player == PlayerAst::That => {
                PredicateAst::PlayerHasMoreLifeThanYou {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHasMoreCardsInHandThanYou { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasMoreCardsInHandThanYou { player }
            }
            PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerTappedLandForManaThisTurn { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerTappedLandForManaThisTurn {
                    player: PlayerAst::Implicit,
                }
            }
            PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player }
                if player == PlayerAst::That =>
            {
                PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                    player: PlayerAst::Implicit,
                }
            }
            other => other,
        }
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let Some(shape) = zone_move_grammar::parse_draw_player_loop_shape(tokens) else {
        return Ok(None);
    };
    let inner_tokens = shape.who_tokens;
    let predicate_tail = trim_commas(&inner_tokens[1..]);
    if predicate_tail.is_empty() {
        return Err(CardTextError::ParseError(format!(
            "missing predicate in draw for-each clause (clause: '{}')",
            clause_words.join(" ")
        )));
    }

    let predicate = bind_loop_player_predicate(
        parse_who_player_predicate_lexed(&inner_tokens).ok_or_else(|| {
            CardTextError::ParseError(format!(
                "missing predicate in draw for-each clause (clause: '{}')",
                clause_words.join(" ")
            ))
        })?,
    );

    let mut draw_effect = draw_effect;
    match &mut draw_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::Draw { .. },
        }) if *player == PlayerAst::Implicit => {
            *player = PlayerAst::You;
        }
        _ => {}
    }

    let effects = vec![EffectAst::Conditional {
        predicate,
        if_true: vec![draw_effect],
        if_false: Vec::new(),
    }];
    Ok(Some(if shape.opponents_only {
        EffectAst::ForEachOpponent { effects }
    } else {
        EffectAst::ForEachPlayer { effects }
    }))
}

pub(crate) fn parse_half_rounded_down_draw_count_words(words: &[&str]) -> Option<(Value, usize)> {
    let tokens = crate::runtime_backend::lexer::synthetic_word_tokens(words.iter().copied());
    zone_move_grammar::parse_half_rounded_down_draw_shape(&tokens)
}

pub(crate) fn parse_draw_trailing_clause(
    tokens: &[OwnedLexToken],
    draw_effect: EffectAst,
) -> Result<Option<EffectAst>, CardTextError> {
    let Some(shape) = zone_move_grammar::parse_draw_trailing_shape(tokens) else {
        return Ok(None);
    };
    match shape {
        zone_move_grammar::DrawTrailingShape::Instead => Ok(Some(draw_effect)),
        zone_move_grammar::DrawTrailingShape::Delayed(timing) => {
            let timing = match timing {
                super::super::grammar::effects::ReturnTimingShape::NextEndStep(player) => {
                    DelayedReturnTimingAst::NextEndStep(player)
                }
                super::super::grammar::effects::ReturnTimingShape::NextUpkeep(player) => {
                    DelayedReturnTimingAst::NextUpkeep(player)
                }
                super::super::grammar::effects::ReturnTimingShape::EndOfCombat => {
                    DelayedReturnTimingAst::EndOfCombat
                }
            };
            Ok(Some(wrap_return_with_delayed_timing(
                draw_effect,
                Some(timing),
            )))
        }
        zone_move_grammar::DrawTrailingShape::ThenPut { put_tokens } => {
            let put_effect = parse_put_into_hand(put_tokens, None)?;
            Ok(Some(EffectAst::Sequence {
                effects: vec![draw_effect, put_effect],
            }))
        }
        zone_move_grammar::DrawTrailingShape::If => {
            let predicate = parse_trailing_if_predicate_lexed(tokens).ok_or_else(|| {
                CardTextError::ParseError("missing condition after trailing if clause".to_string())
            })?;
            Ok(Some(EffectAst::Conditional {
                predicate,
                if_true: vec![draw_effect],
                if_false: Vec::new(),
            }))
        }
        zone_move_grammar::DrawTrailingShape::Unless => try_build_unless(
            vec![draw_effect],
            SubjectVerbPrimitiveClause::new(tokens),
            0,
        ),
    }
}

pub(crate) fn parse_draw_card_prefixed_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    if tokens.is_empty() {
        return Ok(None);
    }

    if let Some(value) = parse_draw_for_each_object_filter_value(tokens)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_draw_equal_to_value(tokens)? {
        return Ok(Some(
            value.with_surface_hint(ironsmith_core::ValueSurfaceHint::EqualTo),
        ));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

fn parse_draw_for_each_object_filter_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let Some(filter_tokens) = zone_move_grammar::strip_draw_for_each_prefix(tokens) else {
        return Ok(None);
    };

    if let Some(history_value) = crate::runtime_backend::front_end::grammar::shared_util::value_semantics::parse_turn_history_count_value(&filter_tokens)
    {
        return Ok(Some(history_value.with_surface_hint(
            ironsmith_core::ValueSurfaceHint::ForEach,
        )));
    }

    if let Some(known_value) = parse_draw_for_each_known_count_value(&filter_tokens)? {
        return Ok(Some(
            known_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }

    if let Some(cast_this_turn_value) =
        crate::runtime_backend::grammar::shared_util::value_semantics::parse_spells_cast_this_turn_matching_count_value_lexed(&filter_tokens)
    {
        return Ok(Some(cast_this_turn_value.with_surface_hint(
            ironsmith_core::ValueSurfaceHint::ForEach,
        )));
    }

    if let Some(this_way_value) = parse_draw_for_each_this_way_metric_value(&filter_tokens) {
        return Ok(Some(
            this_way_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }

    if let Some(counter_value) = parse_draw_for_each_counter_reference_value(&filter_tokens) {
        return Ok(Some(
            counter_value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }

    let filter_words = crate::runtime_backend::token_word_refs(&filter_tokens);
    if let Some(player) = crate::runtime_backend::front_end::grammar::shared_util::value_helper_shapes::parse_party_size_player(&filter_words)
    {
        return Ok(Some(
            Value::PartySize(player)
                .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
        ));
    }

    Ok(Some(
        Value::Count(parse_object_filter(&filter_tokens, false)?)
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
    ))
}

fn parse_draw_for_each_known_count_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    Ok(
        match zone_move_grammar::parse_draw_known_count_shape(tokens) {
            Some(zone_move_grammar::DrawKnownCountShape::KickCount) => Some(Value::KickCount),
            Some(zone_move_grammar::DrawKnownCountShape::ColorsAmong { filter_tokens }) => Some(
                Value::ColorsAmong(parse_object_filter(filter_tokens, false)?),
            ),
            Some(zone_move_grammar::DrawKnownCountShape::CreaturesDiedThisTurn) => {
                Some(Value::CreaturesDiedThisTurn)
            }
            Some(zone_move_grammar::DrawKnownCountShape::CreaturesDiedThisTurnControlledByYou) => {
                Some(Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You))
            }
            None => None,
        },
    )
}

fn parse_draw_for_each_this_way_metric_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    zone_move_grammar::parse_draw_this_way_metric_shape(tokens)
}

fn parse_draw_for_each_counter_reference_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    zone_move_grammar::parse_draw_counter_reference_shape(tokens)
}

pub(crate) fn parse_draw_equal_to_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let Some(shape) = zone_move_grammar::parse_draw_equal_shape(tokens) else {
        return Ok(None);
    };
    let words = crate::runtime_backend::token_word_refs(tokens);
    if words
        .windows(2)
        .any(|window| window == ["differently", "named"])
        && let Some(value) = parse_equal_to_number_of_filter_value(tokens)
    {
        return Ok(Some(value));
    }
    if matches!(
        shape,
        zone_move_grammar::DrawEqualShape::GreatestCardsDiscardedThisWay
    ) {
        return Ok(Some(Value::PendingEffectMetric {
            source: ironsmith_core::EffectMetricSource::Outcome,
            metric: ironsmith_core::EffectMetric::GreatestPlayerCount,
        }));
    }

    if let Some(value) = parse_devotion_value_from_add_clause(tokens)? {
        return Ok(Some(value));
    }

    if let zone_move_grammar::DrawEqualShape::StatOfTarget {
        stat,
        target_tokens,
    } = &shape
        && let Ok(target) = parse_target_phrase(target_tokens)
    {
        let spec =
            crate::runtime_backend::references::reference_helpers::choose_spec_for_target(&target);
        let value = match stat {
            zone_move_grammar::DrawEqualStat::Power => Value::PowerOf(Box::new(spec)),
            zone_move_grammar::DrawEqualStat::Toughness => Value::ToughnessOf(Box::new(spec)),
            zone_move_grammar::DrawEqualStat::ManaValue => Value::ManaValueOf(Box::new(spec)),
        };
        return Ok(Some(value));
    }

    // Preserve an authored prior-action metric before the generic
    // equal-to/filter parsers can collapse it to a bare effect result.  The
    // exact producer is bound later, while the typed query retains details
    // such as the counter kind in "stun counters removed this way".
    if matches!(
        shape,
        zone_move_grammar::DrawEqualShape::Fallback {
            references_this_way: true
        }
    ) && let Some(value) = zone_move_grammar::parse_draw_equal_this_way_metric_shape(tokens)
    {
        return Ok(Some(value));
    }

    if let Some(value) = parse_add_mana_equal_amount_value(tokens)
        .or_else(|| parse_equal_to_number_of_opponents_you_have_value(tokens))
        .or_else(|| parse_equal_to_number_of_counters_on_reference_value(tokens))
        .or_else(|| parse_equal_to_aggregate_filter_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_plus_or_minus_fixed_value(tokens))
        .or_else(|| parse_equal_to_number_of_filter_value(tokens))
    {
        return Ok(Some(value));
    }
    if matches!(
        shape,
        zone_move_grammar::DrawEqualShape::Fallback {
            references_this_way: true
        }
    ) {
        return Ok(Some(Value::EventValue(EventValueSpec::Amount)));
    }
    if let Some(value) = parse_dynamic_cost_modifier_value(tokens)? {
        return Ok(Some(value));
    }

    Ok(None)
}

fn counter_unless_payment_total_cost(
    mana: Vec<ManaSymbol>,
    life: Option<Value>,
    additional_generic: Option<Value>,
    mana_multiplier: Option<Value>,
    x_value: Option<Value>,
    display_hint: ironsmith_core::DynamicManaDisplayHint,
) -> crate::cost::TotalCost {
    let mut components = Vec::new();
    let mana_cost = crate::mana::ManaCost::from_symbols(mana);
    if !mana_cost.is_empty()
        || additional_generic.is_some()
        || mana_multiplier.is_some()
        || x_value.is_some()
    {
        if mana_cost.has_x()
            || additional_generic.is_some()
            || mana_multiplier.is_some()
            || x_value.is_some()
        {
            components.push(crate::costs::Cost::dynamic_mana(
                ironsmith_core::DynamicManaCost::new(
                    mana_cost,
                    x_value,
                    additional_generic,
                    mana_multiplier,
                    display_hint,
                ),
            ));
        } else {
            components.push(crate::costs::Cost::mana(mana_cost));
        }
    }
    if let Some(life) = life {
        components.push(crate::costs::Cost::life(life));
    }
    crate::cost::TotalCost::from_costs(components)
}

pub(crate) fn parse_counter(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok() {
        eprintln!(
            "parse_counter entry: {:?}",
            crate::runtime_backend::token_word_refs(tokens)
        );
    }
    if let Some(effect) = parse_counter_unless_source_damage(tokens)? {
        if std::env::var("IRONSMITH_CHOICE_TRACE").is_ok() {
            eprintln!("parse_counter: unless-source-damage claimed");
        }
        return Ok(effect);
    }

    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let target = parse_counter_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::TrailingIf {
            predicate: spec.predicate,
            effects: vec![EffectAst::subject_verb_counter(target)],
        });
    }

    let clause_words = crate::runtime_backend::token_word_refs(tokens);
    let shape =
        zone_move_grammar::parse_counter_clause_shape(tokens).map_err(|error| match error {
            zone_move_grammar::CounterClauseShapeError::MissingPays => {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    clause_words.join(" ")
                ))
            }
        })?;
    let unless_shape = match shape {
        zone_move_grammar::CounterClauseShape::SecondSpellThisTurn { target_tokens } => {
            return Ok(EffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::TargetSpellCastOrderThisTurn(2),
                if_true: vec![EffectAst::subject_verb_counter(TargetAst::Spell(
                    span_from_tokens(&target_tokens),
                ))],
                if_false: Vec::new(),
            });
        }
        zone_move_grammar::CounterClauseShape::MalformedConditional => {
            return Err(CardTextError::ParseError(format!(
                "missing conditional counter target or predicate (clause: '{}')",
                clause_words.join(" ")
            )));
        }
        zone_move_grammar::CounterClauseShape::Plain { target_tokens } => {
            return Ok(EffectAst::subject_verb_counter(
                parse_counter_target_phrase(target_tokens)?,
            ));
        }
        zone_move_grammar::CounterClauseShape::Unless(shape) => shape,
    };
    let target = parse_counter_target_phrase(unless_shape.target_tokens)?;
    let payment_clause_tokens = &unless_shape.normalized_payment_tokens;
    let has_dynamic_payment_tail = unless_shape.has_dynamic_payment_tail;
    match crate::runtime_backend::families::activation_and_restrictions::parse_payment_clause_as_total_cost(&payment_clause_tokens) {
            Ok(Some(cost)) => {
                let should_keep_subject_verb_dynamic_path = has_dynamic_payment_tail
                    && cost.as_one_of().is_none()
                    && cost.dynamic_mana_cost().is_none();
                if !should_keep_subject_verb_dynamic_path {
                    return Ok(EffectAst::subject_verb_counter_unless_pays(target, cost));
                }
            }
            Ok(None) => {
                if !has_dynamic_payment_tail {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported counter-unless payment cost (clause: '{}')",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
            }
            Err(err) => {
                if !has_dynamic_payment_tail {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported counter-unless payment cost (clause: '{}'): {err}",
                        crate::runtime_backend::token_word_refs(tokens).join(" ")
                    )));
                }
            }
        }

    let mut mana = unless_shape.mana.clone();
    let mut life = None;
    let mut additional_generic = None;
    let mut mana_multiplier = None;
    let mut x_value = None;
    let mut dynamic_display_hint = ironsmith_core::DynamicManaDisplayHint::Default;
    if mana.is_empty() {
        // "unless its controller pays mana equal to ..." uses a dynamic generic payment.
        if unless_shape.starts_with_mana_word
            && let Some(value) = parse_equal_to_aggregate_filter_value(unless_shape.payment_tokens)
                .or_else(|| parse_equal_to_number_of_filter_value(unless_shape.payment_tokens))
        {
            additional_generic = Some(value);
            dynamic_display_hint = ironsmith_core::DynamicManaDisplayHint::ManaEqualTo;
        } else if unless_shape.has_x_mana_payment && unless_shape.twice_x_surface {
            mana.push(ManaSymbol::X);
            mana_multiplier = Some(Value::Fixed(2));
        } else {
            return Err(CardTextError::ParseError(format!(
                "missing mana cost (clause: '{}')",
                crate::runtime_backend::token_word_refs(tokens).join(" ")
            )));
        }
    }

    match &unless_shape.tail {
        zone_move_grammar::CounterPaymentTailShape::None => {}
        zone_move_grammar::CounterPaymentTailShape::Life(amount) => {
            life = Some(amount.clone());
        }
        zone_move_grammar::CounterPaymentTailShape::Other {
            tokens: trailing_tokens,
            same_name_graveyard,
            for_each,
        } => {
            let trailing_words = crate::runtime_backend::token_word_refs(trailing_tokens);
            if let Some(value) = parse_counter_unless_additional_generic_value(trailing_tokens)? {
                additional_generic = Some(value);
            } else if *same_name_graveyard {
                if !mana_cost_is_x_only(&mana) {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        clause_words.join(" "),
                        trailing_words.join(" ")
                    )));
                }
                x_value = Some(zone_move_grammar::same_name_graveyard_count_value());
            } else if let Some(value) = parse_value_binding_clause(trailing_tokens) {
                if mana_cost_is_x_only(&mana) {
                    x_value = Some(value);
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        clause_words.join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else if *for_each {
                if let Some(dynamic) = parse_dynamic_cost_modifier_value(trailing_tokens)? {
                    if let Some(multiplier) = mana_cost_single_generic(&mana) {
                        additional_generic =
                            Some(scale_value_multiplier(dynamic, multiplier as i32));
                        mana.clear();
                    } else {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                            clause_words.join(" "),
                            trailing_words.join(" ")
                        )));
                    }
                } else {
                    return Err(CardTextError::ParseError(format!(
                        "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                        clause_words.join(" "),
                        trailing_words.join(" ")
                    )));
                }
            } else {
                return Err(CardTextError::ParseError(format!(
                    "unsupported trailing counter-unless payment clause (clause: '{}', trailing: '{}')",
                    clause_words.join(" "),
                    trailing_words.join(" ")
                )));
            }
        }
    }

    if mana.is_empty()
        && life.is_none()
        && additional_generic.is_none()
        && mana_multiplier.is_none()
        && x_value.is_none()
    {
        return Err(CardTextError::ParseError(format!(
            "missing mana cost (clause: '{}')",
            crate::runtime_backend::token_word_refs(tokens).join(" ")
        )));
    }

    if x_value.is_none()
        && mana_cost_is_x_only(&mana)
        && let Some(where_tokens) = unless_shape.where_tokens
    {
        x_value = parse_value_binding_clause(where_tokens).or_else(|| {
            zone_move_grammar::counter_same_name_graveyard_shape(where_tokens)
                .then(zone_move_grammar::same_name_graveyard_count_value)
        });
    }

    return Ok(EffectAst::subject_verb_counter_unless_pays(
        target,
        counter_unless_payment_total_cost(
            mana,
            life,
            additional_generic,
            mana_multiplier,
            x_value,
            dynamic_display_hint,
        ),
    ));
}

fn parse_counter_unless_source_damage(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    let clause = SubjectVerbPrimitiveClause::new(tokens).trimmed();
    let Some((target_clause, condition_clause)) = clause.split_once_on_word("unless") else {
        return Ok(None);
    };
    let target_clause = target_clause.trimmed();
    let condition_clause = condition_clause.trimmed();
    if target_clause.is_empty() || condition_clause.is_empty() {
        return Ok(None);
    }

    let Some((controller_clause, alternative_clause)) =
        condition_clause.split_once_on_word_any(&["has", "have"])
    else {
        return Ok(None);
    };
    let controller_words = controller_clause.trimmed_word_refs();
    if controller_words.as_slice() != ["its", "controller"] {
        return Ok(None);
    }

    let alternative_clause = alternative_clause.trimmed();
    let Some((source_clause, damage_clause)) =
        alternative_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };
    let source_words = source_clause.trimmed_word_refs();
    if !matches!(
        source_words.as_slice(),
        ["this"] | ["this", "spell"] | ["this", "source"]
    ) {
        return Ok(None);
    }

    let damage_clause = damage_clause.trimmed();
    let Some((amount, used)) = parse_value(damage_clause.tokens()) else {
        return Ok(None);
    };
    let damage_tokens = damage_clause.tokens();
    if damage_tokens.get(used).and_then(OwnedLexToken::as_word) != Some("damage") {
        return Ok(None);
    }
    let target_words =
        SubjectVerbPrimitiveClause::new(&damage_tokens[used + 1..]).trimmed_word_refs();
    if !matches!(
        target_words.as_slice(),
        ["them"] | ["to", "them"] | ["that", "player"] | ["to", "that", "player"]
    ) {
        return Ok(None);
    }

    let target = parse_counter_target_phrase(target_clause.tokens())?;
    let alternative = EffectAst::subject_verb_damage(
        amount,
        TargetAst::Player(
            PlayerFilter::ControllerOf(crate::filter::ObjectRef::Target),
            None,
        ),
    );
    Ok(Some(EffectAst::UnlessAction {
        effects: vec![EffectAst::subject_verb_counter(target)],
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    }))
}

#[cfg(test)]
mod turn_history_draw_tests {
    use super::*;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        let mut tokens = crate::runtime_backend::lexer::lex_line(text, 0).expect("lex");
        for token in &mut tokens {
            token.lowercase_word();
        }
        tokens
    }

    #[test]
    fn additional_draw_surface_survives_into_the_subject_verb_ast() {
        let parsed = parse_draw(&lex("an additional card"), None).expect("additional draw parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::Draw { count },
            ..
        }) = parsed
        else {
            panic!("expected subject-verb draw AST");
        };
        assert_eq!(count.unhinted(), &Value::Fixed(1));
        assert!(count.has_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards));
    }

    #[test]
    fn draw_for_each_prefers_typed_turn_history_over_live_object_filters() {
        let zubera =
            parse_draw_for_each_object_filter_value(&lex("for each Zubera that died this turn"))
                .expect("draw value parse")
                .expect("history value");
        assert!(zubera.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        assert!(
            matches!(
                zubera.unhinted(),
                Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::Died { .. })
            ),
            "{zubera:?}"
        );

        let paradox = parse_draw_for_each_object_filter_value(&lex(
            "for each spell you've cast this turn from anywhere other than your hand",
        ))
        .expect("draw value parse")
        .expect("history value");
        assert!(paradox.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        assert!(
            matches!(
                paradox.unhinted(),
                Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast {
                    from_outside_hand: true,
                    ..
                })
            ),
            "{paradox:?}"
        );
    }
}
