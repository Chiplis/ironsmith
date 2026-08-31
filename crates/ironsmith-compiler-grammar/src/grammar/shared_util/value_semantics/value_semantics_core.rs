use super::*;

/// Parse noun phrases whose numeric meaning comes from retained turn events.
/// Callers may pass either the bare noun phrase or a leading "for each".
pub fn parse_turn_history_count_value(tokens: &[OwnedLexToken]) -> Option<Value> {
    let mut tokens = trim_edge_punctuation(tokens);
    let leading = TokenWordView::new(&tokens);
    let leading_words = leading.to_word_refs();
    if crate::word_primitives::parse_sequence_prefix(&leading_words, &["for", "each"])
        && let Some(range) = leading.token_span_for_words(2, leading.len())
    {
        tokens = trim_edge_punctuation(&tokens[range]);
    }

    let word_view = TokenWordView::new(&tokens);
    let words = word_view.to_word_refs();
    if words.is_empty() {
        return None;
    }

    let turn_start_untapped_lands_player = if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["untapped"],
            &["land", "lands"],
            &["they"],
            &["controlled"],
            &["at"],
            &["the"],
            &["beginning"],
            &["of"],
            &["this"],
            &["turn"],
        ],
    ) {
        Some(PlayerFilter::IteratedPlayer)
    } else if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["untapped"],
            &["land", "lands"],
            &["you"],
            &["controlled"],
            &["at"],
            &["the"],
            &["beginning"],
            &["of"],
            &["this"],
            &["turn"],
        ],
    ) {
        Some(PlayerFilter::You)
    } else {
        None
    };
    if let Some(player) = turn_start_untapped_lands_player {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::UntappedLandsAtTurnStart(player),
        ));
    }

    if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["attraction", "attractions"],
            &["youve", "you've"],
            &["visited"],
            &["this"],
            &["turn"],
        ],
    ) {
        return Some(Value::AttractionsVisitedThisTurn(PlayerFilter::You));
    }

    if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["time", "times"],
            &["you"],
            &["descended"],
            &["this"],
            &["turn"],
        ],
    ) {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::Descended(
            PlayerFilter::You,
        )));
    }

    // Keep the exact, unqualified creature-death wording on the dedicated
    // value variant. Richer historical filters still use TurnHistoryCount.
    if let Some(value) = parse_creatures_died_this_turn_count_value(&tokens) {
        return Some(value);
    }

    // This composite value ends with the same `spells you've cast this turn`
    // suffix as an ordinary spell-history count. Recognize the whole phrase
    // first so the generic suffix parser does not reinterpret
    // `colors among permanents you control and spells` as an object filter.
    if crate::word_primitives::parse_any_sequence_complete(
        &words,
        &[
            &[
                "colors",
                "among",
                "permanents",
                "you",
                "control",
                "and",
                "spells",
                "youve",
                "cast",
                "this",
                "turn",
            ],
            &[
                "colors",
                "among",
                "permanents",
                "you",
                "control",
                "and",
                "spells",
                "you've",
                "cast",
                "this",
                "turn",
            ],
        ],
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::ColorsAmongPermanentsAndSpellsCast(PlayerFilter::You),
        ));
    }

    if let Some(value) = parse_spell_cast_history_count(&tokens, &word_view, &words) {
        return Some(value);
    }

    for (suffix, controller, default_surface) in [
        (
            &["that", "died", "this", "turn"][..],
            None,
            ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
        ),
        (
            &["that", "died", "under", "your", "control", "this", "turn"][..],
            Some(PlayerFilter::You),
            ironsmith_core::DeathHistoryControllerSurface::DiedUnderControl,
        ),
    ] {
        if let Some(end) = suffix_start(&words, suffix) {
            let mut filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            let has_suffix_controller = controller.is_some();
            if let Some(controller) = controller {
                filter.controller = Some(controller);
            }
            let controller_surface = if !has_suffix_controller && filter.controller.is_some() {
                ironsmith_core::DeathHistoryControllerSurface::ControlledThenDied
            } else {
                default_surface
            };
            return Some(Value::TurnHistoryCount(TurnHistoryCount::Died {
                filter,
                controller_surface,
            }));
        }
    }

    for suffix in [
        &[
            "that",
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ][..],
        &[
            "you",
            "had",
            "enter",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ][..],
        &[
            "you",
            "had",
            "entered",
            "the",
            "battlefield",
            "under",
            "your",
            "control",
            "this",
            "turn",
        ][..],
    ] {
        if let Some(end) = suffix_start(&words, suffix) {
            let mut filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            filter.controller = Some(PlayerFilter::You);
            return Some(Value::TurnHistoryCount(
                TurnHistoryCount::EnteredBattlefield(filter),
            ));
        }
    }

    if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["token", "tokens"],
            &["you", "youve", "you've"],
            &["created"],
            &["this"],
            &["turn"],
        ],
    ) {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::TokensCreated(
            PlayerFilter::You,
        )));
    }

    let discarded_or_cycled_short = words.len() == 7
        && matches!(words.first(), Some(&"card" | &"cards"))
        && matches!(words.get(1), Some(&"youve" | &"you've"))
        && matches!(words.get(3), Some(&"or"))
        && matches!(words.get(5), Some(&"this"))
        && matches!(words.get(6), Some(&"turn"))
        && matches!(
            (words.get(2), words.get(4)),
            (Some(&"cycled"), Some(&"discarded")) | (Some(&"discarded"), Some(&"cycled"))
        );
    let discarded_or_cycled_long = words.len() == 8
        && matches!(words.first(), Some(&"card" | &"cards"))
        && matches!(words.get(1), Some(&"you"))
        && matches!(words.get(2), Some(&"have"))
        && matches!(words.get(4), Some(&"or"))
        && matches!(words.get(6), Some(&"this"))
        && matches!(words.get(7), Some(&"turn"))
        && matches!(
            (words.get(3), words.get(5)),
            (Some(&"cycled"), Some(&"discarded")) | (Some(&"discarded"), Some(&"cycled"))
        );
    if discarded_or_cycled_short || discarded_or_cycled_long {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::DiscardedOrCycled(PlayerFilter::You),
        ));
    }

    let graveyard_put = crate::word_primitives::parse_sequence_start(&words, &["put"]);
    let graveyard_prefix = graveyard_put.and_then(|put| words.get(..put));
    let graveyard_tail = graveyard_put.and_then(|put| words.get(put..));
    let valid_graveyard_card_prefix = matches!(
        graveyard_prefix,
        Some(["card" | "cards"] | ["card" | "cards", "that", "were"])
    );
    if valid_graveyard_card_prefix
        && matches!(
            graveyard_tail,
            Some([
                "put",
                "into",
                "your",
                "graveyard",
                "from",
                "your",
                "hand",
                "or",
                "library",
                "this",
                "turn"
            ])
        )
    {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PutIntoGraveyard {
                owner: PlayerFilter::You,
                from: vec![Zone::Hand, Zone::Library],
            },
        ));
    }
    if valid_graveyard_card_prefix
        && matches!(
            graveyard_tail,
            Some([
                "put",
                "into",
                "their",
                "graveyard",
                "from",
                "anywhere",
                "this",
                "turn"
            ])
        )
    {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PutIntoGraveyard {
                owner: PlayerFilter::IteratedPlayer,
                from: Vec::new(),
            },
        ));
    }

    for suffix in [
        &["youve", "sacrificed", "this", "turn"][..],
        &["you've", "sacrificed", "this", "turn"][..],
        &["you", "have", "sacrificed", "this", "turn"][..],
    ] {
        if let Some(end) = suffix_start(&words, suffix) {
            let filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            return Some(Value::TurnHistoryCount(TurnHistoryCount::Sacrificed {
                player: PlayerFilter::You,
                filter,
            }));
        }
    }

    if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["opponent", "opponents"],
            &["you"],
            &["attacked"],
            &["this"],
            &["turn"],
        ],
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::OpponentsAttacked(PlayerFilter::You),
        ));
    }

    if let Some(end) = suffix_start(&words, &["you", "attacked", "with", "this", "turn"]) {
        let filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::CreaturesAttackedWith {
                player: PlayerFilter::You,
                filter,
            },
        ));
    }

    if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["player", "players"],
            &["who"],
            &["discarded"],
            &["a"],
            &["card"],
            &["this"],
            &["turn"],
        ],
    ) {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::PlayersDiscarded(
            PlayerFilter::Any,
        )));
    }
    if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["opponent", "opponents"],
            &["who", "that"],
            &["was", "were"],
            &["dealt"],
            &["damage"],
            &["this"],
            &["turn"],
        ],
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PlayersDealtDamage(PlayerFilter::Opponent),
        ));
    }
    if crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["opponent", "opponents"],
            &["who", "that"],
            &["was", "were"],
            &["dealt"],
            &["combat"],
            &["damage"],
            &["this"],
            &["turn"],
        ],
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PlayersDealtCombatDamageBy {
                players: PlayerFilter::Opponent,
                sources: ObjectFilter::default(),
            },
        ));
    }
    let opponents_lost_life = crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["opponent", "opponents"],
            &["who"],
            &["lost"],
            &["life"],
            &["this"],
            &["turn"],
        ],
    ) || crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["your"],
            &["opponent", "opponents"],
            &["who"],
            &["lost"],
            &["life"],
            &["this"],
            &["turn"],
        ],
    ) || crate::word_primitives::parse_choice_sequence_complete(
        &words,
        &[
            &["of"],
            &["your"],
            &["opponent", "opponents"],
            &["who"],
            &["lost"],
            &["life"],
            &["this"],
            &["turn"],
        ],
    );
    if opponents_lost_life {
        return Some(Value::TurnHistoryCount(TurnHistoryCount::PlayersLostLife(
            PlayerFilter::Opponent,
        )));
    }

    if crate::word_primitives::parse_sequence_prefix(
        &words,
        &[
            "your",
            "opponents",
            "who",
            "were",
            "dealt",
            "combat",
            "damage",
            "by",
        ],
    ) && crate::word_primitives::parse_sequence_suffix(&words, &["this", "turn"])
    {
        let start = 8;
        let end = words.len().saturating_sub(2);
        let range = word_view.token_span_for_words(start, end)?;
        let sources = crate::grammar::primitives::probe_shape(parse_object_filter(
            &trim_edge_punctuation(&tokens[range]),
            false,
        ))?;
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::PlayersDealtCombatDamageBy {
                players: PlayerFilter::Opponent,
                sources,
            },
        ));
    }

    let outside_hand_suffixes: &[&[&str]] = &[
        &[
            "youve", "cast", "from", "anywhere", "other", "than", "your", "hand", "this", "turn",
        ],
        &[
            "you've", "cast", "from", "anywhere", "other", "than", "your", "hand", "this", "turn",
        ],
        &[
            "you", "have", "cast", "from", "anywhere", "other", "than", "your", "hand", "this",
            "turn",
        ],
        &[
            "youve", "cast", "this", "turn", "from", "anywhere", "other", "than", "your", "hand",
        ],
        &[
            "you've", "cast", "this", "turn", "from", "anywhere", "other", "than", "your", "hand",
        ],
        &[
            "you", "have", "cast", "this", "turn", "from", "anywhere", "other", "than", "your",
            "hand",
        ],
    ];
    for suffix in outside_hand_suffixes {
        if let Some(end) = suffix_start(&words, suffix) {
            let filter = history_filter_from_word_prefix(&tokens, &word_view, end)?;
            return Some(Value::TurnHistoryCount(TurnHistoryCount::SpellsCast {
                player: PlayerFilter::You,
                filter,
                from_zone: None,
                from_outside_hand: true,
                exclude_source: false,
                before_triggering_spell: false,
            }));
        }
    }

    if words.len() >= 8
        && matches!(words.first(), Some(&"+1/+1"))
        && matches!(words.get(1), Some(&"counter") | Some(&"counters"))
        && crate::word_primitives::parse_sequence_suffix(
            &words,
            &["under", "your", "control", "this", "turn"],
        )
        && let Some(put_on) = crate::word_primitives::parse_sequence_start(&words, &["put", "on"])
    {
        let start = put_on + 2;
        let end = words.len().saturating_sub(5);
        let range = word_view.token_span_for_words(start, end)?;
        let mut filter = crate::grammar::primitives::probe_shape(parse_object_filter(
            &trim_edge_punctuation(&tokens[range]),
            false,
        ))?;
        filter.zone = None;
        filter.controller = Some(PlayerFilter::You);
        return Some(Value::TurnHistoryCount(TurnHistoryCount::CountersPutOn {
            counter_type: Some(crate::object::CounterType::PlusOnePlusOne),
            filter,
        }));
    }

    None
}

/// Parse a complete `where X is ...` binding whose value is backed by turn
/// history. This deliberately runs before generic object-count parsing: words
/// such as `graveyard`, `hand`, and `battlefield` describe event provenance in
/// these clauses, not the current zones of objects to count.
pub fn parse_turn_history_value_binding(tokens: &[OwnedLexToken]) -> Option<Value> {
    let tokens = trim_edge_punctuation(tokens);
    let word_view = TokenWordView::new(&tokens);
    let words = word_view.to_word_refs();
    if !crate::word_primitives::parse_sequence_prefix(&words, &["where", "x", "is"]) {
        return None;
    }

    let body_range = word_view.token_span_for_words(3, word_view.len())?;
    let body_tokens = trim_edge_punctuation(&tokens[body_range]);
    let body_view = TokenWordView::new(&body_tokens);
    let body_words = body_view.to_word_refs();

    if crate::word_primitives::parse_choice_sequence_complete(
        &body_words,
        &[
            &["the", "total"],
            &["amount"],
            &["of"],
            &["damage"],
            &["dealt"],
            &["to"],
            &["it"],
            &["this"],
            &["turn"],
        ],
    ) || crate::word_primitives::parse_sequence_complete(
        &body_words,
        &[
            "the", "total", "amount", "of", "damage", "dealt", "to", "it", "this", "turn",
        ],
    ) {
        return Some(Value::TurnHistoryCount(
            TurnHistoryCount::DamageDealtToSource,
        ));
    }

    for prefix in [
        &["the", "number", "of"][..],
        &["number", "of"][..],
        &["equal", "to", "the", "number", "of"][..],
    ] {
        if crate::word_primitives::parse_sequence_prefix(&body_words, prefix) {
            let history_range = body_view.token_span_for_words(prefix.len(), body_view.len())?;
            return parse_turn_history_count_value(&body_tokens[history_range]);
        }
    }

    let plus_word = crate::word_primitives::parse_sequence_start(&body_words, &["plus"])?;
    let history_prefix = body_words.get(plus_word..plus_word + 4)?;
    if !crate::word_primitives::parse_sequence_complete(
        history_prefix,
        &["plus", "the", "number", "of"],
    ) {
        return None;
    }

    let fixed_range = body_view.token_span_for_words(0, plus_word)?;
    let fixed_tokens = trim_edge_punctuation(&body_tokens[fixed_range]);
    let (fixed, used) = parse_number_prefix_lexed(&fixed_tokens)?;
    if used != fixed_tokens.len() {
        return None;
    }

    let history_range =
        body_view.token_span_for_words(plus_word + history_prefix.len(), body_view.len())?;
    let history = parse_turn_history_count_value(&body_tokens[history_range])?;
    Some(Value::Add(
        Box::new(Value::Fixed(fixed as i32)),
        Box::new(history),
    ))
}

pub(super) fn parse_creatures_died_this_turn_count_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let word_view = TokenWordView::new(tokens);
    if words_match_any_phrase(&word_view.to_word_refs(), CREATURES_DIED_THIS_TURN_PHRASES) {
        Some(Value::CreaturesDiedThisTurn)
    } else {
        None
    }
}

pub fn parse_equal_to_number_of_opponents_you_have_value(
    tokens: &[OwnedLexToken],
) -> Option<Value> {
    let clause_words = TokenWordView::new(tokens);
    let clause_refs = clause_words.to_word_refs();
    if value_helper_shapes::starts_equal_to_opponents_you_have(&clause_refs) {
        return Some(
            Value::CountPlayers(PlayerFilter::Opponent)
                .with_surface_hint(ValueSurfaceHint::EqualTo),
        );
    }
    None
}

pub fn starts_explicit_ordered_comparison(
    tokens: &[&str],
    operator: ValueComparisonOperator,
) -> bool {
    match operator {
        ValueComparisonOperator::LessThanOrEqual => matches!(
            tokens,
            ["less", "than", "or", "equal", "to", ..]
                | ["is", "less", "than", "or", "equal", "to", ..]
        ),
        ValueComparisonOperator::GreaterThanOrEqual => matches!(
            tokens,
            ["greater", "than", "or", "equal", "to", ..]
                | ["is", "greater", "than", "or", "equal", "to", ..]
        ),
        _ => false,
    }
}
