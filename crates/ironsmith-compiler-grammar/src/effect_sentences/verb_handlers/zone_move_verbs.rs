use crate::cards::builders::ConditionalEffectAst;
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

pub fn parse_move(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
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
    } else if let Some(effect) = parse_move_counted_counters(tokens)? {
        return Ok(effect);
    } else {
        return Err(CardTextError::ParseError(format!(
            "unsupported move clause (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    };

    let split = grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("onto").void())
        .or_else(|| {
            grammar::split_lexed_once_on_separator(after_prefix, || grammar::kw("to").void())
        });
    let Some((from_tokens, to_tokens)) = split else {
        return Err(CardTextError::ParseError(format!(
            "missing move destination (clause: '{}')",
            crate::lexer::token_word_refs(tokens).join(" ")
        )));
    };

    let from = parse_target_phrase(from_tokens)?;
    let to = parse_counter_move_destination(to_tokens, &from)?;

    Ok(if move_all {
        EffectAst::subject_verb_move_all_counters(from, to)
    } else {
        EffectAst::subject_verb_move_one_counter(from, to)
    })
}

/// "<count> <kind> counters from <source> onto <destination>" (Blaster,
/// Morale Booster: "Move X +1/+1 counters from Blaster onto another target
/// artifact"). Only a named counter kind with an explicit count is read here.
fn parse_move_counted_counters(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use super::super::grammar::primitives as grammar;
    use winnow::Parser as _;

    let (count, used) = if grammar::strip_lexed_prefix_phrase(tokens, &["any", "number", "of"]).is_some() {
        (ironsmith_core::effect::CounterMoveAmount::AnyNumber, 3)
    } else if tokens.first().is_some_and(|token| token.is_word("all")) {
        (ironsmith_core::effect::CounterMoveAmount::All, 1)
    } else if let Some((count, used)) = crate::util::parse_value(tokens) {
        (ironsmith_core::effect::CounterMoveAmount::Exact(count), used)
    } else {
        return Ok(None);
    };
    if used == 0 || used >= tokens.len() {
        return Ok(None);
    }
    let rest = &tokens[used..];
    let Some(noun_idx) = rest
        .iter()
        .position(|token| token.is_any_word(&["counter", "counters"]))
    else {
        return Ok(None);
    };
    if noun_idx == 0 || !rest.get(noun_idx + 1).is_some_and(|token| token.is_word("from")) {
        return Ok(None);
    }
    let Some(counter_type) = crate::util::parse_counter_type_from_tokens(&rest[..noun_idx]) else {
        return Ok(None);
    };
    let after_from = &rest[noun_idx + 2..];
    let Some((from_tokens, to_tokens)) =
        grammar::split_lexed_once_on_separator(after_from, || grammar::kw("onto").void())
    else {
        return Ok(None);
    };
    if from_tokens.is_empty() || to_tokens.is_empty() {
        return Ok(None);
    }
    // A plural donor phrase is the whole set. Choosing the amount for each
    // donor must not accidentally turn it into a choice of one permanent.
    let plural_donors = !from_tokens.iter().any(|token| token.is_word("target"))
        && from_tokens.iter().any(|token| token.is_any_word(&["creatures", "permanents", "artifacts", "lands"]));
    if plural_donors {
        if matches!(count, ironsmith_core::effect::CounterMoveAmount::Exact(_)) {
            return Err(CardTextError::ParseError("a counted transfer from several donors needs a total distribution".into()));
        }
        let filter_tokens = grammar::strip_lexed_prefix_phrase(from_tokens, &["all"])
            .unwrap_or(from_tokens);
        let filter = crate::object_filters::parse_object_filter(filter_tokens, false)?;
        let from = TargetAst::Object(filter.clone(), None, None);
        let to = parse_counter_move_destination(to_tokens, &from)?;
        return Ok(Some(EffectAst::subject_verb_move_counters_from_all(counter_type, count, filter, to)));
    }
    let from = parse_target_phrase(from_tokens)?;
    let to = parse_counter_move_destination(to_tokens, &from)?;
    Ok(Some(EffectAst::subject_verb_move_counters_amount(
        counter_type,
        count,
        from,
        to,
    )))
}


/// A counter movement has a source endpoint and a destination endpoint.
/// "With the same controller" relates the destination to the source, not to
/// the spell's controller or a target set confined to one endpoint.
fn parse_counter_move_destination(tokens: &[OwnedLexToken], from: &TargetAst) -> Result<TargetAst, CardTextError> {
    use super::super::grammar::primitives as grammar;
    let Some(core_tokens) = grammar::strip_lexed_suffix_phrase(tokens, &["with", "the", "same", "controller"])
        .or_else(|| grammar::strip_lexed_suffix_phrase(tokens, &["with", "same", "controller"])) else {
        return parse_target_phrase(tokens);
    };
    fn source_reference(from: &TargetAst) -> Result<crate::filter::ObjectRef, CardTextError> {
        use crate::filter::ObjectRef;
        match from {
            TargetAst::Source(_) => Ok(ObjectRef::tagged(crate::tag::CompilerReferenceTag::SourceObject.bind())),
            TargetAst::Tagged(tag, _) => Ok(ObjectRef::tagged(tag.clone())),
            TargetAst::Object(_, Some(_), _) => Ok(ObjectRef::Target),
            TargetAst::WithCount(inner, _) | TargetAst::WithCountValue(inner, _, _) => source_reference(inner),
            _ => Err(CardTextError::ParseError("same-controller counter movement needs a bound source endpoint".into())),
        }
    }
    fn constrain(target: TargetAst, reference: crate::filter::ObjectRef) -> Result<TargetAst, CardTextError> {
        match target {
            TargetAst::Object(mut filter, target_span, it_span) => {
                let relation = PlayerFilter::ControllerOf(reference);
                // Preserve an existing controller restriction through intersection.
                filter.controller = Some(match filter.controller.take() {
                    None => relation,
                    Some(base) => PlayerFilter::Excluding {
                        base: Box::new(base),
                        excluded: Box::new(PlayerFilter::Excluding {
                            base: Box::new(PlayerFilter::Any), excluded: Box::new(relation),
                        }),
                    },
                });
                Ok(TargetAst::Object(filter, target_span, it_span))
            }
            TargetAst::WithCount(inner, count) => Ok(TargetAst::WithCount(Box::new(constrain(*inner, reference)?), count)),
            TargetAst::WithCountValue(inner, count, value) => Ok(TargetAst::WithCountValue(Box::new(constrain(*inner, reference)?), count, value)),
            _ => Err(CardTextError::ParseError("same-controller counter movement needs an object destination".into())),
        }
    }
    constrain(parse_target_phrase(core_tokens)?, source_reference(from)?)
}

fn draw_count_with_surface(count: Value, additional: bool) -> Value {
    if additional {
        count.with_surface_hint(ironsmith_core::ValueSurfaceHint::AdditionalCards)
    } else {
        count
    }
}

pub fn parse_draw(
    tokens: &[OwnedLexToken],
    subject: Option<SubjectAst>,
) -> Result<EffectAst, CardTextError> {
    if let Some(then_index) = tokens.iter().position(|token| token.is_word("then"))
        && crate::lexer::parser_token_word_refs(&tokens[then_index + 1..]) == ["discard", "one", "of", "them"]
    {
        let player = extract_subject_player(subject.clone()).unwrap_or(PlayerAst::Implicit);
        let draw_tokens = crate::util::trim_edge_punctuation_tokens(&tokens[..then_index]);
        let draw = parse_draw(draw_tokens, subject)?;
        let tag = crate::util::helper_tag_for_tokens(tokens, "drawn");
        return Ok(EffectAst::Sequence { effects: vec![
            EffectAst::TagAffected { tag: tag.clone(), effect: Box::new(draw) },
            EffectAst::subject_verb_discard(player, Value::Fixed(1), false, false,
                Some(ObjectFilter::tagged(tag).in_zone(Zone::Hand)), None),
        ] });
    }
    let clause_words = crate::lexer::token_word_refs(tokens);
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
    if clause_words.starts_with(&["up", "to"]) && tail.is_empty() {
        if let Value::Fixed(maximum) = count {
            if let Ok(max) = u32::try_from(maximum) {
                return Ok(EffectAst::Sequence { effects: vec![
                    EffectAst::subject_verb(SubjectVerbRoleAst::Chooser, player.clone(),
                        SubjectVerbActionAst::Choices(crate::cards::builders::ChoiceActionAst::ChooseNumber { min: 0, max: Some(max), source_owned: false })),
                    subject_verb_player_resource_effect(SubjectVerbRoleAst::AffectedPlayer, player,
                        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count: Value::PendingEffectMetric { source: ironsmith_core::EffectMetricSource::Outcome, metric: ironsmith_core::EffectMetric::Count } })),
                ] });
            }
        }
    }
    // Preserve the source-zone restriction before the generic this-way
    // metric parser reduces the clause to an unqualified effect count.
    if let Some(hand_owner) = crate::grammar::effects::subject_verb_registry_shapes::parse_draw_for_exiled_hand_count_shape(tokens)
    {
        let mut filter = ObjectFilter::default().in_zone(Zone::Hand);
        filter.owner = Some(match hand_owner {
            crate::grammar::effects::subject_verb_registry_shapes::ExiledHandOwner::Your => PlayerFilter::You,
            crate::grammar::effects::subject_verb_registry_shapes::ExiledHandOwner::Their => PlayerFilter::IteratedPlayer,
        });
        return Ok(EffectAst::subject_verb_draw_for_each_tagged_matching(
            player, crate::tag::CompilerReferenceTag::It.bind(), filter,
        ));
    }
    let mut effect = subject_verb_player_resource_effect(
        SubjectVerbRoleAst::AffectedPlayer,
        player,
        SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
            count: draw_count_with_surface(count.clone(), head.additional),
        }),
    );

    if !tail.is_empty() && head.parsed_offset.is_none() {
        if let Some(parsed) = parse_draw_for_each_player_condition(tail, effect.clone())? {
            effect = parsed;
        } else {
            let has_for_each = zone_move_grammar::contains_draw_for_each_shape(tail);
            if has_for_each {
                let dynamic = if let Some(value) = parse_draw_for_each_object_filter_value(tail)? {
                    value
                } else {
                    parse_dynamic_cost_modifier_value(tail)?.ok_or_else(|| {
                        CardTextError::ParseError(format!(
                            "unsupported draw for-each clause (clause: '{}')",
                            crate::lexer::token_word_refs(tokens).join(" ")
                        ))
                    })?
                };
                match count {
                    Value::Fixed(1) => count = dynamic,
                    _ => {
                        return Err(CardTextError::ParseError(format!(
                            "unsupported multiplied draw count (clause: '{}')",
                            crate::lexer::token_word_refs(tokens).join(" ")
                        )));
                    }
                }
                effect = subject_verb_player_resource_effect(
                    SubjectVerbRoleAst::AffectedPlayer,
                    player,
                    SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw {
                        count: draw_count_with_surface(count.clone(), head.additional),
                    }),
                );
            } else if let Some(parsed) = parse_draw_trailing_clause(tail, effect.clone())? {
                effect = parsed;
            } else if zone_move_grammar::tail_is_where_variable_binding(tail)
                && let Some(where_value) = crate::keyword_static::parse_value_binding_clause(tail)
            {
                // Bind the authored `where X is ...` definition into the draw
                // count so the amount and its rendering surface survive when
                // this clause parses outside the sentence dispatcher's
                // whole-sentence where-binding pass.
                let bound = crate::effect_sentences::dispatch_entry::with_where_x_surface_hints(
                    where_value,
                    tokens,
                );
                let mut effects = vec![effect.clone()];
                crate::effect_sentences::dispatch_entry::replace_unbound_x_in_effects_anywhere(
                    &mut effects,
                    &bound,
                    &clause_words.join(" "),
                )?;
                effect = effects.remove(0);
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
    #[expect(
        clippy::redundant_guards,
        reason = "the recursive rewrite preserves each predicate variant's complete named fields"
    )]
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
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControls { player, filter }) if player == PlayerAst::That => {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControls {
                    player: PlayerAst::Implicit,
                    filter,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasAtLeast {
                player,
                filter,
                count,
            }) if player == PlayerAst::That => PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasAtLeast {
                player: PlayerAst::Implicit,
                filter,
                count,
            }),
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsExactly {
                player,
                filter,
                count,
            }) if player == PlayerAst::That => PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsExactly {
                player: PlayerAst::Implicit,
                filter,
                count,
            }),
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsMost { player, filter }) if player == PlayerAst::That => {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsMost {
                    player: PlayerAst::Implicit,
                    filter,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsMoreThanEachOtherPlayer { player, filter })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsMoreThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                    filter,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsMoreThanYou { player, filter })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerControlsMoreThanYou {
                    player: PlayerAst::Implicit,
                    filter,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasLessLifeThanYou { player }) if player == PlayerAst::That => {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasLessLifeThanYou {
                    player: PlayerAst::Implicit,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreLifeThanYou { player }) if player == PlayerAst::That => {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreLifeThanYou {
                    player: PlayerAst::Implicit,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
                    player: PlayerAst::Implicit,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreCardsInHandThanYou { player })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreCardsInHandThanYou { player })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
                    player: PlayerAst::Implicit,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerTappedLandForManaThisTurn { player })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerTappedLandForManaThisTurn {
                    player: PlayerAst::Implicit,
                })
            }
            PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player })
                if player == PlayerAst::That =>
            {
                PredicateAst::Player(crate::cards::builders::PlayerPredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                    player: PlayerAst::Implicit,
                })
            }
            other => other,
        }
    }

    let clause_words = crate::lexer::token_word_refs(tokens);
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

    let tail_words = crate::lexer::token_word_refs(&predicate_tail);
    // "for each opponent who drew a card this way" refers back to a prior
    // effect's participants, not to a player predicate; the sentence-level
    // reading owns that shape.
    if tail_words.ends_with(&["this", "way"]) {
        return Ok(None);
    }
    // "for each opponent who lost life this turn" (Kaito, Bane of Nightmares).
    let iterated_life_loss = matches!(
        tail_words.as_slice(),
        ["lost", "life", "this", "turn"] | ["has", "lost", "life", "this", "turn"]
    )
    .then(|| crate::cards::builders::PredicateAst::ValueComparison {
        left: Value::LifeLostThisTurn(crate::target::PlayerFilter::IteratedPlayer),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    });
    let predicate = match iterated_life_loss {
        Some(predicate) => predicate,
        None => {
            // A player predicate this loop can't express ("who was dealt
            // combat damage this turn") may still be a turn-history count
            // ("draw a card for each player who ..."): leave it to the
            // dynamic-count reading instead of failing the clause.
            let Some(predicate) = parse_who_player_predicate_lexed(inner_tokens) else {
                return Ok(None);
            };
            bind_loop_player_predicate(predicate)
        }
    };

    let mut draw_effect = draw_effect;
    match &mut draw_effect {
        EffectAst::SubjectVerb(SubjectVerbEffectAst {
            subject: SubjectVerbSubjectAst { player, .. },
            action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { .. }),
        }) if *player == PlayerAst::Implicit => {
            *player = PlayerAst::You;
        }
        _ => {}
    }

    let effects = vec![EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate,
        if_true: vec![draw_effect],
        if_false: Vec::new(),
    })];
    Ok(Some(if shape.opponents_only {
        EffectAst::ForEach(ForEachEffectAst::ForEachOpponent { effects })
    } else {
        EffectAst::ForEach(ForEachEffectAst::ForEachPlayer { effects })
    }))
}

pub fn parse_half_rounded_down_draw_count_words(words: &[&str]) -> Option<(Value, usize)> {
    let tokens = crate::lexer::synthetic_word_tokens(words.iter().copied());
    zone_move_grammar::parse_half_rounded_down_draw_shape(&tokens)
}

pub fn parse_draw_trailing_clause(
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
            Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate,
                if_true: vec![draw_effect],
                if_false: Vec::new(),
            })))
        }
        zone_move_grammar::DrawTrailingShape::Unless => try_build_unless(
            vec![draw_effect],
            SubjectVerbPrimitiveClause::new(tokens),
            0,
        ),
    }
}

pub fn parse_draw_card_prefixed_count_value(
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

#[path = "zone_move_verbs/draw_for_each_readings.rs"]
mod draw_for_each_readings;

fn parse_draw_for_each_object_filter_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<Value>, CardTextError> {
    let Some(filter_tokens) = zone_move_grammar::strip_draw_for_each_prefix(tokens) else {
        return Ok(None);
    };
    if let Some(value) = parse_card_types_among_spells_cast_value(filter_tokens) {
        return Ok(Some(value));
    }
    // "draw a card for each graveyard with seven or more cards in it" (The
    // Master of Lake-town) counts graveyards, not the cards in them.
    let mut counted_words = vec!["for", "each"];
    counted_words.extend(crate::lexer::token_word_refs(filter_tokens));
    if let Some((value, used)) = crate::util::parse_for_each_count_value_words(&counted_words)
        && used == counted_words.len()
    {
        match value {
            Value::CountPlayersWithCardsInGraveyardAtLeast(..) => {
                return Ok(Some(
                    value.with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
                ));
            }
            // "draw a card for each of that spell's colors" (Moonveil Regent),
            // "for each creature it devoured" (Skullmulcher) and "for each
            // player being attacked" (Amber Gristle O'Maul).
            Value::SurfaceHinted {
                value: ref inner, ..
            } if matches!(
                inner.as_ref(),
                Value::ColorsOf(_)
                    | Value::SourceDevouredCreatureCount
                    | Value::PlayersBeingAttacked
            ) =>
            {
                return Ok(Some(value));
            }
            _ => {}
        }
    }
    let input = draw_for_each_readings::CountedFilter {
        tokens: filter_tokens,
        read_by_cache: Default::default(),
    };
    match draw_for_each_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }
    Ok(Some(
        Value::Count(parse_object_filter(filter_tokens, false)?)
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

pub fn parse_draw_equal_to_value(tokens: &[OwnedLexToken]) -> Result<Option<Value>, CardTextError> {
    let Some(shape) = zone_move_grammar::parse_draw_equal_shape(tokens) else {
        return Ok(None);
    };
    let words = crate::lexer::token_word_refs(tokens);
    // "that player discards cards equal to the damage" (Jagged Poppet): the
    // amount of the triggering damage event. Trigger compatibility is
    // validated where event-derived amounts are lowered.
    if matches!(
        words.as_slice(),
        ["equal", "to", "the" | "that", "damage"]
    ) {
        return Ok(Some(Value::EventValue(EventValueSpec::Amount)));
    }
    if crate::word_primitives::sequence_occurs(&words, &["differently", "named"])
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
        let spec = crate::model::ast::choose_spec_for_target(&target);
        let value = match stat {
            zone_move_grammar::DrawEqualStat::Power => Value::PowerOf(Box::new(spec)),
            zone_move_grammar::DrawEqualStat::Toughness => Value::ToughnessOf(Box::new(spec)),
            zone_move_grammar::DrawEqualStat::ManaValue => Value::ManaValueOf(Box::new(spec)),
        };
        return Ok(Some(crate::grammar::shared_util::value_expr::with_sacrificed_object_surface(
            value,
            &words,
        )));
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
) -> ironsmith_core::TotalCost<crate::model::CompilerCost> {
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
            components.push(crate::model::CompilerCost::DynamicMana(
                ironsmith_core::DynamicManaCost::new(
                    mana_cost,
                    x_value,
                    additional_generic,
                    mana_multiplier,
                    display_hint,
                ),
            ));
        } else {
            components.push(crate::model::CompilerCost::Mana(mana_cost));
        }
    }
    if let Some(life) = life {
        components.push(crate::model::CompilerCost::Life(life));
    }
    ironsmith_core::TotalCost::from_costs(components)
}

fn counter_with_payment_payer(
    target: TargetAst,
    cost: ironsmith_core::TotalCost<crate::model::CompilerCost>,
    payer: zone_move_grammar::CounterPaymentPayer,
) -> EffectAst {
    match payer {
        zone_move_grammar::CounterPaymentPayer::SpellController =>
            EffectAst::subject_verb_counter_unless_pays(target, cost),
        zone_move_grammar::CounterPaymentPayer::You =>
            EffectAst::Conditionals(ConditionalEffectAst::UnlessPays {
                effects: vec![EffectAst::subject_verb_counter(target)],
                player: PlayerAst::You,
                cost,
                before_delayed_step: false,
            }),
    }
}

/// "Counter target spell if it has the same mana value as the discarded card"
/// (Hisoka, Minamo Sensei), "counter that spell if it has the same mana value
/// as the revealed card" (Counterbalance): a resolution-time comparison with
/// the cost-discarded or just-revealed card. The spell is any spell when it is
/// targeted; the condition is checked as the ability resolves (CR 608.2c), so
/// it is not a targeting restriction.
fn parse_counter_if_same_mana_value(
    tokens: &[OwnedLexToken],
) -> Result<Option<EffectAst>, CardTextError> {
    use super::super::grammar::primitives as grammar;
    let tokens = crate::util::trim_edge_punctuation_tokens(tokens);
    let Some((if_index, (), reference)) = grammar::find_prefix(tokens, || {
        grammar::phrase(&["if", "it", "has", "the", "same", "mana", "value", "as"])
    }) else {
        return Ok(None);
    };
    let reference = crate::util::trim_edge_punctuation_tokens(reference);
    let tag = if grammar::probe_all(
        reference,
        grammar::phrase(&["the", "discarded", "card"]),
        "discarded cost card",
    )
    .is_some()
    {
        crate::tag::CompilerReferenceTag::AdditionalCostObject.bind()
    } else if grammar::probe_all(
        reference,
        winnow::combinator::alt((
            grammar::phrase(&["the", "revealed", "card"]),
            grammar::phrase(&["that", "card"]),
        )),
        "revealed card",
    )
    .is_some()
    {
        crate::tag::CompilerReferenceTag::It.bind()
    } else {
        return Ok(None);
    };
    let spell_tokens = &tokens[..if_index];
    let targeted = spell_tokens.first().is_some_and(|token| token.is_word("target"));
    let triggering = grammar::probe_all(
        spell_tokens,
        grammar::phrase(&["that", "spell"]),
        "triggering spell",
    )
    .is_some();
    if !targeted && !triggering {
        return Ok(None);
    }
    let target = parse_counter_target_phrase(spell_tokens)?;
    let mut filter = ObjectFilter::default();
    filter.tagged_constraints.push(crate::target::TaggedObjectConstraint {
        tag: tag.into(),
        relation: crate::target::TaggedOpbjectRelation::SameManaValueAsTagged,
    });
    // "that spell" in a cast trigger is the triggering spell (CR 603.7c).
    let predicate = if targeted {
        crate::cards::builders::PredicateAst::TargetMatches(filter)
    } else {
        crate::cards::builders::PredicateAst::TaggedMatches(
            crate::tag::CompilerReferenceTag::Triggering.bind(),
            filter,
        )
    };
    Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
        predicate,
        if_true: vec![EffectAst::subject_verb_counter(target)],
        if_false: Vec::new(),
    })))
}

pub fn parse_counter(tokens: &[OwnedLexToken]) -> Result<EffectAst, CardTextError> {
    if let Some(effect) = parse_counter_unless_source_damage(tokens)? {
        return Ok(effect);
    }

    if let Some(effect) = parse_counter_if_same_mana_value(tokens)? {
        return Ok(effect);
    }

    if let Some(spec) = split_trailing_if_clause_lexed(tokens) {
        let target = parse_counter_target_phrase(spec.leading_tokens)?;
        return Ok(EffectAst::Conditionals(ConditionalEffectAst::TrailingIf {
            predicate: spec.predicate,
            effects: vec![EffectAst::subject_verb_counter(target)],
        }));
    }

    let clause_words = crate::lexer::token_word_refs(tokens);
    let shape =
        zone_move_grammar::parse_counter_clause_shape(tokens).map_err(|error| match error {
            zone_move_grammar::CounterClauseShapeError::MissingPays => {
                CardTextError::ParseError(format!(
                    "missing pays keyword (clause: '{}')",
                    clause_words.join(" ")
                ))
            }
            zone_move_grammar::CounterClauseShapeError::UnsupportedPayer => {
                CardTextError::ParseError(format!("unsupported counter payment actor (clause: '{}')", clause_words.join(" ")))
            }
        })?;
    let unless_shape = match shape {
        zone_move_grammar::CounterClauseShape::SecondSpellThisTurn { target_tokens } => {
            return Ok(EffectAst::Conditionals(ConditionalEffectAst::Conditional {
                predicate: crate::cards::builders::PredicateAst::TargetSpellCastOrderThisTurn(2),
                if_true: vec![EffectAst::subject_verb_counter(TargetAst::Spell(
                    span_from_tokens(&target_tokens),
                ))],
                if_false: Vec::new(),
            }));
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
    match crate::activation_and_restrictions::parse_payment_clause_as_total_cost(
        payment_clause_tokens,
    ) {
        Ok(Some(cost)) => {
            let should_keep_subject_verb_dynamic_path = has_dynamic_payment_tail
                && cost.as_one_of().is_none()
                && cost.dynamic_mana_cost().is_none();
            if !should_keep_subject_verb_dynamic_path {
                let cost = cost.try_map(|mut component| {
                    if let crate::model::CompilerCost::DynamicMana(dynamic) = &mut component {
                        for slot in [&mut dynamic.x_value, &mut dynamic.additional_generic, &mut dynamic.multiplier] {
                            if let Some(value) = slot.take() {
                                *slot = Some(crate::effect_sentences::chain_carry::bind_it_metric_to_declared_target(value, &target));
                            }
                        }
                    }
                    Ok::<_, CardTextError>(component)
                })?;
                return Ok(counter_with_payment_payer(target, cost, unless_shape.payer));
            }
        }
        Ok(None) => {
            if !has_dynamic_payment_tail {
                return Err(CardTextError::ParseError(format!(
                    "unsupported counter-unless payment cost (clause: '{}')",
                    crate::lexer::token_word_refs(tokens).join(" ")
                )));
            }
        }
        Err(err) => {
            if !has_dynamic_payment_tail {
                return Err(CardTextError::ParseError(format!(
                    "unsupported counter-unless payment cost (clause: '{}'): {err}",
                    crate::lexer::token_word_refs(tokens).join(" ")
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
                crate::lexer::token_word_refs(tokens).join(" ")
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
            let trailing_words = crate::lexer::token_word_refs(trailing_tokens);
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
                    x_value = Some(crate::effect_sentences::chain_carry::bind_it_metric_to_declared_target(value, &target));
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
            crate::lexer::token_word_refs(tokens).join(" ")
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

    Ok(counter_with_payment_payer(
        target,
        counter_unless_payment_total_cost(
            mana,
            life,
            additional_generic,
            mana_multiplier,
            x_value,
            dynamic_display_hint,
        ),
        unless_shape.payer,
    ))
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
    if !crate::word_primitives::parse_sequence_complete(&controller_words, &["its", "controller"]) {
        return Ok(None);
    }

    let alternative_clause = alternative_clause.trimmed();
    let Some((source_clause, damage_clause)) =
        alternative_clause.split_once_on_word_any(&["deal", "deals"])
    else {
        return Ok(None);
    };
    let source_words = source_clause.trimmed_word_refs();
    if !crate::word_primitives::parse_any_sequence_complete(
        &source_words,
        &[&["this"], &["this", "spell"], &["this", "source"]],
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
    if !crate::word_primitives::parse_any_sequence_complete(
        &target_words,
        &[
            &["them"],
            &["to", "them"],
            &["that", "player"],
            &["to", "that", "player"],
        ],
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
    Ok(Some(EffectAst::Conditionals(ConditionalEffectAst::UnlessAction {
        effects: vec![EffectAst::subject_verb_counter(target)],
        alternative: vec![alternative],
        player: PlayerAst::ItsController,
    })))
}

#[cfg(test)]
mod turn_history_draw_tests {
    use super::*;

    fn lex(text: &str) -> Vec<OwnedLexToken> {
        let mut tokens = crate::lexer::lex_line(text, 0).expect("lex");
        for token in &mut tokens {
            token.lowercase_word();
        }
        tokens
    }

    #[test]
    fn additional_draw_surface_survives_into_the_subject_verb_ast() {
        let parsed = parse_draw(&lex("an additional card"), None).expect("additional draw parse");
        let EffectAst::SubjectVerb(SubjectVerbEffectAst {
            action: SubjectVerbActionAst::LifeResources(LifeResourceActionAst::Draw { count }),
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

    #[test]
    fn draw_for_each_preserves_distinct_power_aggregate() {
        let value = parse_draw_for_each_object_filter_value(&lex(
            "for each different power among creatures you control",
        ))
        .expect("draw value parse")
        .expect("distinct-power value");

        assert!(value.has_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach));
        let Value::DistinctPowers(filter) = value.unhinted() else {
            panic!("expected a distinct-power aggregate, got {value:#?}");
        };
        assert_eq!(filter.controller, Some(PlayerFilter::You));
        assert_eq!(filter.card_types, [crate::types::CardType::Creature]);
    }
}

#[cfg(test)]
mod relative_draw_tests {
    use super::*;
    #[test]
    fn complete_draw_quantities_use_shared_semantic_values() {
        let tokens = crate::lexer::lex_line("equal to the difference", 0).unwrap();
        assert_eq!(
            parse_draw_equal_to_value(&tokens).unwrap(),
            Some(Value::PendingComparisonDifference)
        );
        let tokens = crate::lexer::lex_line("equal to the milled card's mana value", 0).unwrap();
        assert!(
            matches!(parse_draw_equal_to_value(&tokens).unwrap(), Some(Value::PendingPriorEffectMetric(query))
            if query.action == Some(ironsmith_core::PriorEffectAction::Milled)
                && query.metric == ironsmith_core::EffectMetric::FirstManaValue)
        );
        let tokens =
            crate::lexer::lex_line("equal to the difference among strange things", 0).unwrap();
        assert!(parse_draw_equal_to_value(&tokens).unwrap().is_none());
    }
}

#[cfg(test)]
mod counter_payment_actor_tests {
    use super::*;
    #[test]
    fn explicit_you_and_spell_controller_keep_different_payment_owners() {
        let parse = |text| parse_counter(&crate::lexer::lex_line(text, 0).unwrap());
        assert!(matches!(parse("that spell unless you sacrifice a creature").unwrap(),
            EffectAst::Conditionals(ConditionalEffectAst::UnlessPays { player: PlayerAst::You, .. })));
        for text in ["target spell unless its controller discards their hand",
            "target spell unless its controller discards a card",
            "target spell unless its controller exiles all cards from their graveyard",
            "target spell an opponent controls unless they pay {1}"] {
            assert!(parse(text).is_ok(), "{text}");
        }
        for text in ["target spell unless its controller exiles all cards from their graveyard then wins the game",
            "target spell unless a creature pays {1}",
            "target spell unless its controller discards their library",
            "target spell unless you sacrifice a creature nonsense"] {
            assert!(parse(text).is_err(), "{text}");
        }
    }
}

#[cfg(test)]
mod counted_transfer_shape_tests {
    use super::*;
    #[test]
    fn complete_donor_sets_and_named_all_amounts_keep_typed_cardinality() {
        for (text, all, amount) in [
            ("any number of +1/+1 counters from other permanents you control onto this creature", true, "AnyNumber"),
            ("all +1/+1 counters from all creatures onto it", true, "All"),
            ("all charge counters from target artifact onto another target artifact", false, "All"),
            ("any number of +1/+1 counters from this creature onto another target creature", false, "AnyNumber"),
        ] {
            let effect = parse_move(&crate::lexer::lex_line(text, 0).unwrap()).unwrap();
            let debug = format!("{effect:?}");
            assert!(debug.contains(&format!("from_all: {all}")), "{debug}");
            assert!(debug.contains(amount), "{debug}");
        }
        for text in [
            "all 2 counters from target creature onto another target creature",
            "two +1/+1 counters from creatures you control onto this creature",
            "all +1/+1 counters from target creature onto another target creature and draw a card",
        ] {
            assert!(parse_move(&crate::lexer::lex_line(text, 0).unwrap()).is_err(), "{text}");
        }
    }
}

/// "draw a card for each card type among spells you've cast this turn"
/// (April O'Neil, Hacktivist): distinct card types (CR 205.2a) among the
/// spells counted by the ordinary cast-history quantity. The cast-history
/// reading of the remainder supplies the player and spell filter; dropping
/// the "card type among" head would silently count spells instead.
fn parse_card_types_among_spells_cast_value(filter_tokens: &[OwnedLexToken]) -> Option<Value> {
    let words = crate::lexer::token_word_refs(filter_tokens);
    let rest = match words.as_slice() {
        ["card", "type" | "types", "among", rest @ ..] if !rest.is_empty() => rest,
        _ => return None,
    };
    let mut counted_words = vec!["for", "each"];
    counted_words.extend_from_slice(rest);
    let (value, used) = crate::util::parse_for_each_count_value_words(&counted_words)?;
    if used != counted_words.len() {
        return None;
    }
    let (player, filter) = match value.unhinted() {
        Value::TurnHistoryCount(ironsmith_core::TurnHistoryCount::SpellsCast { player, filter, .. })
        | Value::SpellsCastThisTurnMatching { player, filter, .. } => (player.clone(), filter.clone()),
        Value::SpellsCastThisTurn(player) => (player.clone(), ObjectFilter::default()),
        _ => return None,
    };
    Some(
        Value::CardTypesAmongSpellsCastThisTurn { player, filter }
            .with_surface_hint(ironsmith_core::ValueSurfaceHint::ForEach),
    )
}
