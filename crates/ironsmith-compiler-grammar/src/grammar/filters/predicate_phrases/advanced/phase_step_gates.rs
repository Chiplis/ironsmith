use super::*;

use crate::CounterType;
use crate::target::ChooseSpec;
use ironsmith_core::TurnHistoryCount;

const POWER_ABOVE_BASE_SUFFIX: &[&str] =
    &["with", "power", "greater", "than", "its", "base", "power"];

use crate::recognition::ParseOutcome;
#[path = "phase_step_gates/gate_readings.rs"]
mod gate_readings;

pub(super) fn parse_phase_step_gate_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let input = gate_readings::Gate { tokens };
    match gate_readings::read(&input) {
        ParseOutcome::Match(matched) => return Ok(Some(matched.value.value)),
        ParseOutcome::NoMatch => {}
        ParseOutcome::Error(diagnostic) => return Err(diagnostic.into_card_text_error()),
    }

    Ok(None)
}

fn value_at_least(left: Value, amount: i32) -> PredicateAst {
    PredicateAst::ValueComparison {
        left,
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(amount),
    }
}

fn value_equal(left: Value, amount: i32) -> PredicateAst {
    PredicateAst::ValueComparison {
        left,
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Fixed(amount),
    }
}

fn object_filter_comparison(
    comparison: crate::effect::Comparison,
) -> Option<crate::filter::Comparison> {
    use crate::effect::Comparison as ValueComparison;
    use crate::filter::Comparison as FilterComparison;

    Some(match comparison {
        ValueComparison::GreaterThan(value) => FilterComparison::GreaterThan(value),
        ValueComparison::GreaterThanOrEqual(value) => FilterComparison::GreaterThanOrEqual(value),
        ValueComparison::Equal(value) => FilterComparison::Equal(value),
        ValueComparison::OneOf(values) => FilterComparison::OneOf(values.to_vec()),
        ValueComparison::LessThan(value) => FilterComparison::LessThan(value),
        ValueComparison::LessThanOrEqual(value) => FilterComparison::LessThanOrEqual(value),
        ValueComparison::NotEqual(value) => FilterComparison::NotEqual(value),
        ValueComparison::BetweenInclusive(..) => return None,
    })
}

fn parse_existing_value_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_turn_history_value_gate(tokens)
        .or_else(|| parse_life_above_starting_total_gate(tokens))
        .or_else(|| parse_cards_in_library_gate(tokens))
        .or_else(|| parse_damage_received_gate(tokens))
        .or_else(|| parse_discard_history_gate(tokens))
        .or_else(|| parse_life_changed_gate(tokens))
        .or_else(|| parse_counter_total_gate(tokens))
        .or_else(|| parse_two_color_devotion_gate(tokens))
        .or_else(|| parse_source_exiled_card_gate(tokens))
        .or_else(|| parse_exact_cards_in_hand_gate(tokens))
}

fn parse_turn_history_value_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);

    if surface::exact(clause, &["you", "created", "a", "token", "this", "turn"]) {
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::TokensCreated(PlayerFilter::You)),
            1,
        ));
    }

    if surface::exact_any(
        clause,
        &[
            &[
                "a",
                "creature",
                "entered",
                "the",
                "battlefield",
                "under",
                "an",
                "opponents",
                "control",
                "this",
                "turn",
            ],
            &[
                "a",
                "creature",
                "entered",
                "battlefield",
                "under",
                "an",
                "opponents",
                "control",
                "this",
                "turn",
            ],
            &[
                "a",
                "creature",
                "entered",
                "under",
                "an",
                "opponents",
                "control",
                "this",
                "turn",
            ],
        ],
    ) {
        let mut filter = ObjectFilter::creature();
        filter.controller = Some(PlayerFilter::Opponent);
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::EnteredBattlefield(filter)),
            1,
        ));
    }

    let words = clause.word_refs();
    if crate::word_primitives::parse_sequence_prefix(&words, &["a", "counter", "was", "put", "on"])
        && crate::word_primitives::parse_sequence_suffix(&words, &["this", "turn"])
        && words.len() > 7
        && let Some(source) = clause.between_word_range(5, words.len() - 2)
        && is_source_reference_clause(source)
    {
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
                counter_type: None,
                filter: ObjectFilter::source(),
            }),
            1,
        ));
    }

    if surface::exact(
        clause,
        &[
            "you",
            "sacrificed",
            "three",
            "or",
            "more",
            "clues",
            "this",
            "turn",
        ],
    ) {
        let mut filter = ObjectFilter::default().with_subtype(Subtype::Clue);
        filter.card_types.push(CardType::Artifact);
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::Sacrificed {
                player: PlayerFilter::You,
                filter,
            }),
            3,
        ));
    }

    if surface::exact(
        clause,
        &["you", "sacrificed", "a", "permanent", "this", "turn"],
    ) {
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::Sacrificed {
                player: PlayerFilter::You,
                filter: ObjectFilter::permanent(),
            }),
            1,
        ));
    }

    for (subtype, other, words) in [
        (
            Subtype::Phyrexian,
            false,
            &[
                "a",
                "phyrexian",
                "died",
                "under",
                "your",
                "control",
                "this",
                "turn",
            ][..],
        ),
        (
            Subtype::Human,
            true,
            &[
                "another", "human", "died", "under", "your", "control", "this", "turn",
            ][..],
        ),
    ] {
        if surface::exact(clause, words) {
            let mut filter = ObjectFilter::creature().with_subtype(subtype);
            filter.controller = Some(PlayerFilter::You);
            filter.other = other;
            return Some(value_at_least(
                Value::TurnHistoryCount(TurnHistoryCount::died(filter)),
                1,
            ));
        }
    }

    if surface::exact(clause, &["no", "creatures", "attacked", "this", "turn"]) {
        return Some(value_equal(
            Value::TurnHistoryCount(TurnHistoryCount::CreaturesAttackedWith {
                player: PlayerFilter::Any,
                filter: ObjectFilter::creature(),
            }),
            0,
        ));
    }

    if surface::exact(
        clause,
        &[
            "a",
            "permanent",
            "was",
            "put",
            "into",
            "your",
            "hand",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    ) {
        let mut filter = ObjectFilter::permanent();
        filter.owner = Some(PlayerFilter::You);
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::MovedZones {
                filter,
                from: Some(Zone::Battlefield),
                to: Some(Zone::Hand),
            }),
            1,
        ));
    }

    if surface::exact_any(
        clause,
        &[
            &[
                "one", "or", "more", "cards", "were", "put", "into", "exile", "this", "turn",
            ],
            &["a", "card", "was", "put", "into", "exile", "this", "turn"],
        ],
    ) {
        let mut filter = ObjectFilter::default();
        filter.nontoken = true;
        filter.set_explicit_card_noun(true);
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::MovedZones {
                filter,
                from: None,
                to: Some(Zone::Exile),
            }),
            1,
        ));
    }

    if surface::exact_any(
        clause,
        &[
            &["a", "card", "left", "your", "graveyard", "this", "turn"],
            &[
                "a",
                "creature",
                "card",
                "left",
                "your",
                "graveyard",
                "this",
                "turn",
            ],
        ],
    ) {
        let clause_words = clause.word_refs();
        let mut filter = if crate::word_primitives::contains_word(&clause_words, "creature") {
            ObjectFilter::creature()
        } else {
            ObjectFilter::default()
        };
        filter.owner = Some(PlayerFilter::You);
        filter.nontoken = true;
        filter.set_explicit_card_noun(true);
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::MovedZones {
                filter,
                from: Some(Zone::Graveyard),
                to: None,
            }),
            1,
        ));
    }

    if surface::exact(
        clause,
        &[
            "a",
            "+1/+1",
            "counter",
            "was",
            "put",
            "on",
            "a",
            "permanent",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ],
    ) {
        let mut filter = ObjectFilter::permanent();
        filter.controller = Some(PlayerFilter::You);
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
                counter_type: Some(CounterType::PlusOnePlusOne),
                filter,
            }),
            1,
        ));
    }

    if surface::exact(
        clause,
        &[
            "you", "put", "a", "counter", "on", "a", "creature", "this", "turn",
        ],
    ) {
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
                counter_type: None,
                filter: ObjectFilter::creature(),
            }),
            1,
        ));
    }

    if surface::exact(
        clause,
        &[
            "you", "cycled", "two", "or", "more", "cards", "this", "turn",
        ],
    ) {
        return Some(value_at_least(
            Value::TurnHistoryCount(TurnHistoryCount::Cycled(PlayerFilter::You)),
            2,
        ));
    }

    if surface::exact(
        clause,
        &[
            "you",
            "attacked",
            "with",
            "a",
            "hero",
            "this",
            "turn",
            "or",
            "a",
            "hero",
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ],
    ) {
        let hero = ObjectFilter::creature().with_subtype(Subtype::Hero);
        let mut entered_hero = hero.clone();
        entered_hero.controller = Some(PlayerFilter::You);
        return Some(PredicateAst::Or(
            Box::new(value_at_least(
                Value::TurnHistoryCount(TurnHistoryCount::CreaturesAttackedWith {
                    player: PlayerFilter::You,
                    filter: hero,
                }),
                1,
            )),
            Box::new(value_at_least(
                Value::TurnHistoryCount(TurnHistoryCount::EnteredBattlefield(entered_hero)),
                1,
            )),
        ));
    }

    None
}

fn parse_life_above_starting_total_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    if !surface::exact(relation.subject_clause, &["you"]) {
        return None;
    }
    let suffix = &["life", "more", "than", "your", "starting", "life", "total"];
    let amount_clause = relation.tail_clause.without_trailing_phrase(suffix);
    if amount_clause.tokens().len() == relation.tail_clause.tokens().len() {
        return None;
    }
    let (comparison, used) = predicate_quantity_prefix_tokens(amount_clause.tokens())?;
    if used != amount_clause.tokens().len() {
        return None;
    }
    let amount = comparison_to_strict_at_least_threshold(&comparison)? as i32;
    Some(PredicateAst::ValueComparison {
        left: Value::LifeTotal(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Add(
            Box::new(Value::StartingLifeTotal(PlayerFilter::You)),
            Box::new(Value::Fixed(amount)),
        ),
    })
}

fn parse_cards_in_library_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if surface::exact(
        LexedClause::new(tokens),
        &["your", "library", "has", "no", "cards", "in", "it"],
    ) {
        return Some(PredicateAst::ValueComparison {
            left: Value::CardsInLibrary(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::Equal,
            right: Value::Fixed(0),
        });
    }

    let relation = parse_has_relation_clauses(tokens)?;
    let player = if surface::exact(relation.subject_clause, &["you"]) {
        PlayerFilter::You
    } else if surface::exact_any(
        relation.subject_clause,
        &[&["an", "opponent"], &["a", "opponent"], &["opponent"]],
    ) {
        PlayerFilter::Opponent
    } else {
        return None;
    };
    let card_in_phrases: &[&[&str]] = &[&["card", "in"], &["cards", "in"]];
    let atoms = [
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(card_in_phrases),
        ),
        WinnowSequence::any_phrase(card_in_phrases),
        WinnowSequence::modifier("library", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let library = matched.capture_clause("library", relation.tail_clause)?;
    let library_matches = match player {
        PlayerFilter::You => surface::exact(library, &["your", "library"]),
        PlayerFilter::Opponent => surface::exact_any(
            library,
            &[&["their", "library"], &["that", "opponents", "library"]],
        ),
        _ => false,
    };
    if !library_matches {
        return None;
    }
    let quantity = matched.capture_clause("quantity", relation.tail_clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }
    let (operator, amount) = comparison_to_value_comparison_operator(comparison)?;
    Some(PredicateAst::ValueComparison {
        left: Value::CardsInLibrary(player),
        operator,
        right: Value::Fixed(amount),
    })
}

fn parse_damage_received_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::phrase(&["you", "were", "dealt"]),
        WinnowSequence::amount("amount", WinnowCaptureKind::UntilPhrase(&["damage"])),
        WinnowSequence::phrase(&["damage", "this", "turn"]),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let amount = matched.capture_clause("amount", clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(amount.tokens())?;
    if used != amount.tokens().len() {
        return None;
    }
    let (operator, amount) = comparison_to_value_comparison_operator(comparison)?;
    Some(PredicateAst::ValueComparison {
        left: Value::DamageDealtToPlayersThisTurn(PlayerFilter::You),
        operator,
        right: Value::Fixed(amount),
    })
}

fn parse_discard_history_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let player = if surface::exact_any(
        clause,
        &[
            &["you", "discarded", "a", "card", "this", "turn"],
            &[
                "you",
                "discarded",
                "one",
                "or",
                "more",
                "cards",
                "this",
                "turn",
            ],
        ],
    ) {
        PlayerFilter::You
    } else if surface::exact_any(
        clause,
        &[
            &["an", "opponent", "discarded", "a", "card", "this", "turn"],
            &[
                "an",
                "opponent",
                "discarded",
                "one",
                "or",
                "more",
                "cards",
                "this",
                "turn",
            ],
        ],
    ) {
        PlayerFilter::Opponent
    } else {
        return None;
    };
    Some(value_at_least(Value::CardsDiscardedThisTurn(player), 1))
}

fn parse_life_changed_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    if !surface::exact(
        LexedClause::new(tokens),
        &["you", "gained", "or", "lost", "life", "this", "turn"],
    ) {
        return None;
    }
    Some(PredicateAst::Or(
        Box::new(value_at_least(
            Value::LifeGainedThisTurn(PlayerFilter::You),
            1,
        )),
        Box::new(value_at_least(
            Value::LifeLostThisTurn(PlayerFilter::You),
            1,
        )),
    ))
}

fn parse_counter_total_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::phrase(&["there", "are"]),
        WinnowSequence::amount("amount", WinnowCaptureKind::UntilPhrase(&["counters"])),
        WinnowSequence::phrase(&["counters", "among"]),
        WinnowSequence::object(
            "objects",
            WinnowCaptureKind::UntilPhrase(&["you", "control"]),
        ),
        WinnowSequence::phrase(&["you", "control"]),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let amount = matched.capture_clause("amount", clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(amount.tokens())?;
    if used != amount.tokens().len() {
        return None;
    }
    let (operator, amount) = comparison_to_value_comparison_operator(comparison)?;
    let objects = matched.capture_clause("objects", clause)?;
    let mut filter =
        crate::grammar::primitives::probe_shape(parse_object_filter(objects.tokens(), false))?;
    filter.controller = Some(PlayerFilter::You);
    filter.zone.get_or_insert(Zone::Battlefield);
    Some(PredicateAst::ValueComparison {
        left: Value::CountersOn(Box::new(ChooseSpec::All(filter)), None),
        operator,
        right: Value::Fixed(amount),
    })
}

fn parse_two_color_devotion_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::phrase(&["your", "devotion", "to"]),
        WinnowSequence::object("first_color", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::word("and"),
        WinnowSequence::object("second_color", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::word("is"),
        WinnowSequence::amount("amount", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let first = Color::from_name(
        matched
            .capture_clause("first_color", clause)?
            .token(0)?
            .parser_text(),
    )?;
    let second = Color::from_name(
        matched
            .capture_clause("second_color", clause)?
            .token(0)?
            .parser_text(),
    )?;
    if first == second {
        return None;
    }
    let amount = matched.capture_clause("amount", clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(amount.tokens())?;
    if used != amount.tokens().len() {
        return None;
    }
    let (operator, amount) = comparison_to_value_comparison_operator(comparison)?;
    Some(PredicateAst::ValueComparison {
        left: Value::Add(
            Box::new(Value::Devotion {
                player: PlayerFilter::You,
                color: first,
            }),
            Box::new(Value::Devotion {
                player: PlayerFilter::You,
                color: second,
            }),
        ),
        operator,
        right: Value::Fixed(amount),
    })
}

fn parse_source_exiled_card_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::phrase(&["there", "are"]),
        WinnowSequence::object("cards", WinnowCaptureKind::OneOf(&["card", "cards"])),
        WinnowSequence::phrase(&["exiled", "with"]),
        WinnowSequence::modifier("source", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let source = matched.capture_clause("source", clause)?;
    if !is_source_reference_clause(source) {
        return None;
    }
    let mut filter = ObjectFilter::tagged(crate::tag::CompilerReferenceTag::SourceExiled.bind());
    filter.zone = Some(Zone::Exile);
    let source_words = source.word_refs();
    filter.source_surface = source_reference_surface_for_words(&source_words)
        .or_else(|| this_source_surface_for_words(&source_words));
    Some(value_at_least(Value::Count(filter), 1))
}

fn parse_exact_cards_in_hand_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_player_cards_in_hand_condition(tokens)?;
    let crate::effect::Comparison::Equal(amount) = condition.comparison else {
        return None;
    };
    crate::grammar::conditions::unconditional_player_filter(condition.player)
        .filter(|_| amount > 1)
        .map(|player| value_equal(Value::CardsInHand(player), amount))
}

fn parse_control_gate(tokens: &[OwnedLexToken]) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_all_controlled_objects_share_color_gate(tokens) {
        return Ok(Some(predicate));
    }

    let Some(relation) = parse_control_relation_clauses(tokens, false) else {
        return Ok(None);
    };
    let (player, controller) = if surface::exact(relation.subject_clause, &["you"]) {
        (PlayerAst::You, PlayerFilter::You)
    } else if surface::exact_any(
        relation.subject_clause,
        &[
            &["your", "opponents"],
            &["your", "opponent"],
            &["opponents"],
            &["an", "opponent"],
        ],
    ) {
        (PlayerAst::Opponent, PlayerFilter::Opponent)
    } else {
        return Ok(None);
    };

    let tail = relation.tail_clause;
    if tail
        .token(0)
        .is_some_and(|token| token_word_is(token, "no"))
    {
        let object_tokens = tail.tokens().get(1..).unwrap_or_default();
        if object_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter(object_tokens, false)?;
        filter.controller = Some(controller);
        return Ok(Some(PredicateAst::PlayerControlsNo { player, filter }));
    }

    let Some((comparison, quantity_used)) = predicate_quantity_prefix_tokens(tail.tokens()) else {
        return Ok(None);
    };
    let mut filter_tokens = tail.tokens().get(quantity_used..).unwrap_or_default();
    if filter_tokens.is_empty() {
        return Ok(None);
    }
    let mut above_base = false;
    let filter_clause = LexedClause::new(filter_tokens);
    let stripped = filter_clause.without_trailing_phrase(POWER_ABOVE_BASE_SUFFIX);
    if stripped.tokens().len() != filter_tokens.len() {
        above_base = true;
        filter_tokens = stripped.tokens();
    }
    let mut filter = parse_object_filter(filter_tokens, false)?;
    filter.controller = Some(controller);
    filter.power_greater_than_base_power |= above_base;

    let authored_exactly = tail
        .tokens()
        .iter()
        .any(|token| token_word_is(token, "exactly"));
    let predicate = match comparison {
        crate::effect::Comparison::Equal(count) if count >= 0 => {
            if count == 1 && !authored_exactly {
                PredicateAst::PlayerControls { player, filter }
            } else {
                PredicateAst::PlayerControlsExactly {
                    player,
                    filter,
                    count: count as u32,
                }
            }
        }
        comparison => {
            let Some(count) = comparison_to_at_least_threshold(&comparison) else {
                return Ok(None);
            };
            if count <= 1 {
                PredicateAst::PlayerControls { player, filter }
            } else {
                PredicateAst::PlayerHasAtLeast {
                    player,
                    filter,
                    count,
                }
            }
        }
    };
    Ok(Some(predicate))
}

fn parse_all_controlled_objects_share_color_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::word("all"),
        WinnowSequence::object(
            "objects",
            WinnowCaptureKind::UntilPhrase(&["you", "control"]),
        ),
        WinnowSequence::phrase(&["you", "control", "are"]),
        WinnowSequence::modifier("color", WinnowCaptureKind::WordCount(1)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let color = parse_color(
        matched
            .capture_clause("color", clause)?
            .token(0)?
            .parser_text(),
    )?;
    let objects = matched.capture_clause("objects", clause)?;
    let mut filter =
        crate::grammar::primitives::probe_shape(parse_object_filter(objects.tokens(), false))?;
    filter.controller = Some(PlayerFilter::You);
    filter = filter.without_colors(color);
    Some(PredicateAst::PlayerControlsNo {
        player: PlayerAst::You,
        filter,
    })
}

fn parse_attachment_gate(tokens: &[OwnedLexToken]) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);

    if let Some(relation) = parse_has_relation_clauses(tokens)
        && surface::exact_any(
            relation.subject_clause,
            &[&["enchanted", "creature"], &["enchanted", "permanent"]],
        )
    {
        let words = relation.tail_clause.word_refs();
        if let Some((constraint, consumed)) = parse_filter_keyword_constraint_words(&words)
            && consumed == words.len()
        {
            let mut filter = if surface::exact(relation.subject_clause, &["enchanted", "creature"])
            {
                ObjectFilter::creature()
            } else {
                ObjectFilter::permanent()
            };
            apply_filter_keyword_constraint(&mut filter, constraint, false);
            return Ok(Some(PredicateAst::AttachedToSourceMatches(filter)));
        }
    }

    if let Some(relation) = parse_copula_relation_clauses(tokens)
        && surface::exact_any(
            relation.subject_clause,
            &[&["enchanted", "creature"], &["enchanted", "permanent"]],
        )
    {
        let mut filter = if surface::exact(relation.subject_clause, &["enchanted", "creature"]) {
            ObjectFilter::creature()
        } else {
            ObjectFilter::permanent()
        };
        let tail = relation.tail_clause;
        if tail.word_refs().len() == 1
            && let Some(color) = tail
                .token(0)
                .and_then(|token| parse_color(token.parser_text()))
        {
            filter.colors = Some(color);
            return Ok(Some(PredicateAst::AttachedToSourceMatches(filter)));
        }
        if surface::exact(tail, &["tapped"]) {
            filter.tapped = true;
            return Ok(Some(PredicateAst::AttachedToSourceMatches(filter)));
        }
        if surface::exact_any(
            tail,
            &[
                &[
                    "a",
                    "creature",
                    "with",
                    "the",
                    "greatest",
                    "power",
                    "among",
                    "creatures",
                    "on",
                    "the",
                    "battlefield",
                ],
                &[
                    "a",
                    "creature",
                    "with",
                    "greatest",
                    "power",
                    "among",
                    "creatures",
                    "on",
                    "battlefield",
                ],
            ],
        ) {
            let global_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
            filter = global_creatures.clone();
            filter.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
                Value::GreatestPower(global_creatures),
            )));
            return Ok(Some(PredicateAst::AttachedToSourceMatches(filter)));
        }
    }

    if let Some(relation) = parse_copula_relation_clauses(tokens)
        && surface::exact_any(
            relation.subject_clause,
            &[
                &["enchanted", "creature's", "power"],
                &["enchanted", "creatures", "power"],
            ],
        )
    {
        let (comparison, used) = parse_quantity_comparison_prefix(
            relation.tail_clause.tokens(),
            false,
            false,
            "enchanted-object power gate",
        )?;
        if used == relation.tail_clause.tokens().len() {
            let Some(comparison) = object_filter_comparison(comparison) else {
                return Ok(None);
            };
            let mut filter = ObjectFilter::creature();
            filter.power = Some(comparison);
            return Ok(Some(PredicateAst::AttachedToSourceMatches(filter)));
        }
    }

    let atoms = [
        WinnowSequence::object(
            "attachment",
            WinnowCaptureKind::UntilPhrase(&["is", "attached", "to"]),
        ),
        WinnowSequence::phrase(&["is", "attached", "to"]),
        WinnowSequence::modifier("attached_to", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let attachment = matched
        .capture_clause("attachment", clause)
        .expect("attachment capture");
    if !attachment
        .token(0)
        .is_some_and(|token| token_word_is(token, "enchanted"))
    {
        return Ok(None);
    }
    let attachment_tokens = attachment.tokens().get(1..).unwrap_or_default();
    let attached_to = matched
        .capture_clause("attached_to", clause)
        .expect("attached-to capture");
    let mut filter = parse_object_filter(attachment_tokens, false)?;
    filter.attached_to_object = Some(Box::new(parse_object_filter(attached_to.tokens(), false)?));
    Ok(Some(PredicateAst::AttachedToSourceMatches(filter)))
}

fn parse_empty_battlefield_gate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::phrase(&["there", "are", "no"]),
        WinnowSequence::object("objects", WinnowCaptureKind::UntilPhrase(&["on"])),
        WinnowSequence::word("on"),
        WinnowSequence::modifier("battlefield", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let battlefield = matched
        .capture_clause("battlefield", clause)
        .expect("battlefield capture");
    if !is_battlefield_zone_clause(battlefield) {
        return Ok(None);
    }
    let objects = matched
        .capture_clause("objects", clause)
        .expect("object capture");
    let mut filter = parse_object_filter(objects.tokens(), false)?;
    filter.zone = Some(Zone::Battlefield);
    Ok(Some(value_equal(Value::Count(filter), 0)))
}

fn parse_source_state_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if surface::exact_any(
        clause,
        &[
            &[
                "this",
                "creature",
                "didnt",
                "enter",
                "the",
                "battlefield",
                "this",
                "turn",
            ],
            &[
                "this",
                "creature",
                "did",
                "not",
                "enter",
                "the",
                "battlefield",
                "this",
                "turn",
            ],
        ],
    ) {
        let mut filter = ObjectFilter::creature();
        filter.entered_battlefield_this_turn = true;
        return Some(PredicateAst::Not(Box::new(PredicateAst::SourceMatches(
            filter,
        ))));
    }
    if surface::exact(
        clause,
        &["this", "creature", "was", "dealt", "damage", "this", "turn"],
    ) {
        let mut filter = ObjectFilter::creature();
        filter.was_dealt_damage_this_turn = true;
        return Some(PredicateAst::SourceMatches(filter));
    }
    if surface::exact(
        clause,
        &[
            "this", "creature", "dealt", "damage", "to", "an", "opponent", "this", "turn",
        ],
    ) {
        let mut filter = ObjectFilter::creature();
        filter.dealt_damage_to_player_this_turn = Some(PlayerFilter::Opponent);
        return Some(PredicateAst::SourceMatches(filter));
    }
    if surface::exact_any(
        clause,
        &[
            &["this", "creature", "wasnt", "kicked"],
            &["this", "creature", "wasn't", "kicked"],
            &["this", "creature", "was", "not", "kicked"],
        ],
    ) {
        return Some(PredicateAst::Not(Box::new(
            PredicateAst::ThisSpellWasKicked,
        )));
    }
    if surface::exact_any(
        clause,
        &[
            &["this", "card", "is", "suspended"],
            &[
                "this", "card", "is", "in", "exile", "with", "a", "time", "counter", "on", "it",
            ],
        ],
    ) {
        return Some(PredicateAst::And(
            Box::new(PredicateAst::SourceIsInZone(Zone::Exile)),
            Box::new(PredicateAst::SourceHasCounterAtLeast {
                counter_type: CounterType::Time,
                count: 1,
                surface: crate::SourceCounterThresholdSurface::SourceHas,
            }),
        ));
    }
    None
}

fn parse_existing_zone_history_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);

    // Keep the object descriptor owned by the shared filter grammar.  The
    // history clause only owns the passive zone-change frame, so card-type
    // unions and ordinary permanent descriptors do not need one-off entries
    // here.
    let words = clause.word_refs();
    if let Some(put_idx) =
        crate::word_primitives::select_word_position(&words, |word| matches!(word, "was" | "were"))
    {
        let tail = &words[put_idx..];
        let graveyard_owner = if crate::word_primitives::parse_choice_sequence_complete(
            tail,
            &[
                &["was", "were"],
                &["put"],
                &["into"],
                &["a", "the"],
                &["graveyard"],
                &["from"],
                &["the"],
                &["battlefield"],
                &["this"],
                &["turn"],
            ],
        ) {
            None
        } else if crate::word_primitives::parse_choice_sequence_complete(
            tail,
            &[
                &["was", "were"],
                &["put"],
                &["into"],
                &["your"],
                &["graveyard"],
                &["from"],
                &["the"],
                &["battlefield"],
                &["this"],
                &["turn"],
            ],
        ) {
            Some(PlayerFilter::You)
        } else {
            return parse_existing_zone_history_gate_exact(clause);
        };
        let subject = clause.before_word(put_idx)?;
        if let Ok(mut filter) = parse_object_filter_lexed(subject.tokens(), false)
            && filter != ObjectFilter::default()
        {
            if graveyard_owner.is_some() {
                filter.owner = graveyard_owner;
            }
            return Some(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
                filter,
            ));
        }
    }

    parse_existing_zone_history_gate_exact(clause)
}

fn parse_existing_zone_history_gate_exact(clause: LexedClause<'_>) -> Option<PredicateAst> {
    if surface::exact(
        clause,
        &[
            "an",
            "artifact",
            "or",
            "creature",
            "was",
            "put",
            "into",
            "a",
            "graveyard",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    ) {
        let mut filter = ObjectFilter::default();
        filter.card_types = vec![CardType::Artifact, CardType::Creature];
        return Some(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
            filter,
        ));
    }
    if surface::exact(
        clause,
        &[
            "an",
            "enchantment",
            "was",
            "put",
            "into",
            "your",
            "graveyard",
            "from",
            "the",
            "battlefield",
            "this",
            "turn",
        ],
    ) {
        let mut filter = ObjectFilter::enchantment();
        filter.owner = Some(PlayerFilter::You);
        return Some(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
            filter,
        ));
    }
    if surface::exact(
        clause,
        &[
            "a",
            "creature",
            "died",
            "under",
            "an",
            "opponents",
            "control",
            "this",
            "turn",
        ],
    ) {
        return Some(value_at_least(
            Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::Opponent),
            1,
        ));
    }
    if surface::exact(clause, &["no", "creatures", "died", "this", "turn"]) {
        return Some(PredicateAst::Not(Box::new(
            PredicateAst::CreatureDiedThisTurn,
        )));
    }
    None
}

fn parse_player_counter_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_player_counter_condition(tokens)?;
    if condition.counter_type != CounterType::Poison {
        return None;
    }
    let count = comparison_to_at_least_threshold(&condition.comparison)?;
    Some(PredicateAst::PlayerHasPoisonCountersOrMore {
        player: player_ast_from_status_player_filter(condition.player)?,
        count,
    })
}

fn parse_world_status_gate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    surface::exact(LexedClause::new(tokens), &["there", "is", "no", "monarch"]).then(|| {
        PredicateAst::Not(Box::new(PredicateAst::PlayerIsMonarch {
            player: PlayerAst::Any,
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex_line;

    fn parse(text: &str) -> PredicateAst {
        let tokens = lex_line(text, 0).expect("lex predicate");
        parse_phase_step_gate_predicate(&tokens)
            .expect("parse predicate")
            .expect("phase-step predicate")
    }

    #[test]
    fn parses_quantitative_phase_step_gates_into_values() {
        assert!(matches!(
            parse("you have 200 or more cards in your library"),
            PredicateAst::ValueComparison {
                left: Value::CardsInLibrary(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(200),
            }
        ));
        assert!(matches!(
            parse("you have exactly thirteen cards in your hand"),
            PredicateAst::ValueComparison {
                left: Value::CardsInHand(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(13),
            }
        ));
        assert!(matches!(
            parse("your library has no cards in it"),
            PredicateAst::ValueComparison {
                left: Value::CardsInLibrary(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(0),
            }
        ));
    }

    #[test]
    fn parses_source_and_control_phase_step_gates() {
        let PredicateAst::PlayerControls { filter, .. } =
            parse("you control a creature with power greater than its base power")
        else {
            panic!("expected control predicate");
        };
        assert!(filter.power_greater_than_base_power);

        assert!(matches!(
            parse("you control exactly one creature"),
            PredicateAst::PlayerControlsExactly { count: 1, .. }
        ));

        assert!(matches!(
            parse("this creature wasn't kicked"),
            PredicateAst::Not(inner) if matches!(*inner, PredicateAst::ThisSpellWasKicked)
        ));
    }

    #[test]
    fn parses_all_existing_model_phase_step_gate_surfaces() {
        let surfaces = [
            "you have at least 15 life more than your starting life total",
            "you have 200 or more cards in your library",
            "you were dealt 4 or more damage this turn",
            "you discarded a card this turn",
            "an opponent discarded a card this turn",
            "you gained or lost life this turn",
            "there are thirty or more counters among artifacts and creatures you control",
            "your devotion to white and black is seven or greater",
            "you have exactly thirteen cards in your hand",
            "there are no Zombies on the battlefield",
            "there are no Reflection tokens on the battlefield",
            "there are cards exiled with this enchantment",
            "enchanted creature's power is 4 or greater",
            "enchanted Equipment is attached to a creature",
            "you control a creature with power greater than its base power",
            "your opponents control no creatures",
            "your opponents control no permanents with bounty counters on them",
            "all nonland permanents you control are white",
            "this creature didn't enter the battlefield this turn",
            "this creature dealt damage to an opponent this turn",
            "this creature was dealt damage this turn",
            "this card is suspended",
            "this creature wasn't kicked",
            "an artifact or creature was put into a graveyard from the battlefield this turn",
            "an enchantment was put into your graveyard from the battlefield this turn",
            "a creature died under an opponent's control this turn",
            "no creatures died this turn",
            "you created a token this turn",
            "you sacrificed three or more Clues this turn",
            "you sacrificed a permanent this turn",
            "a Phyrexian died under your control this turn",
            "another Human died under your control this turn",
            "no creatures attacked this turn",
            "a permanent was put into your hand from the battlefield this turn",
            "one or more cards were put into exile this turn",
            "a card left your graveyard this turn",
            "a +1/+1 counter was put on a permanent under your control this turn",
            "you put a counter on a creature this turn",
            "you cycled two or more cards this turn",
            "you attacked with a Hero this turn or a Hero entered the battlefield under your control this turn",
            "an opponent has three or more poison counters",
            "there is no monarch",
            // These existing control-count families were previously swallowed
            // by the same trigger-tail fallback.
            "you control one or more Eggs",
            "you control three or more Attractions",
        ];

        for surface in surfaces {
            let tokens = lex_line(surface, 0).expect("lex predicate");
            assert!(
                parse_phase_step_gate_predicate(&tokens)
                    .unwrap_or_else(|error| panic!("{surface}: {error}"))
                    .is_some(),
                "phase-step condition was not modeled: {surface}"
            );
        }
    }

    #[test]
    fn generic_graveyard_card_threshold_keeps_its_untyped_card_filter() {
        let tokens = lex_line("eight or more cards are in your graveyard", 0)
            .expect("lex graveyard predicate");
        assert!(matches!(
            super::parse_predicate(&tokens).expect("parse graveyard predicate"),
            PredicateAst::ValueComparison {
                left: Value::Count(filter),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(8),
            } if filter.zone == Some(Zone::Graveyard)
                && filter.owner == Some(PlayerFilter::You)
                && filter.card_types.is_empty()
        ));
    }

    #[test]
    fn trigger_splitter_attaches_phase_step_conditions_instead_of_swallowing_them() {
        for (text, expected_debug) in [
            (
                "At the beginning of your upkeep, if you have 200 or more cards in your library, you win the game.",
                "CardsInLibrary",
            ),
            (
                "At the beginning of your end step, if you control a creature with power greater than its base power, create a 1/1 white Soldier creature token.",
                "PlayerControls",
            ),
        ] {
            let tokens = lex_line(text, 0).expect("lex triggered line");
            let split =
                crate::grammar::structure::split_triggered_conditional_clause_lexed(&tokens, 1)
                    .unwrap_or_else(|| panic!("trigger condition was swallowed: {text}"));
            let debug = format!("{:?}", split.predicate);
            assert!(debug.contains(expected_debug), "{text}: {debug}");
        }
    }
}
