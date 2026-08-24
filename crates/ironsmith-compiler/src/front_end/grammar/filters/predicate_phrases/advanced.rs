use super::*;
use crate::effect::ValueComparisonOperator;
use crate::filter::StackObjectKind;

#[path = "advanced/phase_step_gates.rs"]
mod phase_step_gates;

fn turn_history_player_subject(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact_any(clause, &[&["you've"], &["youve"]]) {
        return Some(PlayerAst::You);
    }
    let words = clause.word_refs();
    let subject_end = if words
        .last()
        .is_some_and(|word| matches!(*word, "has" | "have"))
    {
        words.len().saturating_sub(1)
    } else {
        words.len()
    };
    comparison_player_subject_clause(clause.between_words_trimmed(0, subject_end))
}

fn turn_history_zone_clause(clause: LexedClause<'_>) -> Option<Zone> {
    let words = clause.word_refs();
    let (zone_word, prefix) = words.split_last()?;
    if !prefix.is_empty()
        && !prefix
            .iter()
            .all(|word| matches!(*word, "a" | "an" | "the" | "your" | "their" | "its"))
    {
        return None;
    }
    parse_zone_word(zone_word)
}

fn without_this_turn(clause: LexedClause<'_>) -> Option<LexedClause<'_>> {
    let words = clause.word_refs();
    crate::word_primitives::parse_sequence_suffix(&words, &["this", "turn"])
        .then(|| clause.between_words_trimmed(0, words.len().saturating_sub(2)))
}

fn parse_player_cast_spell_from_zone_body(clause: LexedClause<'_>) -> Option<(PlayerAst, Zone)> {
    let words = clause.word_refs();
    let cast = crate::word_primitives::parse_sequence_start(&words, &["cast"])?;
    let player = turn_history_player_subject(clause.between_words_trimmed(0, cast))?;
    let tail = &words[cast + 1..];
    let from = crate::word_primitives::parse_sequence_start(tail, &["from"])?;
    if !crate::word_primitives::parse_any_sequence_complete(
        &tail[..from],
        &[&["spell"], &["a", "spell"]],
    ) {
        return None;
    }
    let zone_start = cast + 1 + from + 1;
    let zone_clause = clause.between_words_trimmed(zone_start, words.len());
    if zone_clause
        .first_word()
        .is_some_and(|word| matches!(word, "your" | "their" | "its"))
    {
        // The existing filtered-spell history path retains possessive zone
        // surfaces such as "your hand". Keep those clauses on that path.
        return None;
    }
    let zone = turn_history_zone_clause(zone_clause)?;
    Some((player, zone))
}

fn parse_activated_ability_of_card_in_zone_tail(clause: LexedClause<'_>) -> Option<Zone> {
    let words = clause.word_refs();
    let prefixes = [
        &["activated", "an", "ability", "of", "a", "card", "in"][..],
        &["activated", "an", "ability", "of", "card", "in"][..],
    ];
    let (prefix, start) = crate::word_primitives::find_any_phrase_start(&words, &prefixes)?;
    if start != 0 {
        return None;
    }
    let prefix_len = prefix.len();
    turn_history_zone_clause(clause.between_words_trimmed(prefix_len, words.len()))
}

fn parse_player_activated_ability_of_card_in_zone_body(
    clause: LexedClause<'_>,
) -> Option<(PlayerAst, Zone)> {
    let words = clause.word_refs();
    let activated = crate::word_primitives::parse_sequence_start(&words, &["activated"])?;
    let player = turn_history_player_subject(clause.between_words_trimmed(0, activated))?;
    let zone = parse_activated_ability_of_card_in_zone_tail(
        clause.between_words_trimmed(activated, words.len()),
    )?;
    Some((player, zone))
}

/// Parse the shared-subject/shared-window history shape
/// "<player> cast a spell from <zone> or activated an ability of a card in
/// <zone> this turn".
fn parse_cast_or_activated_from_zone_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = without_this_turn(LexedClause::new(tokens))?;
    let words = clause.word_refs();
    let or = crate::word_primitives::parse_last_sequence_start(&words, &[OR_WORD])?;
    let (player, cast_zone) =
        parse_player_cast_spell_from_zone_body(clause.between_words_trimmed(0, or))?;
    let activated_zone = parse_activated_ability_of_card_in_zone_tail(
        clause.between_words_trimmed(or + 1, words.len()),
    )?;
    Some(PredicateAst::Or(
        Box::new(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::PlayerCastSpellFromZoneThisTurn {
                player,
                zone: cast_zone,
            },
        )),
        Box::new(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::PlayerActivatedAbilityOfCardInZoneThisTurn {
                player,
                zone: activated_zone,
            },
        )),
    ))
}

fn parse_single_zone_action_this_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = without_this_turn(LexedClause::new(tokens))?;
    if let Some((player, zone)) = parse_player_cast_spell_from_zone_body(clause) {
        return Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::PlayerCastSpellFromZoneThisTurn { player, zone },
        ));
    }
    let (player, zone) = parse_player_activated_ability_of_card_in_zone_body(clause)?;
    Some(PredicateAst::TurnHistory(
        TurnHistoryPredicateAst::PlayerActivatedAbilityOfCardInZoneThisTurn { player, zone },
    ))
}

fn intervening_source_surface(clause: LexedClause<'_>) -> Option<SourceReferenceSurface> {
    let words = clause.word_refs();
    source_reference_surface_for_words(&words)
        .or_else(|| this_source_surface_for_words(&words))
        .or_else(|| {
            surface::exact_any(clause, &[&["he"], &["she"]]).then(|| {
                SourceReferenceSurface::ThisPermanentType(render_token_slice(clause.tokens()))
            })
        })
}

/// Parse event-history predicates used by intervening-if triggers. Each shape
/// lowers to a typed history query; no condition text is discarded when a
/// trigger/effect comma split succeeds.
fn parse_turn_history_intervening_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);

    if let Some(predicate) = parse_cast_or_activated_from_zone_this_turn_predicate(tokens)
        .or_else(|| parse_single_zone_action_this_turn_predicate(tokens))
    {
        return Ok(Some(predicate));
    }

    if surface::exact_words(
        &clause.word_refs(),
        &["it", "enlisted", "a", "creature", "this", "combat"],
    ) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::TriggeringObjectEnlistedThisCombat,
        )));
    }

    // "a player cast two or more spells last turn"
    {
        let atoms = [
            WinnowSequence::subject("player", WinnowCaptureKind::UntilPhrase(&["cast"])),
            WinnowSequence::action("cast", WinnowCaptureKind::WordCount(1)),
            WinnowSequence::amount("amount", WinnowCaptureKind::UntilPhrase(&["spells"])),
            WinnowSequence::object("spells", WinnowCaptureKind::WordCount(1)),
            WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
        ];
        if let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) {
            let player = matched
                .capture_clause("player", clause)
                .expect("player capture");
            let spells = matched
                .capture_clause("spells", clause)
                .expect("spells capture");
            let window = matched
                .capture_clause("window", clause)
                .expect("window capture");
            let amount = matched
                .capture_clause("amount", clause)
                .expect("amount capture");
            if surface::exact_any(player, &[&["a", "player"], &["player"]])
                && surface::exact(spells, &["spells"])
                && is_last_turn_clause(window)
            {
                let (comparison, used) = parse_quantity_comparison_prefix(
                    amount.tokens(),
                    false,
                    false,
                    "spells-cast-last-turn predicate",
                )?;
                if used == amount.tokens().len()
                    && let Some(count) = comparison_to_at_least_threshold(&comparison)
                {
                    return Ok(Some(PredicateAst::TurnHistory(
                        TurnHistoryPredicateAst::SpellsCastLastTurnAtLeast(count),
                    )));
                }
            }
        }
    }

    // "an Assassin crewed it this turn"
    {
        let atoms = [
            WinnowSequence::subject("crewers", WinnowCaptureKind::UntilPhrase(&["crewed"])),
            WinnowSequence::action("crewed", WinnowCaptureKind::WordCount(1)),
            WinnowSequence::object("source", WinnowCaptureKind::UntilPhrase(&["this", "turn"])),
            WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
        ];
        if let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) {
            let source = matched
                .capture_clause("source", clause)
                .expect("source capture");
            let window = matched
                .capture_clause("window", clause)
                .expect("window capture");
            if is_source_reference_clause(source) && is_this_turn_clause(window) {
                let crewers = matched
                    .capture_clause("crewers", clause)
                    .expect("crewers capture");
                let filter = parse_object_filter(crewers.tokens(), false)?;
                return Ok(Some(PredicateAst::TurnHistory(
                    TurnHistoryPredicateAst::SourceCrewedByAtLeast { count: 1, filter },
                )));
            }
        }
    }

    // Passive source lifecycle predicates retain the authored source surface.
    for (action, kind) in [
        (&["was", "cast"][..], 0u8),
        (&["was", "kicked"][..], 1u8),
        (&["entered", "this", "turn"][..], 2u8),
        (&["attacked", "this", "turn"][..], 3u8),
    ] {
        let words = clause.word_refs();
        if words.len() <= action.len() || words[words.len() - action.len()..] != *action {
            continue;
        }
        let Some(subject) = clause.between_word_range(0, words.len() - action.len()) else {
            continue;
        };
        let Some(surface) = intervening_source_surface(subject) else {
            continue;
        };
        let predicate = match kind {
            0 => TurnHistoryPredicateAst::SourceWasCast { surface },
            1 => TurnHistoryPredicateAst::SourceWasKicked { surface },
            2 => TurnHistoryPredicateAst::SourceEnteredBattlefieldThisTurn { surface },
            _ => TurnHistoryPredicateAst::SourceAttackedThisTurn { surface },
        };
        return Ok(Some(PredicateAst::TurnHistory(predicate)));
    }

    let words = clause.word_refs();
    if surface::exact_words(
        &words,
        &["you", "didnt", "cast", "it", "from", "your", "hand"],
    ) || surface::exact_words(
        &words,
        &["you", "did", "not", "cast", "it", "from", "your", "hand"],
    ) {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::TriggeringObjectWasCastFromZone(
                Zone::Hand,
            )),
        ))));
    }
    if surface::exact_words(&words, &["it", "has", "madness"]) {
        return Ok(Some(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter::default()
                .with_alternative_cast(ironsmith_core::AlternativeCastKind::Madness),
        )));
    }
    if surface::exact_words(&words, &["it", "wasnt", "cast"])
        || surface::exact_words(&words, &["it", "was", "not", "cast"])
    {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::TriggeringObjectWasCast),
        ))));
    }
    if surface::exact_words(
        &words,
        &["you", "didnt", "play", "a", "land", "this", "turn"],
    ) || surface::exact_words(
        &words,
        &["you", "did", "not", "play", "a", "land", "this", "turn"],
    ) {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::PlayerPlayedLandThisTurn(
                PlayerAst::You,
            )),
        ))));
    }
    if surface::exact_words(&words, &["it", "didnt", "die"])
        || surface::exact_words(&words, &["it", "did", "not", "die"])
    {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::TriggeringObjectDied),
        ))));
    }
    if surface::exact_words(
        &words,
        &[
            "you", "didnt", "play", "a", "card", "from", "exile", "this", "turn",
        ],
    ) || surface::exact_words(
        &words,
        &[
            "you", "did", "not", "play", "a", "card", "from", "exile", "this", "turn",
        ],
    ) {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::PlayerPlayedCardFromZoneThisTurn {
                player: PlayerAst::You,
                zone: Zone::Exile,
            }),
        ))));
    }
    if surface::exact_words(
        &words,
        &[
            "that", "player", "attacked", "you", "during", "their", "last", "turn",
        ],
    ) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::TriggeringPlayerAttackedControllerLastTurn,
        )));
    }
    if surface::exact_words(&words, &["an", "opponent", "lost", "life", "last", "turn"]) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::PlayerLostLifeLastTurn(PlayerAst::Opponent),
        )));
    }
    if surface::exact_words(&words, &["you", "lost", "life", "last", "turn"]) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::PlayerLostLifeLastTurn(PlayerAst::You),
        )));
    }
    if surface::exact_words(&words, &["your", "team", "gained", "life", "this", "turn"]) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::ControllerTeamGainedLifeThisTurn,
        )));
    }
    if surface::exact_words(&words, &["you", "cast", "them"]) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::SourceWasCastByController {
                surface: SourceReferenceSurface::ThisPermanentType("them".to_string()),
            },
        )));
    }
    if surface::exact_words(
        &words,
        &[
            "none", "of", "them", "were", "cast", "or", "no", "mana", "was", "spent", "to", "cast",
            "them",
        ],
    ) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::TriggeringObjectsNoneWereCastOrNoManaSpent,
        )));
    }
    if surface::exact_words(
        &words,
        &[
            "the", "amount", "of", "mana", "spent", "to", "cast", "it", "was", "less", "than",
            "its", "mana", "value",
        ],
    ) {
        return Ok(Some(PredicateAst::ValueComparison {
            left: Value::ManaSpentToCastTriggeringObject,
            operator: ValueComparisonOperator::LessThan,
            right: Value::ManaValueOf(Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(
                "triggering",
            )))),
        }));
    }
    if surface::exact_words(
        &words,
        &["each", "player", "has", "10", "or", "less", "life"],
    ) || surface::exact_words(
        &words,
        &["each", "player", "has", "ten", "or", "less", "life"],
    ) {
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::AllPlayersLifeAtMost(10),
        )));
    }
    if surface::exact_words(&words, &["it", "isnt", "a", "mana", "ability"])
        || surface::exact_words(&words, &["it", "is", "not", "a", "mana", "ability"])
    {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::TriggeringAbilityIsManaAbility),
        ))));
    }

    // "mana from a Treasure was spent to cast it or activate it"
    if words.len() >= 11
        && crate::word_primitives::parse_sequence_prefix(&words, &["mana", "from"])
        && crate::word_primitives::parse_sequence_suffix(
            &words,
            &["was", "spent", "to", "cast", "it", "or", "activate", "it"],
        )
    {
        let source_word_count = words.len() - 10;
        if let Some(source_clause) = clause.between_word_range(2, 2 + source_word_count) {
            let source_filter = parse_object_filter(source_clause.tokens(), false)?;
            return Ok(Some(PredicateAst::TurnHistory(
                TurnHistoryPredicateAst::ManaFromSourceSpentOnTriggeringAction { source_filter },
            )));
        }
    }

    // Quantification over a different opponent from the controller of the
    // triggering spell's existing object target.
    if words.len() >= 11
        && crate::word_primitives::parse_sequence_prefix(
            &words,
            &["another", "opponent", "controls", "one", "or", "more"],
        )
        && crate::word_primitives::parse_sequence_suffix(
            &words,
            &["that", "spell", "could", "target"],
        )
        && let Some(filter_clause) = clause.between_word_range(6, words.len() - 4)
    {
        let filter = parse_object_filter(filter_clause.tokens(), false)?;
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::AnotherOpponentControlsPotentialTarget { filter },
        )));
    }

    // A blocker-composition gate relative to the attacker from the current
    // CreatureBlocked event. Keep the two filters reusable rather than
    // encoding Wall-specific runtime behavior.
    if surface::exact_words(
        &words,
        &[
            "at",
            "least",
            "one",
            "other",
            "wall",
            "creature",
            "is",
            "blocking",
            "that",
            "creature",
            "and",
            "no",
            "non",
            "wall",
            "creatures",
            "are",
            "blocking",
            "that",
            "creature",
        ],
    ) {
        let required_clause = clause
            .between_word_range(3, 6)
            .expect("validated blocker requirement words");
        let prohibited_clause = clause
            .between_word_range(12, 15)
            .expect("validated blocker prohibition words");
        let required = parse_object_filter(required_clause.tokens(), true)?;
        let prohibited = parse_object_filter(prohibited_clause.tokens(), false)?;
        return Ok(Some(PredicateAst::TurnHistory(
            TurnHistoryPredicateAst::TriggeringAttackerBlockers {
                required,
                required_count: 1,
                prohibited,
            },
        )));
    }
    if surface::exact_words(&words, &["its", "not", "their", "turn"])
        || surface::exact_words(&words, &["it", "is", "not", "their", "turn"])
    {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::TriggeringPlayersTurn {
                definite_player: false,
            }),
        ))));
    }
    if surface::exact_any(
        clause,
        &[
            &["its", "not", "that", "players", "turn"],
            &["it", "isnt", "that", "players", "turn"],
            &["it", "isn't", "that", "players", "turn"],
            &["it", "is", "not", "that", "players", "turn"],
        ],
    ) {
        return Ok(Some(PredicateAst::Not(Box::new(
            PredicateAst::TurnHistory(TurnHistoryPredicateAst::TriggeringPlayersTurn {
                definite_player: true,
            }),
        ))));
    }

    Ok(None)
}

pub(super) fn parse_implicit_subject_and_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["and"])),
        WinnowSequence::word("and"),
        WinnowSequence::modifier("right", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let left_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in and predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in and predicate".to_string())
        })?;
    if left_clause.tokens().is_empty() || right_clause.tokens().is_empty() {
        return Ok(None);
    }
    let Some(right_first) = right_clause.token(0) else {
        return Ok(None);
    };
    let right_starts_with_have = token_word_is(right_first, HAVE_WORD);
    if !right_starts_with_have && !token_word_is(right_first, YOU_WORD) {
        return Ok(None);
    }

    let left = parse_implicit_subject_conjunct(left_clause.tokens())?;
    let right_tokens = if right_starts_with_have {
        let mut tokens = vec![OwnedLexToken::word(
            YOU_WORD.to_string(),
            TextSpan::synthetic(),
        )];
        tokens.extend_from_slice(right_clause.tokens());
        tokens
    } else {
        right_clause.tokens().to_vec()
    };
    let right = parse_implicit_subject_conjunct(&right_tokens)?;
    Ok(Some(PredicateAst::And(Box::new(left), Box::new(right))))
}

fn parse_implicit_subject_conjunct(
    tokens: &[OwnedLexToken],
) -> Result<PredicateAst, CardTextError> {
    // These independently articulated conjunctions most often coordinate a
    // control condition with a hand condition. Parse those leaf families
    // directly so the very large general predicate dispatcher does not need
    // to recurse while its own stack frame is live.
    if let Some(predicate) = parse_player_controls_no_predicate(tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_player_cards_in_hand_predicate(tokens) {
        return Ok(predicate);
    }
    if non_article_token_words_starts_with_any(tokens, YOU_CONTROL_PREFIXES)
        && let Some(predicate) = parse_player_controls_predicate(
            tokens,
            PlayerAst::You,
            Some(PlayerFilter::You),
            2,
            true,
            true,
        )?
    {
        return Ok(predicate);
    }
    parse_predicate(tokens)
}

pub(super) fn parse_while_conjoined_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["while"])),
        WinnowSequence::word("while"),
        WinnowSequence::modifier("right", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let left_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing left side in while predicate".to_string())
        })?;
    let right_clause = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing right side in while predicate".to_string())
        })?;
    if left_clause.tokens().is_empty() || right_clause.tokens().is_empty() {
        return Ok(None);
    }

    let left = parse_predicate(left_clause.tokens())?;
    let right = parse_predicate(right_clause.tokens())?;
    if matches!(
        left,
        PredicateAst::ManaSpentToCastThisSpellAtLeast { .. }
            | PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(_)
            | PredicateAst::SameColorManaSpentToCastThisSpellAtLeast(_)
    ) {
        return Err(CardTextError::ParseError(format!(
            "unsupported mana-spent predicate tail (predicate: '{}')",
            render_token_slice(tokens).trim()
        )));
    }
    Ok(Some(PredicateAst::And(Box::new(left), Box::new(right))))
}

pub(super) fn player_filter_for_turn_value(player: PlayerAst) -> Option<PlayerFilter> {
    match player {
        PlayerAst::You | PlayerAst::Implicit => Some(PlayerFilter::You),
        PlayerAst::Active => Some(PlayerFilter::Active),
        PlayerAst::Any => Some(PlayerFilter::Any),
        PlayerAst::Chosen => Some(PlayerFilter::ChosenPlayer),
        PlayerAst::Defending => Some(PlayerFilter::Defending),
        PlayerAst::Attacking => Some(PlayerFilter::Attacking),
        PlayerAst::MostCardsInHand => Some(PlayerFilter::MostCardsInHand),
        PlayerAst::MostLifeTied => Some(PlayerFilter::MostLifeTied),
        PlayerAst::LowestLifeTied => Some(PlayerFilter::LowestLifeTied),
        PlayerAst::Target => Some(PlayerFilter::target_player()),
        PlayerAst::TargetOpponent => Some(PlayerFilter::target_opponent()),
        PlayerAst::Opponent => Some(PlayerFilter::Opponent),
        PlayerAst::PlayerToYourLeft => Some(PlayerFilter::PlayerToYourLeft),
        PlayerAst::PlayerToYourRight => Some(PlayerFilter::PlayerToYourRight),
        PlayerAst::NotYou => Some(PlayerFilter::NotYou),
        PlayerAst::That => Some(PlayerFilter::IteratedPlayer),
        PlayerAst::ThatPlayerOrTargetController => {
            Some(PlayerFilter::TargetPlayerOrControllerOfTarget)
        }
        PlayerAst::TriggeringSourceController => Some(PlayerFilter::ControllerOf(
            crate::filter::ObjectRef::tagged("triggering_source"),
        )),
        PlayerAst::ItsController | PlayerAst::ItsOwner | PlayerAst::Enchanted => None,
    }
}

pub(super) fn player_ast_from_status_player_filter(player: PlayerFilter) -> Option<PlayerAst> {
    match player {
        PlayerFilter::You => Some(PlayerAst::You),
        PlayerFilter::Any => Some(PlayerAst::Any),
        PlayerFilter::Defending => Some(PlayerAst::Defending),
        PlayerFilter::Attacking => Some(PlayerAst::Attacking),
        PlayerFilter::Opponent => Some(PlayerAst::Opponent),
        PlayerFilter::IteratedPlayer => Some(PlayerAst::That),
        PlayerFilter::Target(base) if *base == PlayerFilter::Opponent => {
            Some(PlayerAst::TargetOpponent)
        }
        PlayerFilter::Target(_) => Some(PlayerAst::Target),
        _ => None,
    }
}

pub(super) fn parse_player_status_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let status = crate::grammar::conditions::parse_player_status_condition(tokens)?;
    match status.status {
        crate::grammar::conditions::PlayerStatusAst::Monarch => {
            Some(PredicateAst::PlayerIsMonarch {
                player: player_ast_from_status_player_filter(status.player)?,
            })
        }
        crate::grammar::conditions::PlayerStatusAst::Initiative => {
            Some(PredicateAst::PlayerHasInitiative {
                player: player_ast_from_status_player_filter(status.player)?,
            })
        }
        crate::grammar::conditions::PlayerStatusAst::MaxSpeed => {
            Some(PredicateAst::ValueComparison {
                left: Value::Speed(status.player),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(4),
            })
        }
    }
}

pub(super) fn parse_world_state_or_timing_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    parse_initiative_choice_predicate_shape(tokens)
        .or_else(|| parse_night_state_predicate_shape(tokens))
        .or_else(|| parse_first_combat_phase_predicate_shape(tokens))
        .or_else(|| parse_source_controllers_main_phase_predicate_shape(tokens))
        .or_else(|| parse_cast_this_spell_during_main_phase_shape(tokens))
}

pub(super) fn parse_empty_battlefield_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let relation = parse_copula_relation_clauses(clause.tokens())?;
    let subject_atoms = [
        WinnowSequence::amount("quantity", WinnowCaptureKind::OneOf(&["no"])),
        WinnowSequence::object(
            "object",
            WinnowCaptureKind::OneOf(&["creature", "creatures"]),
        ),
    ];
    WinnowSequence::new(&subject_atoms).parse_full(relation.subject_clause)?;
    let tail_atoms = [
        WinnowSequence::word("on"),
        WinnowSequence::modifier("zone", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&tail_atoms).parse_full(relation.tail_clause)?;
    let zone = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, relation.tail_clause)?;
    if !is_battlefield_zone_clause(zone) {
        return None;
    }
    Some(PredicateAst::ValueComparison {
        left: Value::Count(ObjectFilter::creature().in_zone(crate::zone::Zone::Battlefield)),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Fixed(0),
    })
}

pub(super) fn is_battlefield_zone_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["battlefield"], &["the", "battlefield"]])
}

pub(super) fn parse_initiative_choice_predicate_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["has"];
    let atoms = [
        WinnowSequence::subject("first_player", WinnowCaptureKind::OneOf(&["you"])),
        WinnowSequence::word("or"),
        WinnowSequence::subject(
            "second_player",
            WinnowCaptureKind::UntilPhrase(action_phrase),
        ),
        WinnowSequence::action(
            "status_verb",
            WinnowCaptureKind::WordCount(action_phrase.len()),
        ),
        WinnowSequence::object("status", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let second_player = matched.capture_clause("second_player", clause)?;
    if !is_player_youre_attacking_clause(second_player) {
        return None;
    }
    let status = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_initiative_status_clause(status) {
        return None;
    }
    Some(PredicateAst::Or(
        Box::new(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::You,
        }),
        Box::new(PredicateAst::PlayerHasInitiative {
            player: PlayerAst::Defending,
        }),
    ))
}

pub(super) fn is_player_youre_attacking_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["player", "youre", "attacking"],
            &["a", "player", "youre", "attacking"],
        ],
    )
}

pub(super) fn is_initiative_status_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["initiative"], &["the", "initiative"]])
}

pub(super) fn parse_night_state_predicate_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula = [WinnowSequence::action(
        "copula",
        WinnowCaptureKind::OneOf(&["is"]),
    )];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::OneOf(&["it", "its"])),
        WinnowSequence::optional(&copula),
        WinnowSequence::object("state", WinnowCaptureKind::OneOf(&["night"])),
    ];
    WinnowSequence::new(&atoms).parse_full(clause)?;
    Some(PredicateAst::ItIsNight)
}

pub(super) fn parse_first_combat_phase_predicate_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let copula = [WinnowSequence::action(
        "copula",
        WinnowCaptureKind::OneOf(&["is"]),
    )];
    let article = [WinnowSequence::word("the")];
    let tail_article = [WinnowSequence::word("the")];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::OneOf(&["it", "its"])),
        WinnowSequence::optional(&copula),
        WinnowSequence::optional(&article),
        WinnowSequence::object("phase", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::word("of"),
        WinnowSequence::optional(&tail_article),
        WinnowSequence::modifier("turn", WinnowCaptureKind::OneOf(&["turn"])),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let phase = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_first_combat_phase_clause(phase) {
        return None;
    }
    Some(PredicateAst::FirstCombatPhaseOfTurn)
}

pub(super) fn is_first_combat_phase_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["first", "combat", "phase"])
}

pub(super) fn parse_cast_this_spell_during_main_phase_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let during_phrase = &["during"];
    let atoms = [
        WinnowSequence::subject("player", WinnowCaptureKind::OneOf(&["you"])),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["cast"])),
        WinnowSequence::object("spell", WinnowCaptureKind::UntilPhrase(during_phrase)),
        WinnowSequence::word("during"),
        WinnowSequence::modifier("phase", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !surface::exact(object, &["this", "spell"]) {
        return None;
    }
    let phase = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !is_your_main_phase_clause(phase) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel(
        "CastDuringYourMainPhase".into(),
    ))
}

pub(super) fn is_your_main_phase_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "main", "phase"])
}

pub(super) fn parse_source_controllers_main_phase_predicate_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    surface::exact_any(
        clause,
        &[
            &["its", "your", "main", "phase"],
            &["it", "is", "your", "main", "phase"],
        ],
    )
    .then_some(PredicateAst::SourceControllersMainPhase)
}

pub(super) fn parse_player_achievement_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let achievement = crate::grammar::conditions::parse_player_achievement_condition(tokens)?;
    let player = player_ast_from_status_player_filter(achievement.player)?;
    let predicate = match achievement.achievement {
        crate::grammar::conditions::PlayerAchievementAst::CitysBlessing => {
            Some(PredicateAst::PlayerHasCitysBlessing { player })
        }
        crate::grammar::conditions::PlayerAchievementAst::CompletedDungeon { dungeon_name } => {
            Some(PredicateAst::PlayerCompletedDungeon {
                player,
                dungeon_name,
            })
        }
        crate::grammar::conditions::PlayerAchievementAst::FullParty => {
            if player == PlayerAst::You {
                Some(PredicateAst::YouHaveFullParty)
            } else {
                None
            }
        }
        crate::grammar::conditions::PlayerAchievementAst::VisitedAttractionThisTurn => {
            Some(PredicateAst::TurnHistory(
                TurnHistoryPredicateAst::PlayerVisitedAttractionThisTurn(player),
            ))
        }
    }?;
    if achievement.negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

pub(super) fn parse_player_cards_in_hand_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    // Detect "... at the beginning of this turn" suffix and/or past-tense "had",
    // both of which select the at-turn-start variants. We rewrite the past-tense
    // verb to present in place (preserving real spans) so the shared captured
    // parser can match, instead of round-tripping through synthetic word tokens.
    let clause = LexedClause::new(tokens);
    let stripped = strip_at_beginning_this_turn_suffix_clause(clause);
    let at_turn_start_suffix = stripped.tokens().len() != clause.tokens().len();
    let base_tokens = stripped.tokens();

    let had_idx = token_index_for_word(base_tokens, "had");
    let at_turn_start = at_turn_start_suffix || had_idx.is_some();

    let mut present_tokens = base_tokens.to_vec();
    if let Some(had_idx) = had_idx {
        present_tokens[had_idx].replace_word("have");
    }

    let condition =
        crate::grammar::conditions::parse_player_cards_in_hand_condition(&present_tokens)?;
    let player_filter = condition.player.clone();
    let player = player_ast_from_status_player_filter(condition.player.clone())?;

    if !at_turn_start && player == PlayerAst::You && condition.is_no_cards_in_hand() {
        return Some(PredicateAst::YouHaveNoCardsInHand);
    }

    match condition.comparison {
        crate::effect::Comparison::GreaterThanOrEqual(count) if count >= 0 => {
            Some(cards_in_hand_or_more(player, count as u32, at_turn_start))
        }
        crate::effect::Comparison::GreaterThan(count) if count >= -1 => Some(
            cards_in_hand_or_more(player, (count + 1) as u32, at_turn_start),
        ),
        crate::effect::Comparison::LessThanOrEqual(count) if count >= 0 => {
            Some(cards_in_hand_or_fewer(player, count as u32, at_turn_start))
        }
        crate::effect::Comparison::LessThan(count) if count > 0 => Some(cards_in_hand_or_fewer(
            player,
            (count - 1) as u32,
            at_turn_start,
        )),
        crate::effect::Comparison::Equal(count)
            if count >= 0
                && !at_turn_start
                && present_tokens
                    .iter()
                    .any(|token| token_word_is(token, "exactly")) =>
        {
            Some(PredicateAst::ValueComparison {
                left: Value::CardsInHand(player_filter),
                operator: crate::effect::ValueComparisonOperator::Equal,
                right: Value::Fixed(count),
            })
        }
        // "you have a card in hand" parses as Equal(1) but means "at least one";
        // map the count-or-more reading so the turn-start variant resolves.
        crate::effect::Comparison::Equal(count) if count >= 0 => {
            Some(cards_in_hand_or_more(player, count as u32, at_turn_start))
        }
        _ => None,
    }
}

pub(super) fn cards_in_hand_or_more(
    player: PlayerAst,
    count: u32,
    at_turn_start: bool,
) -> PredicateAst {
    if at_turn_start {
        PredicateAst::PlayerCardsInHandAtTurnStartOrMore { player, count }
    } else {
        PredicateAst::PlayerCardsInHandOrMore { player, count }
    }
}

pub(super) fn cards_in_hand_or_fewer(
    player: PlayerAst,
    count: u32,
    at_turn_start: bool,
) -> PredicateAst {
    if at_turn_start {
        PredicateAst::PlayerCardsInHandAtTurnStartOrFewer { player, count }
    } else {
        PredicateAst::PlayerCardsInHandOrFewer { player, count }
    }
}

pub(super) fn strip_at_beginning_this_turn_suffix_clause(
    clause: LexedClause<'_>,
) -> LexedClause<'_> {
    for suffix in [
        ["at", "the", "beginning", "of", "this", "turn"].as_slice(),
        ["at", "beginning", "of", "this", "turn"].as_slice(),
    ] {
        let stripped = clause.without_trailing_phrase(suffix);
        if stripped.tokens().len() != clause.tokens().len() {
            return stripped;
        }
    }
    clause
}

pub(super) fn parse_player_life_total_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_player_life_total_condition(tokens)?;
    let (operator, amount) = comparison_to_value_comparison_operator(condition.comparison)?;
    Some(PredicateAst::ValueComparison {
        left: crate::effect::Value::LifeTotal(condition.player),
        operator,
        right: crate::effect::Value::Fixed(amount),
    })
}

pub(super) fn parse_player_life_tie_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_player_life_tie_condition(tokens)?;
    Some(PredicateAst::ValueComparison {
        left: crate::effect::Value::CountPlayers(condition.tied_players),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: crate::effect::Value::Fixed(condition.minimum_players as i32),
    })
}

pub(super) fn parse_player_life_relation_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = crate::grammar::conditions::parse_player_life_relation_condition(tokens)?;
    let player = player_ast_from_status_player_filter(relation.player)?;
    match relation.relation {
        crate::grammar::conditions::PlayerLifeRelationAst::HasMoreLifeThanYou => {
            Some(PredicateAst::PlayerHasMoreLifeThanYou { player })
        }
        crate::grammar::conditions::PlayerLifeRelationAst::HasLessLifeThanYou => {
            Some(PredicateAst::PlayerHasLessLifeThanYou { player })
        }
        crate::grammar::conditions::PlayerLifeRelationAst::HasNoOpponentWithMoreLifeThan => {
            Some(PredicateAst::PlayerHasNoOpponentWithMoreLifeThan { player })
        }
        crate::grammar::conditions::PlayerLifeRelationAst::HasMoreLifeThanEachOtherPlayer => {
            Some(PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer { player })
        }
    }
}

pub(super) fn parse_count_parity_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("count", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("scope", WinnowCaptureKind::UntilPhrase(&["is"])),
        WinnowSequence::word("is"),
        WinnowSequence::action("parity", WinnowCaptureKind::OneOf(&["even", "odd"])),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let count_prefix = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !surface::exact_any(
        count_prefix,
        &[
            &["number", "of"],
            &["count", "of"],
            &["the", "number"],
            &["the", "count"],
        ],
    ) {
        return None;
    }
    let parity = matched.capture_clause("parity", clause)?;
    let even = match parity.token(0)?.parser_text() {
        "even" => true,
        "odd" => false,
        _ => return None,
    };
    let captured_scope = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let scope_tokens = if captured_scope.token(0)?.parser_text() == "of" {
        &captured_scope.tokens()[1..]
    } else {
        captured_scope.tokens()
    };
    let scope = LexedClause::new(scope_tokens);
    let count = match scope {
        scope if surface::exact_any(scope, &[&["permanent"], &["permanents"]]) => {
            crate::static_abilities::AnthemCountExpression::MatchingFilter(
                crate::target::ObjectFilter::permanent(),
            )
        }
        _ => return None,
    };
    Some(PredicateAst::CountParity {
        count,
        even,
        display: Some(format!(
            "the number of {} is {}",
            render_token_slice(scope.tokens()),
            if even { "even" } else { "odd" }
        )),
    })
}

pub(super) fn parse_player_cards_in_hand_relation_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation =
        crate::grammar::conditions::parse_player_cards_in_hand_relation_condition(tokens)?;
    let player = player_ast_from_status_player_filter(relation.player)?;
    match relation.relation {
        crate::grammar::conditions::PlayerCardsInHandRelationAst::HasMoreCardsInHandThanYou => {
            Some(PredicateAst::PlayerHasMoreCardsInHandThanYou { player })
        }
        crate::grammar::conditions::PlayerCardsInHandRelationAst::HasMoreCardsInHandThanEachOtherPlayer => {
            Some(PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer { player })
        }
    }
}

pub(super) fn parse_player_turn_event_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_player_turn_event_condition(tokens)?;
    let (operator, count) = comparison_to_value_comparison_operator(condition.comparison)?;
    let mut left = match condition.event {
        crate::grammar::conditions::PlayerTurnEventAst::CardsDrawn => {
            Value::MaxCardsDrawnThisTurn(condition.player)
        }
        crate::grammar::conditions::PlayerTurnEventAst::LandsEnteredBattlefieldUnderControl => {
            if comparison_to_strict_at_least_threshold(&condition.comparison)
                .is_some_and(|count| count <= 1)
                || matches!(condition.comparison, crate::effect::Comparison::Equal(1))
            {
                let player = player_ast_from_status_player_filter(condition.player)?;
                return Some(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player });
            }
            Value::LandsEnteredBattlefieldThisTurn(condition.player)
        }
    };
    if matches!(left.unhinted(), Value::LandsEnteredBattlefieldThisTurn(_))
        && tokens.iter().any(|token| token.is_word("another"))
    {
        left = left.with_surface_hint(ironsmith_core::ValueSurfaceHint::AnotherLandEnteredThisTurn);
    }

    Some(PredicateAst::ValueComparison {
        left,
        operator,
        right: Value::Fixed(count),
    })
}

pub(super) fn parse_turn_timing_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let subject = [WinnowSequence::subject(
        "subject",
        WinnowCaptureKind::OneOf(&["it", "its"]),
    )];
    let copula = [WinnowSequence::action(
        "copula",
        WinnowCaptureKind::OneOf(&["is", "s"]),
    )];
    let negation = [WinnowSequence::modifier(
        "negation",
        WinnowCaptureKind::OneOf(&["not"]),
    )];
    let atoms = [
        WinnowSequence::optional(&subject),
        WinnowSequence::optional(&copula),
        WinnowSequence::optional(&negation),
        WinnowSequence::object("turn", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    if matched.capture("copula").is_some() && matched.capture("subject").is_none() {
        return None;
    }
    let turn_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_your_turn_clause(turn_clause) {
        return None;
    }
    let predicate = PredicateAst::YourTurn;
    if matched.capture("negation").is_some() {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

pub(super) fn is_your_turn_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "turn"])
}

pub(super) fn parse_opponent_controls_tagged_object_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_control_relation_clauses(tokens, false)?;
    if !is_opponent_controller_clause(relation.subject_clause) {
        return None;
    }
    let mut filter = ObjectFilter {
        controller: Some(PlayerFilter::Opponent),
        ..Default::default()
    };
    match controlled_tagged_object_kind(relation.tail_clause)? {
        ControlledTaggedObjectKind::Permanent => {}
        ControlledTaggedObjectKind::Creature => filter.card_types.push(CardType::Creature),
    }
    Some(PredicateAst::ItMatches(filter))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ControlledTaggedObjectKind {
    Permanent,
    Creature,
}

pub(super) fn is_opponent_controller_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["opponent"], &["an", "opponent"]])
}

pub(super) fn controlled_tagged_object_kind(
    clause: LexedClause<'_>,
) -> Option<ControlledTaggedObjectKind> {
    if surface::exact_any(clause, &[&["it"], &["that", "permanent"]]) {
        return Some(ControlledTaggedObjectKind::Permanent);
    }
    if surface::exact(clause, &["that", "creature"]) {
        return Some(ControlledTaggedObjectKind::Creature);
    }
    None
}

pub(super) fn parse_secret_choices_match_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("choices", WinnowCaptureKind::UntilPhrase(&["match"])),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["match"])),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_secret_choices_subject_clause(subject) {
        return None;
    }
    Some(PredicateAst::SecretChoicesMatch)
}

pub(super) fn is_secret_choices_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["they"], &["those", "choices"]])
}

pub(super) fn parse_vote_result_predicate(
    tokens: &[OwnedLexToken],
    allow_tied: bool,
) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_vote_option_result_predicate(tokens, allow_tied) {
        return Ok(Some(predicate));
    }
    parse_no_vote_objects_matched_predicate(tokens)
}

pub(super) fn parse_x_value_comparison_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if let ["x", "is", tail @ ..] = words.as_slice() {
        let parsed = match tail {
            ["less", "than", "or", "equal", "to", amount] => Some((
                crate::effect::ValueComparisonOperator::LessThanOrEqual,
                parse_named_number(amount)? as i32,
            )),
            ["less", "than", amount] => Some((
                crate::effect::ValueComparisonOperator::LessThan,
                parse_named_number(amount)? as i32,
            )),
            ["greater", "than", "or", "equal", "to", amount] => Some((
                crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                parse_named_number(amount)? as i32,
            )),
            ["greater", "than", amount] => Some((
                crate::effect::ValueComparisonOperator::GreaterThan,
                parse_named_number(amount)? as i32,
            )),
            ["equal", "to", amount] | ["exactly", amount] => Some((
                crate::effect::ValueComparisonOperator::Equal,
                parse_named_number(amount)? as i32,
            )),
            _ => None,
        };
        if let Some((operator, amount)) = parsed {
            return Some(PredicateAst::ValueComparison {
                left: Value::X,
                operator,
                right: Value::Fixed(amount),
            });
        }
    }

    let relation = parse_copula_relation_clauses(tokens)?;
    if !surface::exact(relation.subject_clause, &["x"]) {
        return None;
    }
    let comparison_clause = relation.tail_clause;
    let (comparison, used) =
        parse_quantity_comparison_prefix(comparison_clause.tokens(), false, false, "x comparison")
            .ok()?;
    if used != comparison_clause.tokens().len() {
        return None;
    }
    let (operator, amount) = comparison_to_value_comparison_operator(comparison)?;
    Some(PredicateAst::ValueComparison {
        left: Value::X,
        operator,
        right: Value::Fixed(amount),
    })
}

pub(super) fn parse_controlled_creatures_total_power_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    if !surface::exact_any(
        relation.subject_clause,
        &[
            &["creature", "you", "control"],
            &["creature", "you", "controls"],
            &["creatures", "you", "control"],
            &["creatures", "you", "controls"],
        ],
    ) {
        return None;
    }

    let tail_words = relation.tail_clause.word_refs();
    let mut comparison_words: primitives::WordSliceInput<'_> = tail_words.as_slice();
    primitives::word_slice_exact("total")
        .parse_next(&mut comparison_words)
        .ok()?;
    primitives::word_slice_exact("power")
        .parse_next(&mut comparison_words)
        .ok()?;
    let clause_words = LexedClause::new(tokens).word_refs();
    let (comparison, used) =
        parse_filter_comparison_tokens("power", comparison_words, &clause_words).ok()??;
    if used != comparison_words.len() {
        return None;
    }
    let (operator, amount) = match comparison {
        crate::filter::Comparison::GreaterThan(amount) => {
            (crate::effect::ValueComparisonOperator::GreaterThan, amount)
        }
        crate::filter::Comparison::GreaterThanOrEqual(amount) => (
            crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            amount,
        ),
        crate::filter::Comparison::Equal(amount) => {
            (crate::effect::ValueComparisonOperator::Equal, amount)
        }
        crate::filter::Comparison::LessThan(amount) => {
            (crate::effect::ValueComparisonOperator::LessThan, amount)
        }
        crate::filter::Comparison::LessThanOrEqual(amount) => (
            crate::effect::ValueComparisonOperator::LessThanOrEqual,
            amount,
        ),
        crate::filter::Comparison::NotEqual(amount) => {
            (crate::effect::ValueComparisonOperator::NotEqual, amount)
        }
        crate::filter::Comparison::OneOf(_)
        | crate::filter::Comparison::EqualExpr(_)
        | crate::filter::Comparison::NotEqualExpr(_)
        | crate::filter::Comparison::LessThanExpr(_)
        | crate::filter::Comparison::LessThanOrEqualExpr(_)
        | crate::filter::Comparison::GreaterThanExpr(_)
        | crate::filter::Comparison::GreaterThanOrEqualExpr(_) => return None,
    };
    Some(PredicateAst::ValueComparison {
        left: Value::TotalPower(ObjectFilter::creature().you_control()),
        operator,
        right: Value::Fixed(amount),
    })
}

pub(super) fn parse_value_reference_comparison_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    for comparison_start in 1..tokens.len() {
        let Some((left, left_used)) = parse_value(&tokens[..comparison_start]) else {
            continue;
        };
        if left_used != comparison_start || !is_predicate_reference_value(&left) {
            continue;
        }
        let Some((operator, right_tokens)) =
            crate::grammar::values::parse_value_comparison_tokens(&tokens[comparison_start..])
        else {
            continue;
        };
        let Some((right, right_used)) = parse_value(right_tokens) else {
            continue;
        };
        if right_used != right_tokens.len() {
            continue;
        }
        return Some(PredicateAst::ValueComparison {
            left,
            operator,
            right,
        });
    }
    None
}

pub(super) fn is_predicate_reference_value(value: &Value) -> bool {
    matches!(
        value,
        Value::X
            | Value::Count(_)
            | Value::CountScaled(_, _)
            | Value::CountersOnSource(_)
            | Value::CountersOn(_, _)
            | Value::PowerOf(_)
            | Value::ToughnessOf(_)
            | Value::ManaValueOf(_)
            | Value::SourcePower
            | Value::SourceToughness
            | Value::ManaSpentToCastTriggeringObject
    )
}

pub(super) fn parse_paid_cost_label_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let paid_tail_phrases: &[&[&str]] = &[
        &["cost", "was", "paid"],
        &["cost", "wasnt", "paid"],
        &["cost", "was", "not", "paid"],
    ];
    let atoms = [
        WinnowSequence::object(
            "label",
            WinnowCaptureKind::UntilAnyPhrase(paid_tail_phrases),
        ),
        WinnowSequence::action("paid_tail", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let label_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let mut label_words = label_clause.word_refs();
    if label_words.first().copied() == Some("the") {
        label_words.remove(0);
    }
    let label_words = strip_source_possessive_label_prefix(&label_words);
    let paid_tail = matched.capture_clause("paid_tail", clause)?;
    let negated = paid_cost_tail_is_negated(paid_tail)?;
    let label = if label_words.len() == 3
        && surface::exact_words(&label_words[..1], &["this"])
        && is_this_spell_possessive_word(label_words[1])
    {
        named_paid_cost_label_from_word(label_words[2])?
    } else if label_words.len() == 2 && is_paid_cost_possessive_word(label_words[0]) {
        named_paid_cost_label_from_word(label_words[1])?
    } else if label_words.len() == 1 {
        mana_cost_label_from_words(label_words)
            .or_else(|| named_paid_cost_label_from_word(label_words[0]))?
    } else {
        mana_cost_label_from_words(label_words)?
    };
    let predicate = PredicateAst::ThisSpellPaidLabel(label.into());
    if negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

pub(super) fn paid_cost_tail_is_negated(clause: LexedClause<'_>) -> Option<bool> {
    if surface::prefix(clause, &["cost", "was", "paid"]) {
        return Some(false);
    }
    if surface::prefix_any(
        clause,
        &[&["cost", "wasnt", "paid"], &["cost", "was", "not", "paid"]],
    ) {
        return Some(true);
    }
    None
}

pub(super) fn parse_vote_option_result_predicate(
    tokens: &[OwnedLexToken],
    allow_tied: bool,
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("option", WinnowCaptureKind::UntilPhrase(&["gets"])),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["gets"])),
        WinnowSequence::object("result", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let option = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if option.tokens().is_empty() {
        return None;
    }
    let result = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let option = render_token_slice(option.tokens());
    if surface::exact(result, &["more", "votes"]) {
        return Some(PredicateAst::VoteOptionGetsMoreVotes { option });
    }
    if allow_tied
        && surface::exact_any(
            result,
            &[
                &["more", "votes", "or", "vote", "is", "tied"],
                &["more", "votes", "or", "the", "vote", "is", "tied"],
            ],
        )
    {
        return Some(PredicateAst::VoteOptionGetsMoreVotesOrTied { option });
    }
    None
}

pub(super) fn parse_no_vote_objects_matched_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::amount("quantity", WinnowCaptureKind::OneOf(&["no"])),
        WinnowSequence::object("objects", WinnowCaptureKind::UntilPhrase(&["got", "votes"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let action = matched
        .capture_clause_by_role(WinnowCaptureRole::Action, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing action in vote result predicate".to_string())
        })?;
    if !surface::exact(action, &["got", "votes"]) {
        return Ok(None);
    }
    let objects = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in vote result predicate".to_string())
        })?;
    let filter = parse_object_filter(objects.tokens(), false)?;
    Ok(Some(PredicateAst::NoVoteObjectsMatched { filter }))
}

pub(super) fn parse_spell_context_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_spell_context_condition(tokens)?;
    match condition {
        crate::grammar::conditions::SpellContextConditionAst::ControllerIsPoisoned {
            ..
        } => Some(PredicateAst::TargetSpellControllerIsPoisoned),
        crate::grammar::conditions::SpellContextConditionAst::NoManaSpentToCast {
            ..
        } => Some(PredicateAst::TargetSpellNoManaSpentToCast),
        crate::grammar::conditions::SpellContextConditionAst::YouControlMoreCreaturesThanController {
            ..
        } => Some(PredicateAst::YouControlMoreCreaturesThanTargetSpellController),
    }
}

pub(super) fn parse_player_spell_cast_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let condition =
        crate::grammar::conditions::parse_player_spell_cast_this_turn_condition(tokens)?;
    match condition {
        crate::grammar::conditions::PlayerSpellCastThisTurnConditionAst::CountAtLeast {
            player,
            count,
        } => Some(PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: player_ast_from_status_player_filter(player)?,
            count,
        }),
        crate::grammar::conditions::PlayerSpellCastThisTurnConditionAst::MatchingFilterCountAtLeast {
            player,
            filter,
            count,
        } => Some(PredicateAst::ValueComparison {
            left: Value::SpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source: false,
            },
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(count as i32),
        }),
        crate::grammar::conditions::PlayerSpellCastThisTurnConditionAst::MatchingFilters {
            player,
            filters,
            negated,
        } => {
            let mut predicates = filters.into_iter().map(|filter| {
                PredicateAst::ValueComparison {
                    left: Value::SpellsCastThisTurnMatching {
                        player: player.clone(),
                        filter,
                        exclude_source: false,
                    },
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(1),
                }
            });
            let first = predicates.next()?;
            let predicate = predicates
                .fold(first, |left, right| PredicateAst::And(Box::new(left), Box::new(right)));
            if negated {
                Some(PredicateAst::Not(Box::new(predicate)))
            } else {
                Some(predicate)
            }
        }
    }
}

pub(super) fn parse_player_life_change_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let condition =
        crate::grammar::conditions::parse_player_life_change_this_turn_condition(tokens)?;
    match condition.direction {
        crate::grammar::conditions::PlayerLifeChangeDirectionAst::Gained => {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            Some(PredicateAst::PlayerGainedLifeThisTurnOrMore {
                player: player_ast_from_status_player_filter(condition.player)?,
                count,
            })
        }
        crate::grammar::conditions::PlayerLifeChangeDirectionAst::Lost
            if condition.player == PlayerFilter::Opponent
                && comparison_to_strict_at_least_threshold(&condition.comparison) == Some(1) =>
        {
            Some(PredicateAst::OpponentLostLifeThisTurn)
        }
        crate::grammar::conditions::PlayerLifeChangeDirectionAst::Lost
            if condition.player == PlayerFilter::Any =>
        {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            Some(PredicateAst::AnyPlayerLostLifeThisTurnOrMore { count })
        }
        crate::grammar::conditions::PlayerLifeChangeDirectionAst::Lost => {
            let (operator, count) = comparison_to_value_comparison_operator(condition.comparison)?;
            Some(PredicateAst::ValueComparison {
                left: Value::LifeLostThisTurn(condition.player),
                operator,
                right: Value::Fixed(count),
            })
        }
    }
}

pub(super) fn parse_player_descended_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let player = if surface::exact_any(
        clause,
        &[
            &["you", "descended", "this", "turn"],
            &["youve", "descended", "this", "turn"],
        ],
    ) {
        PlayerAst::You
    } else if surface::exact(clause, &["that", "player", "descended", "this", "turn"]) {
        PlayerAst::That
    } else {
        return None;
    };

    Some(PredicateAst::PlayerDescendedThisTurn { player })
}

pub(super) fn parse_object_death_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let explicit_one_or_more =
        crate::word_primitives::parse_sequence_prefix(&words, &["one", "or", "more"]);
    let condition = crate::grammar::conditions::parse_object_death_this_turn_condition(tokens)?;
    match condition.event {
        crate::grammar::conditions::ObjectDeathThisTurnEventAst::Died => {
            let count = comparison_to_strict_at_least_threshold(&condition.comparison)?;
            if let Some(damager) = condition.damaged_by {
                return Some(PredicateAst::CreatureDealtDamageBySourceDiedThisTurn {
                    victim: condition.filter,
                    damager,
                    count,
                });
            }
            if let Some(player) = condition.under_controller {
                return Some(PredicateAst::ValueComparison {
                    left: Value::CreaturesDiedThisTurnControlledBy(player),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(count as i32),
                });
            }
            if count <= 1 && !explicit_one_or_more {
                Some(PredicateAst::CreatureDiedThisTurn)
            } else {
                Some(PredicateAst::CreatureDiedThisTurnOrMore(count))
            }
        }
        crate::grammar::conditions::ObjectDeathThisTurnEventAst::PutIntoYourGraveyardFromAnywhere => {
            Some(PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn)
        }
    }
}

pub(super) fn parse_player_would_action_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_player_would_action_condition(tokens)?;
    let player = player_ast_from_status_player_filter(condition.player)?;
    match condition.action {
        crate::grammar::conditions::PlayerWouldActionAst::DrawCard => {
            Some(PredicateAst::PlayerWouldDrawCard { player })
        }
        crate::grammar::conditions::PlayerWouldActionAst::Proliferate => {
            Some(PredicateAst::PlayerWouldProliferate { player })
        }
        crate::grammar::conditions::PlayerWouldActionAst::BeginExtraTurn => {
            Some(PredicateAst::PlayerWouldBeginExtraTurn { player })
        }
    }
}

pub(super) fn parse_battlefield_entry_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let condition = crate::grammar::conditions::parse_battlefield_entry_condition(tokens)?;
    match condition {
        crate::grammar::conditions::BattlefieldEntryConditionAst::ObjectEntered {
            filter,
            min_count,
            window:
                crate::grammar::conditions::BattlefieldEntryTurnWindowAst::ThisTurn,
        } => {
            if let Some(count) = min_count.filter(|count| *count > 1) {
                return Some(PredicateAst::ValueComparison {
                    left: Value::TurnHistoryCount(
                        ironsmith_core::TurnHistoryCount::EnteredBattlefield(filter),
                    ),
                    operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                    right: Value::Fixed(count as i32),
                });
            }
            Some(PredicateAst::ObjectEnteredBattlefieldThisTurn(filter))
        }
        crate::grammar::conditions::BattlefieldEntryConditionAst::ObjectEntered {
            filter,
            min_count: _,
            window:
                crate::grammar::conditions::BattlefieldEntryTurnWindowAst::LastTurn,
        } => Some(PredicateAst::ObjectEnteredBattlefieldLastTurn(filter)),
        crate::grammar::conditions::BattlefieldEntryConditionAst::LandEnteredUnderYourControlThisTurn {
            player,
        } => Some(PredicateAst::PlayerHadLandEnterBattlefieldThisTurn { player }),
    }
}

pub(super) fn parse_battlefield_change_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let condition =
        crate::grammar::conditions::parse_battlefield_change_this_turn_condition(tokens)?;
    match condition {
        crate::grammar::conditions::BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefield {
            negated,
        } => {
            let predicate = PredicateAst::PermanentLeftBattlefieldThisTurn;
            if negated {
                Some(PredicateAst::Not(Box::new(predicate)))
            } else {
                Some(predicate)
            }
        }
        crate::grammar::conditions::BattlefieldChangeThisTurnConditionAst::NonlandPermanentLeftBattlefieldOrSpellWarped => {
            Some(PredicateAst::Or(
                Box::new(PredicateAst::NonlandPermanentLeftBattlefieldThisTurn),
                Box::new(PredicateAst::SpellWasWarpedThisTurn),
            ))
        }
        crate::grammar::conditions::BattlefieldChangeThisTurnConditionAst::PermanentLeftBattlefieldUnderYourControl {
            surface,
        } => {
            Some(PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn {
                surface,
            })
        }
        crate::grammar::conditions::BattlefieldChangeThisTurnConditionAst::ObjectPutIntoGraveyardFromBattlefield {
            filter,
        } => Some(PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(filter)),
    }
}

pub(super) fn parse_combat_damage_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    parse_source_dealt_combat_damage_this_turn_shape(tokens)
        .or_else(|| parse_player_dealt_combat_damage_by_subtype_this_turn_shape(tokens))
}

pub(super) fn is_player_object_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["player"], &["a", "player"]])
}

pub(super) fn combat_damage_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact_any(clause, &[&["a", "player"], &["player"]]) {
        return Some(PlayerAst::Any);
    }
    if surface::exact_any(clause, &[&["an", "opponent"], &["opponent"]]) {
        return Some(PlayerAst::Opponent);
    }
    None
}

pub(super) fn single_subtype_word_clause(clause: LexedClause<'_>) -> Option<&str> {
    let words = clause.word_refs();
    let words = strip_leading_article_word_refs(&words);
    (words.len() == 1).then_some(words[0])
}

pub(super) fn is_this_turn_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["this", "turn"])
}

pub(super) fn is_this_combat_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["this", "combat"])
}

pub(super) fn is_attacked_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["attacked"])
}

pub(super) fn is_triggering_attack_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["that", "creature"], &["it"]])
}

pub(super) fn is_other_creatures_this_combat_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["other", "creature", "this", "combat"],
            &["other", "creatures", "this", "combat"],
            &["others", "creature", "this", "combat"],
            &["others", "creatures", "this", "combat"],
        ],
    )
}

pub(super) fn is_source_attacked_or_blocked_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["this", "creature"],
            &["this", "permanent"],
            &["this"],
            &["it"],
        ],
    )
}

pub(super) fn is_attacked_or_blocked_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["attacked", "or", "blocked"])
}

pub(super) fn is_source_did_not_attack_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact(clause, &["this", "creature"])
}

pub(super) fn is_entered_under_your_control_tail_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["or", "come", "under", "your", "control"],
            &["or", "came", "under", "your", "control"],
        ],
    )
}

pub(super) fn parse_source_dealt_combat_damage_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["dealt", "combat", "damage", "to"];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["this", "turn"])),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !surface::exact(subject_clause, &["it"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_player_object_clause(object_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::SourceDealtCombatDamageToPlayerThisTurn)
}

pub(super) fn parse_player_dealt_combat_damage_by_subtype_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["was", "dealt", "combat", "damage", "by"];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
        WinnowSequence::object("subtype", WinnowCaptureKind::UntilPhrase(&["this", "turn"])),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    let player = combat_damage_player_subject_clause(subject_clause)?;
    let subtype_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let subtype_word = single_subtype_word_clause(subtype_clause)?;
    let subtype = parse_subtype_word(subtype_word)?;
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn { player, subtype })
}

pub(super) fn parse_combat_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_negative_attack_history_shape(tokens)
        .or_else(|| parse_you_attacked_this_turn_shape(tokens))
        .or_else(|| parse_triggering_object_had_to_attack_this_combat_shape(tokens))
        .or_else(|| parse_you_attacked_with_n_or_more_creatures_shape(tokens))
        .or_else(|| parse_you_attacked_with_exactly_other_creatures_shape(tokens))
        .or_else(|| parse_source_attacked_or_blocked_this_turn_shape(tokens))
}

/// Negative attack-history gates share the same turn-history predicates as
/// positive raid-style checks. Keep the two subjects distinct: "this
/// creature" asks about the source object, while "you ... with a creature"
/// asks whether the source controller declared any attacker this turn.
pub(super) fn parse_negative_attack_history_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if surface::exact_any(
        clause,
        &[
            &["this", "creature", "didnt", "attack", "this", "turn"],
            &["this", "creature", "did", "not", "attack", "this", "turn"],
        ],
    ) {
        return Some(PredicateAst::Not(Box::new(
            PredicateAst::SourceAttackedThisTurn,
        )));
    }

    if surface::exact_any(
        clause,
        &[
            &[
                "you", "didnt", "attack", "with", "a", "creature", "this", "turn",
            ],
            &[
                "you", "did", "not", "attack", "with", "a", "creature", "this", "turn",
            ],
        ],
    ) {
        return Some(PredicateAst::Not(Box::new(
            PredicateAst::YouAttackedThisTurn,
        )));
    }

    None
}

pub(super) fn parse_you_attacked_this_turn_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["attacked"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_attacked_action_clause(action_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::YouAttackedThisTurn)
}

pub(super) fn parse_triggering_object_had_to_attack_this_combat_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["had", "to", "attack"], &["must", "attack"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !is_triggering_attack_subject_clause(subject_clause) {
            continue;
        }
        let window_clause = matched.capture_clause("window", clause)?;
        if !is_this_combat_clause(window_clause) {
            continue;
        }
        return Some(PredicateAst::TriggeringObjectHadToAttackThisCombat);
    }
    None
}

/// "you attacked with N or more creatures this turn" (Windbrisk Heights)
pub(super) fn parse_you_attacked_with_n_or_more_creatures_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let tail_phrases: &[&[&str]] = &[
        &["or", "more", "creatures", "this", "turn"],
        &["or", "more", "creature", "this", "turn"],
    ];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount("count", WinnowCaptureKind::UntilAnyPhrase(tail_phrases)),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !surface::exact(action_clause, &["attacked", "with"]) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    if used != count_clause.tokens().len() {
        return None;
    }
    Some(PredicateAst::YouAttackedWithNOrMoreCreaturesThisTurn(count))
}

pub(super) fn parse_you_attacked_with_exactly_other_creatures_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let tail_phrases: &[&[&str]] = &[
        &["other", "creature", "this", "combat"],
        &["other", "creatures", "this", "combat"],
        &["others", "creature", "this", "combat"],
        &["others", "creatures", "this", "combat"],
    ];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::amount("count", WinnowCaptureKind::UntilAnyPhrase(tail_phrases)),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !surface::exact(action_clause, &["attacked", "with", "exactly"]) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_other_creatures_this_combat_clause(object_clause) {
        return None;
    }
    let count_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(count_clause.tokens())?;
    if used != count_clause.tokens().len() {
        return None;
    }
    Some(PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(count))
}

pub(super) fn parse_source_attacked_or_blocked_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["attacked", "or", "blocked"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_attacked_or_blocked_subject_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_attacked_or_blocked_action_clause(action_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::SourceAttackedOrBlockedThisTurn)
}

pub(super) fn parse_source_did_not_attack_or_enter_control_this_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["didnt", "attack"]),
        ),
        WinnowSequence::modifier("negation", WinnowCaptureKind::OneOf(&["didnt"])),
        WinnowSequence::action("attack", WinnowCaptureKind::OneOf(&["attack"])),
        WinnowSequence::modifier(
            "enter",
            WinnowCaptureKind::UntilAnyPhrase(&[&["this", "turn"]]),
        ),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_source_did_not_attack_subject_clause(subject_clause) {
        return None;
    }
    let enter_clause = matched.capture_clause("enter", clause)?;
    if !is_entered_under_your_control_tail_clause(enter_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_this_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::And(
        Box::new(PredicateAst::Not(Box::new(
            PredicateAst::SourceAttackedThisTurn,
        ))),
        Box::new(PredicateAst::Not(Box::new(
            PredicateAst::SourceCameUnderYourControlThisTurn,
        ))),
    ))
}

pub(super) fn parse_spell_lifecycle_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_you_cast_source_from_shape(tokens)
        .or_else(|| parse_you_cast_source_shape(tokens))
        .or_else(|| parse_tagged_was_cast_shape(tokens))
        .or_else(|| parse_this_spell_was_cast_from_shape(tokens))
        .or_else(|| parse_no_spells_cast_last_turn_shape(tokens))
        .or_else(|| parse_this_spell_paid_named_label_shape(tokens))
        .or_else(|| parse_target_was_kicked_shape(tokens))
}

pub(super) fn parse_you_cast_source_from_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["from"])),
        WinnowSequence::modifier("origin", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    let action = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let origin = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !is_you_clause(subject)
        || !is_cast_action_clause(action)
        || !is_source_spell_object_clause(object)
    {
        return None;
    }
    let origin_tokens = origin.tokens();
    if !token_slice_first_is(origin_tokens, "from") {
        return None;
    }
    let origin_clause = LexedClause::new(&origin_tokens[1..]);
    if surface::exact(
        origin_clause,
        &["anywhere", "other", "than", "your", "hand"],
    ) {
        return Some(PredicateAst::ThisSpellWasCastFromNonHand);
    }
    let origin_words = origin_clause.word_refs();
    let zone = if origin_words.len() == 2 && origin_words[0] == "your" {
        parse_zone_word(origin_words[1])?
    } else {
        spell_cast_origin_zone_clause(origin_clause)?
    };
    Some(PredicateAst::ThisSpellWasCastFromZone(zone))
}

pub(super) fn is_cast_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["cast"])
}

pub(super) fn is_source_spell_object_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(clause, &[&["it"], &["this", "spell"]])
}

pub(super) fn is_tagged_cast_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["it"],
            &["that", "creature"],
            &["that", "permanent"],
            &["that", "object"],
        ],
    )
}

pub(super) fn is_was_cast_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "cast"])
}

pub(super) fn is_this_spell_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact(clause, &["this", "spell"])
}

pub(super) fn is_was_cast_from_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "cast", "from"])
}

pub(super) fn spell_cast_origin_zone_clause(clause: LexedClause<'_>) -> Option<Zone> {
    if surface::exact(clause, &["anywhere", "other", "than", "your", "hand"]) {
        return None;
    }
    let words = clause.word_refs();
    let words = if words
        .first()
        .is_some_and(|word| is_article(word) || *word == DEFINITE_ARTICLE_WORD)
    {
        &words[1..]
    } else {
        words.as_slice()
    };
    (words.len() == 1)
        .then(|| parse_zone_word(words[0]))
        .flatten()
}

pub(super) fn is_no_amount_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["no"])
}

pub(super) fn is_spell_object_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["spell"], &["spells"]])
}

pub(super) fn is_were_cast_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["was", "cast"], &["were", "cast"]])
}

pub(super) fn is_last_turn_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["last", "turn"])
}

pub(super) fn is_kicked_source_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["this", "spell"],
            &["this", "creature"],
            &["this", "permanent"],
            &["it"],
        ],
    )
}

pub(super) fn is_was_kicked_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "kicked"])
}

pub(super) fn is_bargained_source_clause(clause: LexedClause<'_>) -> bool {
    is_source_spell_object_clause(clause)
}

pub(super) fn is_was_bargained_action_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["was", "bargained"])
}

pub(super) fn is_that_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["that"]) || surface::exact(clause, &["that", "spell"])
}

pub(super) fn parse_you_cast_source_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["cast"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_cast_action_clause(action_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_source_spell_object_clause(object_clause) {
        return None;
    }
    Some(PredicateAst::SourceWasCast)
}

pub(super) fn parse_tagged_was_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(&["was", "cast"])),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_cast_subject_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_cast_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)))
}

pub(super) fn parse_this_spell_was_cast_from_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "cast", "from"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(3)),
        WinnowSequence::object("origin", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_this_spell_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_cast_from_action_clause(action_clause) {
        return None;
    }
    let origin_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if surface::exact(
        origin_clause,
        &["anywhere", "other", "than", "your", "hand"],
    ) {
        return Some(PredicateAst::ThisSpellWasCastFromNonHand);
    }
    let zone = spell_cast_origin_zone_clause(origin_clause)?;
    Some(PredicateAst::ThisSpellWasCastFromZone(zone))
}

pub(super) fn parse_no_spells_cast_last_turn_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::amount("amount", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("object", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::modifier("window", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let amount_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    if !is_no_amount_clause(amount_clause) {
        return None;
    }
    let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_spell_object_clause(object_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_were_cast_action_clause(action_clause) {
        return None;
    }
    let window_clause = matched.capture_clause("window", clause)?;
    if !is_last_turn_clause(window_clause) {
        return None;
    }
    Some(PredicateAst::NoSpellsWereCastLastTurn)
}

pub(super) fn parse_this_spell_paid_named_label_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    parse_this_spell_was_kicked_with_cost_shape(tokens)
        .or_else(|| parse_this_spell_was_kicked_shape(tokens))
        .or_else(|| parse_this_spell_was_bargained_shape(tokens))
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["was", "promised"], false)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["wasnt", "promised"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["wasn't", "promised"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Gift", &["was", "not", "promised"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["was", "paid"], false)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["wasnt", "paid"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["wasn't", "paid"], true)
        })
        .or_else(|| {
            parse_named_spell_label_action_shape(tokens, "Tribute", &["was", "not", "paid"], true)
        })
        .or_else(|| parse_behold_spell_label_shape(tokens))
}

pub(super) fn parse_this_spell_was_kicked_with_cost_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let was_idx = token_index_for_word(tokens, "was")?;
    if !tokens
        .get(was_idx + 1)
        .is_some_and(|token| token.is_word("kicked"))
        || !tokens
            .get(was_idx + 2)
            .is_some_and(|token| token.is_word("with"))
    {
        return None;
    }

    if !is_kicked_source_clause(LexedClause::new(&tokens[..was_idx])) {
        return None;
    }

    let mut cost_start = was_idx + 3;
    if tokens
        .get(cost_start)
        .is_some_and(|token| token.is_word("its") || token.is_word("their"))
    {
        cost_start += 1;
    }
    let kicker_idx = token_index_for_word_from(tokens, "kicker", cost_start)?;
    if kicker_idx + 1 != tokens.len() || cost_start >= kicker_idx {
        return None;
    }

    let parsed_cost = parse_activation_cost_tokens(&tokens[cost_start..kicker_idx]).ok()?;
    let compiler_cost = crate::cst_lowering::recognize_activation_cost_cst(&parsed_cost).ok()?;
    let compiler_cost = compiler_cost.to_core_total_cost();
    let cost_text = compiler_cost
        .mana_cost()
        .map(|cost| cost.to_oracle())
        .unwrap_or_else(|| compiler_cost.display());
    (!cost_text.is_empty())
        .then(|| PredicateAst::ThisSpellPaidLabel(format!("Kicker {cost_text}").into()))
}

pub(super) fn parse_this_spell_was_kicked_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "kicked"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_kicked_source_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_kicked_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::ThisSpellWasKicked)
}

pub(super) fn parse_this_spell_was_bargained_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "bargained"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_bargained_source_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_bargained_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::ThisSpellPaidLabel("Bargain".into()))
}

pub(super) fn parse_named_spell_label_action_shape(
    tokens: &[OwnedLexToken],
    label: &str,
    action_phrase: &[&str],
    negated: bool,
) -> Option<PredicateAst> {
    let words = crate::lexer::token_word_refs(tokens);
    let mut input: primitives::WordSliceInput<'_> = words.as_slice();
    if input
        .first()
        .is_some_and(|word| matches!(*word, "the" | "a" | "an"))
    {
        input = &input[1..];
    }
    let (actual_label, rest) = input.split_first()?;
    if !actual_label.eq_ignore_ascii_case(label) {
        return None;
    }
    input = rest;
    for expected in action_phrase {
        let (actual, rest) = input.split_first()?;
        if actual != expected {
            return None;
        }
        input = rest;
    }
    if !input.is_empty() {
        return None;
    }
    let predicate = PredicateAst::ThisSpellPaidLabel(label.into());
    if negated {
        Some(PredicateAst::Not(Box::new(predicate)))
    } else {
        Some(predicate)
    }
}

pub(super) fn parse_behold_spell_label_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["was", "beheld"], &["beheld"]];
    let atoms = [
        WinnowSequence::object("subtype", WinnowCaptureKind::UntilAnyPhrase(action_phrases)),
        WinnowSequence::any_phrase(action_phrases),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subtype_clause = matched.capture_clause("subtype", clause)?;
    let subtype_tokens = strip_leading_article_tokens(subtype_clause.tokens());
    let subtype_words = LexedClause::new(subtype_tokens).word_refs();
    if subtype_words.len() != 1 {
        return None;
    }
    let subtype = parse_subtype_word(subtype_words[0])?;
    Some(PredicateAst::ThisSpellPaidLabel(
        crate::cost::OptionalCostRef::with_discriminator(
            crate::cost::OptionalCostKind::Behold,
            subtype.to_string(),
        ),
    ))
}

pub(super) fn parse_target_was_kicked_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject(
            "subject",
            WinnowCaptureKind::UntilPhrase(&["was", "kicked"]),
        ),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(2)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_that_clause(subject_clause) {
        return None;
    }
    let action_clause = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    if !is_was_kicked_action_clause(action_clause) {
        return None;
    }
    Some(PredicateAst::TargetWasKicked)
}

pub(super) fn parse_mana_spent_capture_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_mana_from_source_spent_to_cast_shape(tokens)
        .or_else(|| parse_no_mana_spent_to_cast_shape(tokens))
        .or_else(|| parse_no_colored_mana_spent_to_cast_shape(tokens))
        .or_else(|| parse_snow_mana_of_any_spell_color_spent_to_cast_shape(tokens))
        .or_else(|| parse_mana_symbol_spent_to_cast_shape(tokens))
        .or_else(|| {
            parse_same_color_mana_spent_to_cast_predicate(tokens)
                .map(PredicateAst::SameColorManaSpentToCastThisSpellAtLeast)
        })
        .or_else(|| {
            parse_mana_spent_to_cast_predicate(tokens).map(|(amount, symbol)| {
                PredicateAst::ManaSpentToCastThisSpellAtLeast { amount, symbol }
            })
        })
}

fn parse_mana_from_source_spent_to_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens).trimmed();
    let words = clause.word_refs();
    let mana_idx = crate::word_primitives::parse_sequence_start(&words, &["mana"])?;
    if words.len() < mana_idx + 8 || words.get(mana_idx + 1) != Some(&"from") {
        return None;
    }
    let (_, spent_idx) = crate::word_primitives::find_any_phrase_start(
        &words,
        &[
            &["was", "spent", "to", "cast"],
            &["were", "spent", "to", "cast"],
        ],
    )?;
    if spent_idx <= mana_idx + 2
        || !crate::word_primitives::parse_any_sequence_complete(
            &words[spent_idx + 4..],
            &[&["it"], &["that", "spell"], &["this", "spell"]],
        )
    {
        return None;
    }
    let source_clause = clause.between_word_range(mana_idx + 2, spent_idx)?;
    let source_filter = parse_object_filter(source_clause.tokens(), false).ok()?;
    let amount = if mana_idx == 0 {
        1
    } else {
        let amount_clause = clause.between_word_range(0, mana_idx)?;
        let (comparison, used) = parse_quantity_comparison_prefix(
            amount_clause.tokens(),
            false,
            false,
            "mana-source spend predicate",
        )
        .ok()?;
        if used != amount_clause.tokens().len() {
            return None;
        }
        comparison_to_at_least_threshold(&comparison)?
    };
    Some(PredicateAst::ValueComparison {
        left: Value::ManaFromSourceSpentToCastThisSpell {
            source_filter,
            include_source_noun: false,
            reference: ironsmith_core::ManaSpentCastReferenceSurface::It,
        },
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(amount as i32),
    })
}

pub(super) fn parse_no_mana_spent_to_cast_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if !surface::exact_any(
        clause,
        &[
            &["no", "mana", "was", "spent", "to", "cast", "it"],
            &["no", "mana", "were", "spent", "to", "cast", "it"],
            &["no", "mana", "was", "spent", "to", "cast", "this", "spell"],
            &["no", "mana", "were", "spent", "to", "cast", "this", "spell"],
            &["no", "mana", "was", "spent", "to", "cast", "that", "spell"],
            &["no", "mana", "were", "spent", "to", "cast", "that", "spell"],
        ],
    ) {
        return None;
    }
    Some(PredicateAst::Not(Box::new(
        PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: None,
        },
    )))
}

pub(super) fn parse_no_colored_mana_spent_to_cast_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if !surface::exact_any(
        clause,
        &[
            &["no", "colored", "mana", "was", "spent", "to", "cast", "it"],
            &["no", "colored", "mana", "were", "spent", "to", "cast", "it"],
            &[
                "no", "colored", "mana", "was", "spent", "to", "cast", "this", "spell",
            ],
            &[
                "no", "colored", "mana", "were", "spent", "to", "cast", "this", "spell",
            ],
            &[
                "no", "colored", "mana", "was", "spent", "to", "cast", "that", "spell",
            ],
            &[
                "no", "colored", "mana", "were", "spent", "to", "cast", "that", "spell",
            ],
        ],
    ) {
        return None;
    }
    Some(PredicateAst::Not(Box::new(
        PredicateAst::ColoredManaSpentToCastThisSpellAtLeast(1),
    )))
}

pub(super) fn parse_snow_mana_of_any_spell_color_spent_to_cast_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let first = tokens.first()?;
    let symbol = parse_mana_symbol(first.parser_text()).ok()?;
    if symbol != crate::mana::ManaSymbol::Snow {
        return None;
    }

    let clause = LexedClause::new(&tokens[1..]);
    surface::exact_any(
        clause,
        &[
            &[
                "of", "any", "of", "that", "spell", "colors", "was", "spent", "to", "cast", "it",
            ],
            &[
                "of", "any", "of", "that", "spells", "colors", "was", "spent", "to", "cast", "it",
            ],
            &[
                "of", "any", "of", "that", "spell's", "colors", "was", "spent", "to", "cast", "it",
            ],
        ],
    )
    .then_some(PredicateAst::SnowManaOfAnySpellColorSpentToCastThisSpell)
}

pub(super) fn parse_mana_symbol_spent_to_cast_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::amount(
            "symbols",
            WinnowCaptureKind::UntilAnyPhrase(MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES),
        ),
        WinnowSequence::any_phrase(MANA_SPENT_TO_CAST_THIS_SPELL_PHRASES),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let symbol_clause = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let validation_words = mana_spent_symbol_clause_words(symbol_clause);
    if validation_words.is_empty()
        || !validation_words
            .iter()
            .all(|word| word_is_any(word, MANA_SYMBOL_WORDS))
    {
        return None;
    }
    let mut predicates = symbol_clause
        .tokens()
        .iter()
        .filter_map(|token| parse_mana_symbol(token.parser_text()).ok())
        .map(|symbol| PredicateAst::ManaSpentToCastThisSpellAtLeast {
            amount: 1,
            symbol: Some(symbol),
        });
    let first = predicates.next()?;
    Some(predicates.fold(first, |left, right| {
        PredicateAst::And(Box::new(left), Box::new(right))
    }))
}

pub(super) fn parse_attached_tagged_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_this_permanent_attached_to_shape(tokens)
}

pub(super) fn parse_this_permanent_attached_to_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["attached", "to"], &["is", "attached", "to"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::object("attached_to", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !is_this_or_that_permanent_clause(subject_clause) {
            continue;
        }
        let object_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
        let mut filter = parse_object_filter(object_clause.tokens(), false).ok()?;
        if filter.card_types.is_empty() {
            filter.card_types.push(CardType::Creature);
        }
        return Some(PredicateAst::TaggedMatches(
            crate::tag::CompilerReferenceTag::Enchanted.key(),
            filter,
        ));
    }
    None
}

pub(super) fn is_this_or_that_permanent_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["this", "permanent"],
            &["that", "permanent"],
            &["this", "equipment"],
            &["that", "equipment"],
        ],
    )
}

pub(super) fn is_tagged_exiled_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["any", "of", "those", "cards"],
            &["those", "cards"],
            &["that", "card"],
            &["it"],
        ],
    )
}

pub(super) fn is_exiled_zone_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["exiled"])
}

pub(super) fn is_that_permanent_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["that", "permanent"])
}

pub(super) fn is_tagged_entered_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[&["it"], &["that", "card"], &["that", "permanent"]],
    )
}

pub(super) fn is_your_control_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "control"])
}

pub(super) fn is_tagged_creature_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(clause, &[&["it"], &["that", "creature"]])
}

pub(super) fn is_blocking_state_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["blocking"])
}

pub(super) fn is_soulbond_partner_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(clause, &[&["creature"], &["another", "creature"]])
}

pub(super) fn tagged_creature_role_clause(clause: LexedClause<'_>) -> Option<&'static str> {
    if surface::exact(clause, &["equipped", "creature"]) {
        return Some("equipped");
    }
    if surface::exact(clause, &["enchanted", "creature"]) {
        return Some("enchanted");
    }
    None
}

pub(super) fn parse_additional_cost_object_state_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let optional_article = [WinnowSequence::any_word(&["a", "an", "the"])];
    let atoms = [
        WinnowSequence::optional(&optional_article),
        WinnowSequence::action("cost_action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::word("was"),
        WinnowSequence::modifier("descriptor", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let subject = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing subject in sacrificed predicate".to_string())
        })?;
    let Some(subject_token) = subject.token(0) else {
        return Ok(None);
    };
    let cost_action = matched
        .capture_clause_by_role(WinnowCaptureRole::Action, clause)
        .and_then(|action| action.token(0))
        .and_then(|token| match token.parser_text() {
            "sacrificed" => Some(ironsmith_core::AdditionalCostObjectAction::Sacrificed),
            "exiled" => Some(ironsmith_core::AdditionalCostObjectAction::Exiled),
            _ => None,
        });
    let Some(cost_action) = cost_action else {
        return Ok(None);
    };
    let subject_card_type = parse_card_type(subject_token.parser_text())
        .filter(|card_type| is_permanent_type(*card_type));
    let subject_is_permanent =
        token_word_is(subject_token, PERMANENT_WORD) || subject_card_type.is_some();
    if !subject_is_permanent {
        return Ok(None);
    }

    let descriptor = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing descriptor in sacrificed predicate".to_string())
        })?;
    if descriptor.tokens().is_empty() {
        return Ok(None);
    }
    let mut filter = match parse_object_filter(descriptor.tokens(), false) {
        Ok(filter) => filter,
        Err(err) => parse_color_only_object_filter_clause(descriptor).ok_or(err)?,
    };
    if filter.card_types.is_empty()
        && let Some(card_type) = subject_card_type
    {
        filter.card_types.push(card_type);
    }
    let subject_kind = match subject_card_type {
        Some(CardType::Creature) => ironsmith_core::SacrificedObjectKind::Creature,
        Some(CardType::Artifact) => ironsmith_core::SacrificedObjectKind::Artifact,
        Some(CardType::Enchantment) => ironsmith_core::SacrificedObjectKind::Enchantment,
        _ => ironsmith_core::SacrificedObjectKind::Permanent,
    };
    filter.set_additional_cost_object_surface(Some(
        ironsmith_core::AdditionalCostObjectSurface::new(cost_action, subject_kind),
    ));
    Ok(Some(PredicateAst::TaggedMatches(
        TagKey::from(ADDITIONAL_COST_OBJECT_TAG),
        filter,
    )))
}

pub(super) fn parse_tagged_exiled_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["remain"], &["remains"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(action_phrases)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("zone", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_exiled_subject_clause(subject_clause) {
        return None;
    }
    let zone_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_exiled_zone_clause(zone_clause) {
        return None;
    }
    Some(PredicateAst::TaggedMatches(
        TagKey::from(IT_TAG),
        ObjectFilter::default().in_zone(Zone::Exile),
    ))
}

pub(super) fn parse_tagged_state_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    parse_tagged_controlled_permanent_shape(tokens)
        .or_else(|| parse_tagged_entered_under_your_control_shape(tokens))
        .or_else(|| parse_tagged_wasnt_blocking_shape(tokens))
        .or_else(|| parse_implicit_object_present_state_shape(tokens))
        .or_else(|| parse_implicit_object_bare_state_shape(tokens))
        .or_else(|| parse_tagged_historical_identity_shape(tokens))
        .or_else(|| parse_it_soulbond_paired_shape(tokens))
        .or_else(|| parse_tagged_creature_filter_shape(tokens))
}

fn parse_triggering_object_first_tap_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let rest = if words.first() == Some(&"its") {
        &words[1..]
    } else if crate::word_primitives::parse_sequence_prefix(&words, &["it", "is"]) {
        &words[2..]
    } else {
        return None;
    };
    if rest.len() != 10
        || !crate::word_primitives::parse_sequence_prefix(rest, &["the", "first", "time", "that"])
        || !matches!(rest[4], "creature" | "object" | "permanent")
        || !crate::word_primitives::parse_sequence_complete(
            &rest[5..],
            &["has", "become", "tapped", "this", "turn"],
        )
    {
        return None;
    }
    Some(PredicateAst::TriggeringObjectBecameTappedFirstTimeThisTurn)
}

fn parse_triggering_object_first_counters_this_turn_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let words = TokenWordView::new(tokens).to_word_refs();
    let rest = if words.first() == Some(&"its") {
        &words[1..]
    } else if crate::word_primitives::parse_sequence_prefix(&words, &["it", "is"]) {
        &words[2..]
    } else {
        return None;
    };
    if rest.len() != 12
        || !crate::word_primitives::parse_sequence_prefix(rest, &["the", "first", "time"])
        || !crate::word_primitives::parse_sequence_prefix(
            &rest[3..],
            &["counters", "have", "been", "put", "on"],
        )
        || rest[8] != "that"
        || !matches!(rest[9], "creature" | "object" | "permanent")
        || !crate::word_primitives::parse_sequence_complete(&rest[10..], &["this", "turn"])
    {
        return None;
    }
    Some(PredicateAst::TriggeringObjectHadCountersPutFirstTimeThisTurn)
}

pub(super) fn parse_tagged_controlled_permanent_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_control_or_controlled_relation_clauses(tokens)?;
    if !is_you_clause(relation.subject_clause) {
        return None;
    }
    if !is_that_permanent_clause(relation.tail_clause) {
        return None;
    }
    let mut filter = ObjectFilter::default();
    filter.set_demonstrative_antecedent_surface(Some(
        ironsmith_core::DemonstrativeAntecedentSurface::Permanent,
    ));
    Some(PredicateAst::PlayerTaggedObjectMatches {
        player: PlayerAst::You,
        tag: TagKey::from(IT_TAG),
        filter,
        mode: ironsmith_core::TaggedObjectMatchMode::LastKnown,
    })
}

pub(super) fn parse_tagged_entered_under_your_control_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrase = &["entered", "under"];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
        WinnowSequence::object("controller", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_entered_subject_clause(subject_clause) {
        return None;
    }
    let controller_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !is_your_control_clause(controller_clause) {
        return None;
    }
    Some(PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
        player: PlayerAst::You,
        tag: TagKey::from(IT_TAG),
    })
}

pub(super) fn parse_tagged_wasnt_blocking_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["wasnt"], &["wasn't"], &["was", "not"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::object("state", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !is_tagged_creature_subject_clause(subject_clause) {
            continue;
        }
        let state_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
        if !is_blocking_state_clause(state_clause) {
            continue;
        }
        return Some(PredicateAst::TaggedMatches(
            TagKey::from(IT_TAG),
            ObjectFilter {
                nonblocking: true,
                ..Default::default()
            },
        ));
    }
    None
}

pub(super) fn is_implicit_object_state_subject_clause(clause: LexedClause<'_>) -> bool {
    let clause = LexedClause::new(strip_leading_article_tokens(clause.trimmed().tokens()));
    surface::exact_any(
        clause,
        &[
            &["it"],
            &["its"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "object"],
            &["that", "permanent"],
            &["that", "spell"],
        ],
    )
}

pub(super) fn object_filter_has_identity_or_state(filter: &ObjectFilter) -> bool {
    object_filter_has_identity(filter) || object_filter_has_state(filter)
}

pub(super) fn object_filter_has_state(filter: &ObjectFilter) -> bool {
    filter.tapped
        || filter.untapped
        || filter.attacking
        || filter.attacking_alone
        || filter.nonattacking
        || filter.blocking
        || filter.nonblocking
        || filter.blocked
        || filter.unblocked
}

pub(super) fn implicit_object_state_predicate_from_filter(
    filter: ObjectFilter,
    negative: bool,
) -> Option<PredicateAst> {
    if !object_filter_has_identity_or_state(&filter) {
        return None;
    }
    let predicate = PredicateAst::ItMatches(filter);
    Some(if negative {
        PredicateAst::Not(Box::new(predicate))
    } else {
        predicate
    })
}

pub(super) fn parse_implicit_object_present_state_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let state_phrases: &[&[&str]] = &[
        &["is"],
        &["are"],
        &["isnt"],
        &["isn't"],
        &["arent"],
        &["aren't"],
    ];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(state_phrases)),
        WinnowSequence::action(
            "state",
            WinnowCaptureKind::OneOf(&["is", "are", "isnt", "isn't", "arent", "aren't"]),
        ),
        WinnowSequence::object("descriptor", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_implicit_object_state_subject_clause(subject_clause) {
        return None;
    }
    let subject_is_bare_pronoun = surface::exact_any(subject_clause, &[&["it"], &["its"]]);
    let action = matched.capture_clause_by_role(WinnowCaptureRole::Action, clause)?;
    let mut negative = source_identity_copula_is_negative(action);
    let descriptor_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let (descriptor_negative, descriptor_clause) =
        parse_source_identity_descriptor_clause(descriptor_clause)?;
    negative |= descriptor_negative;
    if descriptor_clause.tokens().is_empty()
        || source_identity_descriptor_contains_ignored_state(descriptor_clause)
    {
        return None;
    }
    let descriptor_starts_with_other = descriptor_clause
        .token(0)
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let mut filter = parse_object_filter(descriptor_clause.tokens(), descriptor_starts_with_other)
        .ok()
        .or_else(|| parse_color_only_object_filter_word_refs(descriptor_clause))
        .or_else(|| parse_identity_descriptor_filter_tokens(descriptor_clause.tokens()))?;
    if let Some(surface) = demonstrative_antecedent_surface(subject_clause.tokens()) {
        filter.set_demonstrative_antecedent_surface(Some(surface));
    }
    if subject_is_bare_pronoun && !object_filter_has_state(&filter) {
        return None;
    }
    implicit_object_state_predicate_from_filter(filter, negative)
}

pub(super) fn parse_implicit_object_bare_state_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let state_words = &["attacking", "blocking", "tapped", "untapped"];
    let state_phrases: &[&[&str]] = &[&["attacking"], &["blocking"], &["tapped"], &["untapped"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(state_phrases)),
        WinnowSequence::object("state", WinnowCaptureKind::OneOf(state_words)),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_implicit_object_state_subject_clause(subject_clause) {
        return None;
    }
    let state_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let filter = parse_object_filter(state_clause.tokens(), false).ok()?;
    implicit_object_state_predicate_from_filter(filter, false)
}

pub(super) fn parse_tagged_historical_identity_shape(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["was"], &["were"]];
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::UntilAnyPhrase(action_phrases)),
        WinnowSequence::action("action", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("descriptor", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_tagged_identity_subject_clause(subject_clause) {
        return None;
    }
    let descriptor_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let (negative, descriptor_clause) = parse_source_identity_descriptor_clause(descriptor_clause)?;
    if negative
        || descriptor_clause.tokens().is_empty()
        || source_identity_descriptor_contains_ignored_state(descriptor_clause)
    {
        return None;
    }
    let filter = parse_object_filter(descriptor_clause.tokens(), false)
        .ok()
        .or_else(|| parse_color_only_object_filter_word_refs(descriptor_clause))
        .or_else(|| parse_identity_descriptor_filter_tokens(descriptor_clause.tokens()))?;
    if !object_filter_has_identity(&filter) {
        return None;
    }
    Some(PredicateAst::ItMatchedLastKnown(filter))
}

pub(super) fn is_tagged_identity_subject_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["it"],
            &["that", "card"],
            &["that", "creature"],
            &["that", "permanent"],
        ],
    )
}

pub(super) fn parse_it_soulbond_paired_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let action_phrases: &[&[&str]] = &[&["paired", "with"], &["is", "paired", "with"]];
    for action_phrase in action_phrases {
        let atoms = [
            WinnowSequence::subject("subject", WinnowCaptureKind::UntilPhrase(action_phrase)),
            WinnowSequence::action("action", WinnowCaptureKind::WordCount(action_phrase.len())),
            WinnowSequence::object("partner", WinnowCaptureKind::Rest),
        ];
        let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
            continue;
        };
        let subject_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
        if !surface::exact_any(subject_clause, &[&["it"], &["its"], &["it's"]]) {
            continue;
        }
        let partner_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
        if !is_soulbond_partner_clause(partner_clause) {
            continue;
        }
        return Some(PredicateAst::ItIsSoulbondPaired);
    }
    None
}

pub(super) fn parse_tagged_creature_filter_shape(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("tagged_subject", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("filter", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let tagged_clause = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    let tag = tagged_creature_role_clause(tagged_clause)?;
    let filter_clause = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    let mut filter = parse_object_filter(filter_clause.tokens(), false).ok()?;
    if filter.card_types.is_empty() {
        filter.card_types.push(CardType::Creature);
    }
    Some(PredicateAst::TaggedMatches(TagKey::from(tag), filter))
}

pub(super) fn graveyard_possessive_matches_subject(
    player: PlayerAst,
    possessive: LexedClause<'_>,
) -> bool {
    let Some(token) = possessive.token(0) else {
        return false;
    };
    match player {
        PlayerAst::You | PlayerAst::Implicit => token_word_is(token, YOUR_WORD),
        _ => token_word_is(token, THEIR_WORD),
    }
}

pub(super) fn comparison_player_subject_clause(clause: LexedClause<'_>) -> Option<PlayerAst> {
    let word_len = clause.word_len();
    if word_len == 2 && surface::exact(clause, THAT_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::That)
    } else if word_len == 2 && surface::exact(clause, TARGET_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::Target)
    } else if word_len == 2 && surface::exact(clause, TARGET_OPPONENT_SUBJECT_PREFIX) {
        Some(PlayerAst::TargetOpponent)
    } else if word_len == 2 && surface::exact(clause, EACH_OPPONENT_SUBJECT_PREFIX) {
        Some(PlayerAst::Opponent)
    } else if word_len == 2 && surface::exact_any(clause, A_OR_ANY_PLAYER_SUBJECT_PREFIXES) {
        Some(PlayerAst::Any)
    } else if word_len == 2 && surface::exact(clause, DEFENDING_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::Defending)
    } else if word_len == 2 && surface::exact(clause, ATTACKING_PLAYER_SUBJECT_PREFIX) {
        Some(PlayerAst::Attacking)
    } else if word_len == 1
        && clause
            .token(0)
            .is_some_and(|token| token_word_is(token, YOU_WORD))
    {
        Some(PlayerAst::You)
    } else if surface::exact_any(clause, AN_OR_THE_OPPONENT_SUBJECT_PHRASES)
        || (word_len == 1 && surface::exact_any(clause, OPPONENT_SUBJECT_PREFIXES))
    {
        Some(PlayerAst::Opponent)
    } else if word_len == 1
        && clause
            .token(0)
            .is_some_and(|token| token_word_is(token, PLAYER_SUBJECT_WORD))
    {
        Some(PlayerAst::Any)
    } else {
        None
    }
}

pub(super) fn parse_player_cards_in_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let card_in_phrases: &[&[&str]] = &[&["card", "in"], &["cards", "in"]];
    let atoms = [
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(card_in_phrases),
        ),
        WinnowSequence::any_phrase(card_in_phrases),
        WinnowSequence::modifier("possessive", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::object("zone", WinnowCaptureKind::OneOf(&["graveyard"])),
    ];
    let relation = parse_has_relation_clauses(tokens)?;
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let player = comparison_player_subject_clause(relation.subject_clause)?;
    let quantity =
        matched.capture_clause_by_role(WinnowCaptureRole::Amount, relation.tail_clause)?;
    let (comparison, used) = predicate_quantity_prefix_tokens(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    let possessive = matched.capture_clause("possessive", relation.tail_clause)?;
    if !graveyard_possessive_matches_subject(player, possessive) {
        return None;
    }
    let player_filter = player_filter_for_turn_value(player)?;

    Some(PredicateAst::ValueComparison {
        left: Value::CardsInGraveyard(player_filter),
        operator,
        right: Value::Fixed(count),
    })
}

pub(super) fn parse_quantified_objects_in_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_prepositional_copula_relation_clauses(tokens, &["in"])?;
    if !surface::exact(relation.preposition_clause, &["in"])
        || !is_graveyard_location_clause(relation.tail_clause)
    {
        return None;
    }

    let subject_tokens = relation.subject_clause.tokens();
    let (comparison, used) = predicate_quantity_prefix_tokens(subject_tokens)?;
    if used >= subject_tokens.len() {
        return None;
    }

    let descriptor_tokens = &subject_tokens[used..];
    let mut filter = if descriptor_tokens
        .iter()
        .all(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
    {
        Some(ObjectFilter::default())
    } else {
        parse_object_filter(descriptor_tokens, false).ok()
    }
    .or_else(|| {
        descriptor_tokens
            .last()
            .filter(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
            .and_then(|_| {
                let trimmed = &descriptor_tokens[..descriptor_tokens.len().saturating_sub(1)];
                parse_object_filter(trimmed, false).ok()
            })
    })?;
    filter.zone = Some(Zone::Graveyard);
    if surface::exact(relation.tail_clause, &["your", "graveyard"]) {
        filter.owner = Some(PlayerFilter::You);
    }

    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator,
        right: Value::Fixed(count),
    })
}

pub(super) fn parse_player_controls_more_than_you_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::amount("comparison", WinnowCaptureKind::OneOf(&["more"])),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["than"])),
        WinnowSequence::word("than"),
        WinnowSequence::modifier("comparison_player", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let subject = relation.subject_clause;
    let player = comparison_player_subject_clause(subject)?;
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let tail = matched.capture_clause("comparison_player", relation.tail_clause)?;
    if !is_you_comparison_tail_clause(tail) {
        return None;
    }
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let filter = parse_object_filter(object.tokens(), other).ok()?;
    if filter == ObjectFilter::default() {
        return None;
    }

    Some(PredicateAst::PlayerControlsMoreThanYou { player, filter })
}

pub(super) fn parse_player_controls_fewer_than_you_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::amount("comparison", WinnowCaptureKind::OneOf(&["fewer"])),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["than"])),
        WinnowSequence::word("than"),
        WinnowSequence::modifier("comparison_player", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let player = comparison_player_subject_clause(relation.subject_clause)?;
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let comparison_player = matched.capture_clause("comparison_player", relation.tail_clause)?;
    if !is_you_comparison_tail_clause(comparison_player) {
        return None;
    }
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let mut controlled_filter = parse_object_filter(object.tokens(), other).ok()?;
    if controlled_filter == ObjectFilter::default() {
        return None;
    }
    controlled_filter.controller = Some(player_filter_for_turn_value(player)?);
    let mut your_filter = controlled_filter.clone();
    your_filter.controller = Some(PlayerFilter::You);

    Some(PredicateAst::ValueComparison {
        left: Value::Count(controlled_filter),
        operator: ValueComparisonOperator::LessThan,
        right: Value::Count(your_filter),
    })
}

pub(super) fn parse_player_controls_more_than_each_other_player_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let atoms = [
        WinnowSequence::amount("comparison", WinnowCaptureKind::OneOf(&["more"])),
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["than"])),
        WinnowSequence::word("than"),
        WinnowSequence::modifier("comparison_player", WinnowCaptureKind::Rest),
    ];
    let relation = parse_control_relation_clauses(tokens, false)?;
    let subject = relation.subject_clause;
    let player = comparison_player_subject_clause(subject)?;
    let matched = WinnowSequence::new(&atoms).parse_full(relation.tail_clause)?;
    let tail = matched.capture_clause("comparison_player", relation.tail_clause)?;
    if !surface::exact_any(
        tail,
        &[&["each", "other", "player"], &["each", "other", "players"]],
    ) {
        return None;
    }
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, relation.tail_clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let filter = parse_object_filter(object.tokens(), other).ok()?;
    if filter == ObjectFilter::default() {
        return None;
    }

    Some(PredicateAst::PlayerControlsMoreThanEachOtherPlayer { player, filter })
}

pub(super) fn parse_opponent_controls_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let relation = parse_control_relation_clauses(tokens, false)?;
    if !is_opponent_controller_clause(relation.subject_clause) {
        return None;
    }
    let object = relation.tail_clause;
    if object_starts_with_more_than_clause(object) {
        return None;
    }
    if object.tokens().is_empty() {
        return None;
    }
    let other = object
        .tokens()
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let mut filter = parse_object_filter(object.tokens(), other).ok()?;
    filter.controller = Some(PlayerFilter::Opponent);
    filter.zone = None;

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::Opponent,
        filter,
    })
}

pub(super) fn object_starts_with_more_than_clause(clause: LexedClause<'_>) -> bool {
    let Some(first) = clause.token(0) else {
        return false;
    };
    token_word_is(first, MORE_WORD)
        && clause
            .tokens()
            .iter()
            .skip(1)
            .any(|token| token_word_is(token, THAN_WORD))
}

pub(super) fn is_you_comparison_tail_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["you"], &["you", "do"]])
}

pub(super) fn parse_keyword_subject_object_filter_tokens(
    object_tokens: &[OwnedLexToken],
) -> Result<ObjectFilter, CardTextError> {
    let object_tokens = strip_leading_article_tokens(object_tokens);
    if non_article_token_words_eq_any(object_tokens, NONLAND_CARD_OBJECT_PHRASES) {
        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Land);
        return Ok(filter);
    }

    let normalized_tokens;
    let object_tokens = if object_tokens
        .last()
        .is_some_and(|token| token.parser_text() == "cards")
    {
        normalized_tokens = {
            let mut tokens = object_tokens.to_vec();
            if let Some(last) = tokens.last_mut() {
                *last = OwnedLexToken::synthetic_word("card");
            }
            tokens
        };
        normalized_tokens.as_slice()
    } else {
        object_tokens
    };
    parse_object_filter(object_tokens, false).or_else(|_| {
        let trimmed = if object_tokens
            .last()
            .is_some_and(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
        {
            &object_tokens[..object_tokens.len().saturating_sub(1)]
        } else {
            object_tokens
        };
        parse_object_filter(trimmed, false)
    })
}

pub(super) fn parse_graveyard_escape_keyword_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    const IN_YOUR_GRAVEYARD_PHRASE: &[&str] = &["in", "your", "graveyard"];
    const GRAVEYARD_SUBJECT_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::object(
            "object",
            WinnowCaptureKind::UntilPhrase(IN_YOUR_GRAVEYARD_PHRASE),
        ),
        WinnowSequence::phrase(IN_YOUR_GRAVEYARD_PHRASE),
    ]);

    let Some(relation) = parse_has_relation_clauses(tokens) else {
        return Ok(None);
    };
    if !surface::exact(relation.tail_clause, &["escape"]) {
        return Ok(None);
    }
    let Some(matched) = GRAVEYARD_SUBJECT_PATTERN.parse_full(relation.subject_clause) else {
        return Ok(None);
    };
    let object = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, relation.subject_clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in escape predicate".to_string())
        })?;
    if object.tokens().is_empty() {
        return Ok(None);
    }

    let mut filter = parse_keyword_subject_object_filter_tokens(object.tokens())?;
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);
    filter.alternative_cast = Some(crate::filter::AlternativeCastKind::Escape);
    Ok(Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    }))
}

pub(super) fn parse_player_object_keyword_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    if let Some(predicate) = parse_graveyard_escape_keyword_predicate(tokens)? {
        return Ok(Some(predicate));
    }

    let Some(relation) = parse_has_relation_clauses(tokens) else {
        return Ok(None);
    };
    let subject = relation.subject_clause;
    let keyword = relation.tail_clause;
    let Some((constraint, consumed)) = parse_filter_keyword_constraint_tokens(keyword.tokens())
    else {
        return Ok(None);
    };
    if consumed != keyword.tokens().len() {
        return Ok(None);
    }

    let subject_has_control = subject
        .tokens()
        .iter()
        .any(|token| token_word_is(token, CONTROL_WORD));
    let subject_has_zone = subject
        .tokens()
        .iter()
        .any(|token| token_word_is_any(token, ZONE_WORDS));
    let mut filter = if subject_has_control {
        let object_tokens = subject
            .tokens()
            .iter()
            .filter(|token| {
                !token_word_is(token, YOU_WORD)
                    && !token_word_is_any(token, CONTROL_OR_CONTROLS_WORDS)
            })
            .cloned()
            .collect::<Vec<_>>();
        if object_tokens.is_empty() {
            return Ok(None);
        }
        let mut filter = parse_object_filter(&object_tokens, false)?;
        filter.controller = Some(PlayerFilter::You);
        filter
    } else if subject_has_zone {
        if let Ok(mut filter) = parse_object_filter(subject.tokens(), false) {
            if filter.owner.is_none() {
                filter.owner = Some(PlayerFilter::You);
            }
            filter
        } else if let Some(filter) = parse_keyword_subject_object_in_zone_filter(subject.tokens())?
        {
            filter
        } else {
            return Ok(None);
        }
    } else {
        return Ok(None);
    };

    apply_filter_keyword_constraint(&mut filter, constraint, false);
    Ok(Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    }))
}

pub(super) fn parse_keyword_subject_object_in_zone_filter(
    subject_tokens: &[OwnedLexToken],
) -> Result<Option<ObjectFilter>, CardTextError> {
    const OBJECT_IN_ZONE_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["in"])),
        WinnowSequence::word("in"),
        WinnowSequence::modifier("zone", WinnowCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(subject_tokens);
    let Some(matched) = OBJECT_IN_ZONE_PATTERN.parse_full(clause) else {
        return Ok(None);
    };
    let object = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in keyword-zone predicate".to_string())
        })?;
    let zone = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing zone in keyword-zone predicate".to_string())
        })?;
    if object.tokens().is_empty() || zone.tokens().is_empty() {
        return Ok(None);
    }
    let Ok(mut filter) = parse_keyword_subject_object_filter_tokens(object.tokens()) else {
        return Ok(None);
    };
    if is_your_graveyard_clause(zone) {
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
    } else {
        return Ok(None);
    }
    Ok(Some(filter))
}

pub(super) fn is_your_graveyard_clause(clause: LexedClause<'_>) -> bool {
    surface::exact(clause, &["your", "graveyard"])
}

pub(super) fn is_there_are_or_were_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["there", "are"], &["there", "were"]])
}

pub(super) fn permanents_you_control_scope(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    if surface::exact_any(clause, PERMANENTS_YOU_CONTROL_SCOPE_PHRASES) {
        return Some(ObjectFilter::permanent().you_control());
    }
    None
}

pub(super) fn cards_in_your_graveyard_scope(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    if surface::exact_any(clause, CARDS_IN_YOUR_GRAVEYARD_SCOPE_PHRASES) {
        return Some(
            ObjectFilter::default()
                .in_zone(Zone::Graveyard)
                .owned_by(PlayerFilter::You),
        );
    }
    None
}

pub(super) fn permanents_and_your_graveyard_scope(clause: LexedClause<'_>) -> Option<ObjectFilter> {
    let word_len = clause.word_len();
    let battlefield_end = (3..=word_len.min(4)).find(|end| {
        clause
            .between_word_range(0, *end)
            .and_then(permanents_you_control_scope)
            .is_some()
    })?;
    let connector_tail = clause.between_word_range(battlefield_end, battlefield_end + 1);
    let split_tail = clause.between_word_range(battlefield_end, battlefield_end + 2);
    let connector_end = if connector_tail
        .is_some_and(|tail| surface::exact(tail, PERMANENTS_AND_OR_GRAVEYARD_CONNECTOR_PHRASE))
    {
        battlefield_end + 1
    } else if split_tail
        .is_some_and(|tail| surface::exact(tail, PERMANENTS_AND_OR_SPLIT_CONNECTOR_PHRASE))
    {
        battlefield_end + 2
    } else {
        return None;
    };
    let battlefield = permanents_you_control_scope(clause.between_word_range(0, battlefield_end)?)?;
    let graveyard =
        cards_in_your_graveyard_scope(clause.between_word_range(connector_end, word_len)?)?;
    let mut filter = ObjectFilter::default();
    filter.any_of = vec![battlefield, graveyard];
    Some(filter)
}

pub(super) fn parse_colors_among_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(&[&["color"], &["colors"]]),
        ),
        WinnowSequence::object("unit", WinnowCaptureKind::OneOf(&["color", "colors"])),
        WinnowSequence::word("among"),
        WinnowSequence::modifier("scope", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_there_are_or_were_clause(existential) {
        return None;
    }

    let quantity = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = parse_number(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }

    let scope = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let filter = permanents_you_control_scope(scope)?;
    Some(PredicateAst::ValueComparison {
        left: Value::ColorsAmong(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

pub(super) fn parse_card_types_among_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let card_type_phrases: &[&[&str]] = &[
        &["card", "type"],
        &["card", "types"],
        &["cards", "type"],
        &["cards", "types"],
    ];
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(card_type_phrases),
        ),
        WinnowSequence::any_phrase(card_type_phrases),
        WinnowSequence::word("among"),
        WinnowSequence::modifier("scope", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_there_are_or_were_clause(existential) {
        return None;
    }

    let quantity = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = predicate_at_least_quantity_prefix_tokens(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }

    let scope = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let filter = if surface::exact_any(scope, SACRIFICED_PERMANENTS_SCOPE_PHRASES) {
        ObjectFilter::tagged("sacrificed_0")
    } else {
        permanents_and_your_graveyard_scope(scope)?
    };

    Some(PredicateAst::ValueComparison {
        left: Value::CardTypesAmong(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

pub(super) fn parse_life_total_at_least_starting_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    if non_article_token_words_eq_phrase(tokens, LIFE_TOTAL_AT_LEAST_STARTING_PHRASE) {
        return Some(PredicateAst::ValueComparison {
            left: Value::LifeTotal(PlayerFilter::You),
            operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::StartingLifeTotal(PlayerFilter::You),
        });
    }
    None
}

pub(super) fn parse_life_total_at_least_last_noted_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    if !non_article_token_words_eq_any(tokens, LIFE_TOTAL_AT_LEAST_LAST_NOTED_PHRASES) {
        return None;
    }
    Some(PredicateAst::ValueComparison {
        left: Value::LifeTotal(PlayerFilter::You),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::LastNotedLifeTotal,
    })
}

pub(super) fn parse_counted_objects_have_counter_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    let counted_object = relation.subject_clause;
    let (comparison, used) = predicate_quantity_prefix_tokens(counted_object.tokens())?;
    let count = comparison_to_strict_at_least_threshold(&comparison)?;
    if used >= counted_object.tokens().len() {
        return None;
    }

    let object_tokens = &counted_object.tokens()[used..];
    if object_tokens.is_empty() {
        return None;
    }
    let counter = relation.tail_clause;
    let (counter_constraint, consumed) = parse_counted_object_counter_constraint_clause(counter)?;
    if consumed != counter.tokens().len() {
        return None;
    }

    let other = object_tokens
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let mut filter = parse_object_filter(object_tokens, other).ok()?;
    filter.with_counter = Some(counter_constraint);
    if filter.zone.is_none()
        && filter.card_types.iter().any(|card_type| {
            matches!(
                card_type,
                CardType::Artifact
                    | CardType::Creature
                    | CardType::Enchantment
                    | CardType::Land
                    | CardType::Planeswalker
                    | CardType::Battle
            )
        })
    {
        filter.zone = Some(Zone::Battlefield);
    }

    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    })
}

pub(super) fn parse_counted_object_counter_constraint_clause(
    clause: LexedClause<'_>,
) -> Option<(crate::filter::CounterConstraint, usize)> {
    if clause.tokens().is_empty() {
        return None;
    }
    let words = TokenWordView::new(clause.tokens());
    let constraint_words = words.word_refs();
    if let Some((counter_constraint, consumed_words)) =
        parse_filter_counter_constraint_words(&constraint_words)
    {
        let trailing_words = &constraint_words[consumed_words..];
        if crate::word_primitives::parse_any_sequence_complete(
            trailing_words,
            &[&["on", "it"], &["on", "them"]],
        ) {
            return Some((counter_constraint, clause.tokens().len()));
        }
        let consumed_tokens = words.token_index_after_words(consumed_words)?;
        return Some((counter_constraint, consumed_tokens));
    }

    let counter_type = parse_counter_type_from_tokens(clause.tokens())?;
    Some((
        ironsmith_core::CounterConstraint::Typed(counter_type),
        clause.tokens().len(),
    ))
}

#[rustfmt::skip]
pub(super) fn parse_counted_source_exiled_objects_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let relation = parse_has_relation_clauses(tokens)?;
    let counted_object = relation.subject_clause;
    let (comparison, used) = predicate_quantity_prefix_tokens(counted_object.tokens())?;
    let (operator, count) = comparison_to_value_comparison_operator(comparison)?;
    if used >= counted_object.tokens().len() {
        return None;
    }

    let tail = relation.tail_clause;
    if !surface::prefix_any(tail, BEEN_EXILED_WITH_THIS_SOURCE_PREFIXES) {
        return None;
    }

    let object_tokens = &counted_object.tokens()[used..];
    let mut filter = if object_tokens
        .iter()
        .all(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
    {
        ObjectFilter::default()
    } else {
        parse_object_filter(object_tokens, false).ok()?
    };
    filter.zone = Some(Zone::Exile);
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(crate::tag::SOURCE_EXILED_TAG),
        relation: TaggedOpbjectRelation::IsTaggedObject,
    });

    Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator,
        right: Value::Fixed(count),
    })
}

pub(super) fn parse_happily_style_conjoined_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let cleaned_tokens: Vec<OwnedLexToken> = tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Comma)
        .cloned()
        .collect();
    let cleaned_clause = LexedClause::new(&cleaned_tokens);
    let words = cleaned_clause.word_refs();
    let second_there_idx = surface::find_words(&words[1..], THERE_ARE_PREFIX).map(|idx| idx + 1)?;
    let life_word_idx =
        surface::find_words(&words[second_there_idx + 1..], AND_YOUR_LIFE_TOTAL_PHRASE)
            .map(|idx| idx + second_there_idx + 1)?;
    let life_idx = cleaned_clause
        .words()
        .token_span_for_words(life_word_idx, life_word_idx + 1)?
        .start;

    let first_clause = cleaned_clause.between_word_range(0, second_there_idx)?;
    let second_clause = cleaned_clause.between_word_range(second_there_idx, life_word_idx)?;

    let first = parse_colors_among_predicate(first_clause.tokens())?;
    let second = parse_card_types_among_predicate(second_clause.tokens())?;
    let third = parse_life_total_at_least_starting_predicate(&cleaned_tokens[life_idx + 1..])?;

    Some(PredicateAst::And(
        Box::new(PredicateAst::And(Box::new(first), Box::new(second))),
        Box::new(third),
    ))
}

pub(super) fn parse_revealed_or_controlled_subtype_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let suffix_phrase = &["as", "you", "cast", "this", "spell"];
    let atoms = [
        WinnowSequence::subject("revealer", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("reveal_action", WinnowCaptureKind::OneOf(&["revealed"])),
        WinnowSequence::object(
            "revealed_subtype",
            WinnowCaptureKind::UntilPhrase(&["card"]),
        ),
        WinnowSequence::word("card"),
        WinnowSequence::word("or"),
        WinnowSequence::action(
            "control_action",
            WinnowCaptureKind::OneOf(&["control", "controlled", "controls"]),
        ),
        WinnowSequence::object("controlled_subtype", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let revealer = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_you_clause(revealer) {
        return None;
    }

    let revealed_subtype = matched.capture_clause("revealed_subtype", clause)?;
    let controlled_subtype = matched.capture_clause("controlled_subtype", clause)?;
    let revealed_subtype = single_subtype_descriptor_clause(revealed_subtype, &[])?;
    let controlled_subtype = single_subtype_descriptor_clause(controlled_subtype, suffix_phrase)?;
    let revealed_token = revealed_subtype.token(0)?;
    let controlled_token = controlled_subtype.token(0)?;
    if revealed_token.parser_text() != controlled_token.parser_text() {
        return None;
    }
    let subtype = parse_subtype_word(revealed_token.parser_text())?;

    Some(PredicateAst::Or(
        Box::new(PredicateAst::ThisSpellPaidLabel(
            crate::cost::OptionalCostRef::with_discriminator(
                crate::cost::OptionalCostKind::Behold,
                subtype.to_string(),
            ),
        )),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::default().with_subtype(subtype),
        }),
    ))
}

pub(super) fn single_subtype_descriptor_clause<'a>(
    clause: LexedClause<'a>,
    optional_suffix: &[&str],
) -> Option<LexedClause<'a>> {
    let mut tokens = clause.trimmed().tokens();
    if !optional_suffix.is_empty()
        && let Some(without_suffix) = primitives::strip_lexed_suffix_phrase(tokens, optional_suffix)
    {
        tokens = without_suffix;
    }
    let descriptor = strip_leading_article_tokens(tokens);
    if descriptor.len() != 1 {
        return None;
    }
    parse_subtype_word(descriptor[0].parser_text())?;
    Some(LexedClause::new(descriptor))
}

pub(super) fn is_card_graveyard_existential_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(clause, &[&["there", "is"], &["there", "are"]])
}

pub(super) fn is_graveyard_location_clause(clause: LexedClause<'_>) -> bool {
    surface::exact_any(
        clause,
        &[
            &["your", "graveyard"],
            &["graveyard"],
            &["the", "graveyard"],
        ],
    )
}

pub(super) fn parse_subtype_card_descriptor_clause(
    clause: LexedClause<'_>,
) -> Option<ObjectFilter> {
    let descriptor_tokens = strip_leading_article_tokens(clause.trimmed().tokens());
    if descriptor_tokens.len() != 2
        || !token_word_is_any(&descriptor_tokens[1], CARD_OR_CARDS_WORDS)
    {
        return None;
    }

    let subtype = descriptor_tokens[0]
        .as_word()
        .and_then(parse_subtype_word)?;
    Some(ObjectFilter::default().with_subtype(subtype))
}

/// Parse independently articulated existential objects that share a graveyard
/// suffix, such as "there is an instant card and a sorcery card in your
/// graveyard" or "an instant card and a sorcery card are in your graveyard."
/// The repeated articles make this two existential requirements, not one
/// disjunctive type filter.
pub(super) fn parse_conjoined_cards_in_your_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let outer_atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("descriptors", WinnowCaptureKind::UntilPhrase(&["in"])),
        WinnowSequence::action("preposition", WinnowCaptureKind::OneOf(&["in"])),
        WinnowSequence::modifier("location", WinnowCaptureKind::Rest),
    ];
    let descriptors = if let Some(location_idx) =
        surface::find(clause, &["are", "in", "your", "graveyard"])
        && location_idx + 4 == clause.word_len()
    {
        clause
            .before_word(location_idx)
            .ok_or_else(|| CardTextError::ParseError("missing existential objects".to_string()))?
    } else if let Some(outer) = WinnowSequence::new(&outer_atoms).parse_full(clause) {
        let existential = outer
            .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
            .ok_or_else(|| CardTextError::ParseError("missing existential subject".to_string()))?;
        if !is_card_graveyard_existential_clause(existential) {
            return Ok(None);
        }
        let location = outer
            .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
            .ok_or_else(|| CardTextError::ParseError("missing existential location".to_string()))?;
        if !surface::exact(location, &["your", "graveyard"]) {
            return Ok(None);
        }
        outer
            .capture_clause("descriptors", clause)
            .ok_or_else(|| CardTextError::ParseError("missing existential objects".to_string()))?
    } else {
        return Ok(None);
    };
    let descriptor_atoms = [
        WinnowSequence::object("left", WinnowCaptureKind::UntilPhrase(&["and"])),
        WinnowSequence::word("and"),
        WinnowSequence::object("right", WinnowCaptureKind::Rest),
    ];
    let Some(split) = WinnowSequence::new(&descriptor_atoms).parse_full(descriptors) else {
        return Ok(None);
    };
    let left = split
        .capture_clause("left", descriptors)
        .ok_or_else(|| CardTextError::ParseError("missing left existential object".to_string()))?
        .trimmed();
    let right = split
        .capture_clause("right", descriptors)
        .ok_or_else(|| CardTextError::ParseError("missing right existential object".to_string()))?
        .trimmed();

    let is_independently_articled_card = |object: LexedClause<'_>| {
        object
            .token(0)
            .is_some_and(|token| is_article(token.parser_text()))
            && object
                .tokens()
                .iter()
                .any(|token| token_word_is_any(token, CARD_OR_CARDS_WORDS))
    };
    if !is_independently_articled_card(left) || !is_independently_articled_card(right) {
        return Ok(None);
    }

    let mut left_filter = parse_object_filter(left.tokens(), false)?;
    let mut right_filter = parse_object_filter(right.tokens(), false)?;
    for filter in [&mut left_filter, &mut right_filter] {
        filter.zone = Some(Zone::Graveyard);
        filter.owner = Some(PlayerFilter::You);
    }

    Ok(Some(PredicateAst::And(
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: left_filter,
        }),
        Box::new(PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: right_filter,
        }),
    )))
}

pub(super) fn parse_card_in_your_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object("descriptor", WinnowCaptureKind::UntilPhrase(&["in"])),
        WinnowSequence::action("preposition", WinnowCaptureKind::OneOf(&["in"])),
        WinnowSequence::modifier("location", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let existential = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !is_card_graveyard_existential_clause(existential) {
        return None;
    }

    let location = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    if !is_graveyard_location_clause(location) {
        return None;
    }

    let descriptor = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if descriptor.tokens().is_empty() {
        return None;
    }
    let mut filter = parse_object_filter(descriptor.tokens(), false)
        .ok()
        .or_else(|| {
            descriptor
                .tokens()
                .last()
                .and_then(OwnedLexToken::as_word)
                .filter(|word| word_is_any(word, CARD_OR_CARDS_WORDS))
                .and_then(|_| {
                    let trimmed_tokens =
                        &descriptor.tokens()[..descriptor.tokens().len().saturating_sub(1)];
                    parse_object_filter(trimmed_tokens, false).ok()
                })
        })
        .or_else(|| parse_subtype_card_descriptor_clause(descriptor))?;
    filter.zone = Some(Zone::Graveyard);
    filter.owner = Some(PlayerFilter::You);

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter,
    })
}

pub(super) fn parse_object_on_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let Some(relation) = parse_prepositional_copula_relation_clauses(tokens, &["on"]) else {
        return Ok(None);
    };
    if !surface::exact(relation.preposition_clause, &["on"])
        || !is_battlefield_zone_clause(relation.tail_clause)
    {
        return Ok(None);
    }

    let object_clause = relation.subject_clause;
    let object_tokens = object_clause.tokens();
    if object_tokens.is_empty() {
        return Ok(None);
    }
    let mut filter = parse_object_filter(object_tokens, false)?;
    if filter.name.is_some()
        && let Some(name) = parse_named_object_filter_name_tail(object_tokens)
    {
        filter.name = Some(name);
    }
    filter.zone = Some(Zone::Battlefield);

    Ok(Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThan,
        right: Value::Fixed(0),
    }))
}

pub(super) fn parse_named_object_filter_name_tail(tokens: &[OwnedLexToken]) -> Option<String> {
    const NAMED_OBJECT_PATTERN: WinnowSequence<'static> = WinnowSequence::new(&[
        WinnowSequence::object("object", WinnowCaptureKind::UntilPhrase(&["named"])),
        WinnowSequence::word("named"),
        WinnowSequence::modifier("name", WinnowCaptureKind::Rest),
    ]);

    let clause = LexedClause::new(tokens);
    let matched = NAMED_OBJECT_PATTERN.parse_full(clause)?;
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if object.tokens().is_empty() {
        return None;
    }
    let name = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let name_words = name.word_refs();
    let name_end = find_name_clause_end(name_words.as_slice(), 0);
    let name = render_token_slice(name.between_words_trimmed(0, name_end).tokens())
        .trim()
        .to_ascii_lowercase();
    (!name.is_empty()).then_some(name)
}

pub(super) fn graveyard_card_types_subject(clause: LexedClause<'_>) -> Option<PlayerAst> {
    if surface::exact(clause, YOUR_GRAVEYARD_PHRASE) {
        Some(PlayerAst::You)
    } else if surface::exact_any(clause, THAT_PLAYER_GRAVEYARD_PHRASES) {
        Some(PlayerAst::That)
    } else if surface::exact_any(clause, TARGET_PLAYER_GRAVEYARD_PHRASES) {
        Some(PlayerAst::Target)
    } else if surface::exact_any(clause, TARGET_OPPONENT_GRAVEYARD_PHRASES) {
        Some(PlayerAst::TargetOpponent)
    } else if surface::exact_any(clause, OPPONENT_GRAVEYARD_PHRASES) {
        Some(PlayerAst::Opponent)
    } else {
        None
    }
}

pub(super) fn parse_card_types_in_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let card_type_phrases: &[&[&str]] = &[
        &["card", "type", "among", "card", "in"],
        &["card", "type", "among", "cards", "in"],
        &["card", "types", "among", "card", "in"],
        &["card", "types", "among", "cards", "in"],
    ];
    let atoms = [
        WinnowSequence::subject("lead", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::amount(
            "quantity",
            WinnowCaptureKind::UntilAnyPhrase(card_type_phrases),
        ),
        WinnowSequence::any_phrase(card_type_phrases),
        WinnowSequence::modifier("graveyard", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let lead = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    let constrained_player = card_types_graveyard_lead_player_clause(lead)?;
    let quantity = matched.capture_clause_by_role(WinnowCaptureRole::Amount, clause)?;
    let (count, used) = predicate_at_least_quantity_prefix_tokens(quantity.tokens())?;
    if used != quantity.tokens().len() {
        return None;
    }
    let graveyard = matched.capture_clause_by_role(WinnowCaptureRole::Modifier, clause)?;
    let player = graveyard_card_types_subject(graveyard)?;
    if constrained_player.is_some_and(|expected| expected != player) {
        return None;
    }

    Some(PredicateAst::PlayerHasCardTypesInGraveyardOrMore { player, count })
}

pub(super) fn card_types_graveyard_lead_player_clause(
    clause: LexedClause<'_>,
) -> Option<Option<PlayerAst>> {
    if is_there_are_clause(clause) {
        return Some(None);
    }
    if surface::exact(clause, &["you", "have"]) {
        return Some(Some(PlayerAst::You));
    }
    None
}

pub(super) fn parse_there_are_objects_on_battlefield_predicate(
    tokens: &[OwnedLexToken],
) -> Result<Option<PredicateAst>, CardTextError> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("existential", WinnowCaptureKind::WordCount(2)),
        WinnowSequence::object(
            "counted_object",
            WinnowCaptureKind::UntilLastPhrase(&["on"]),
        ),
        WinnowSequence::action("preposition", WinnowCaptureKind::OneOf(&["on"])),
        WinnowSequence::modifier("location", WinnowCaptureKind::Rest),
    ];
    let Some(matched) = WinnowSequence::new(&atoms).parse_full(clause) else {
        return Ok(None);
    };
    let existential = matched
        .capture_clause_by_role(WinnowCaptureRole::Subject, clause)
        .ok_or_else(|| {
            CardTextError::ParseError(
                "missing existential in battlefield count predicate".to_string(),
            )
        })?;
    if !is_there_are_clause(existential) {
        return Ok(None);
    }
    let location = matched
        .capture_clause_by_role(WinnowCaptureRole::Modifier, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing location in battlefield count predicate".to_string())
        })?;
    if !is_battlefield_zone_clause(location) {
        return Ok(None);
    }

    let counted_object = matched
        .capture_clause_by_role(WinnowCaptureRole::Object, clause)
        .ok_or_else(|| {
            CardTextError::ParseError("missing object in battlefield count predicate".to_string())
        })?;
    let Some((count, used)) = predicate_at_least_quantity_prefix_tokens(counted_object.tokens())
    else {
        return Ok(None);
    };
    let object_tokens = counted_object.tokens().get(used..).unwrap_or_default();
    let other = object_tokens
        .first()
        .is_some_and(|token| token_word_is_any(token, OTHER_OR_ANOTHER_WORDS));
    let filter_tokens = if other {
        object_tokens.get(1..).unwrap_or_default()
    } else {
        object_tokens
    };
    if filter_tokens.is_empty() {
        return Ok(None);
    }

    let mut filter = parse_object_filter(filter_tokens, other)?;
    filter.zone = Some(Zone::Battlefield);

    Ok(Some(PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(count as i32),
    }))
}

pub(super) fn parse_exploited_triggering_object_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let atoms = [
        WinnowSequence::subject("subject", WinnowCaptureKind::WordCount(1)),
        WinnowSequence::action("action", WinnowCaptureKind::OneOf(&["exploited"])),
        WinnowSequence::object("object", WinnowCaptureKind::Rest),
    ];
    let matched = WinnowSequence::new(&atoms).parse_full(clause)?;
    let subject = matched.capture_clause_by_role(WinnowCaptureRole::Subject, clause)?;
    if !surface::exact(subject, &["it"]) {
        return None;
    }
    let object = matched.capture_clause_by_role(WinnowCaptureRole::Object, clause)?;
    if !surface::exact_any(object, &[&["that", "creature"], &["that", "object"]]) {
        return None;
    }
    Some(PredicateAst::And(
        Box::new(PredicateAst::TaggedMatches(
            TagKey::from(crate::tag::EXPLOITED_TAG),
            ObjectFilter::tagged("triggering"),
        )),
        Box::new(PredicateAst::TaggedMatches(
            TagKey::from(crate::tag::EXPLOITER_TAG),
            ObjectFilter::source(),
        )),
    ))
}

pub(super) fn predicate_diagnostic_tokens(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    let mut display_tokens: Vec<OwnedLexToken> = tokens
        .iter()
        .filter(|token| {
            !token
                .as_word()
                .is_some_and(|_| is_article(token.parser_text()))
        })
        .cloned()
        .collect();

    if let Some(first) = display_tokens.first_mut()
        && token_word_is_any(first, ITS_WORDS)
    {
        first.replace_word("it");
    }
    if display_tokens.len() >= 2
        && token_word_is(&display_tokens[0], IT_WORD)
        && display_tokens[1].is_word("s")
    {
        display_tokens.remove(1);
    }

    if let Some(instead_idx) =
        primitives::find_prefix(&display_tokens, || primitives::kw(INSTEAD_WORD))
            .map(|(token_idx, _, _)| token_idx)
        && instead_idx > 0
    {
        let maybe_predicate = &display_tokens[..instead_idx];
        let maybe_clause = LexedClause::new(maybe_predicate);
        let maybe_word_len = maybe_clause.word_len();
        let paid_tail = maybe_word_len >= 3
            && maybe_clause
                .between_word_range(maybe_word_len - 3, maybe_word_len)
                .is_some_and(|tail| surface::exact_any(tail, COST_PAID_INSTEAD_TAIL_PHRASES));
        let unpaid_tail = maybe_word_len >= 4
            && maybe_clause
                .between_word_range(maybe_word_len - 4, maybe_word_len)
                .is_some_and(|tail| surface::exact(tail, COST_NOT_PAID_INSTEAD_TAIL_PHRASE));
        if paid_tail || unpaid_tail {
            display_tokens.truncate(instead_idx);
        }
    }

    let display_clause = LexedClause::new(&display_tokens);
    if surface::contains(display_clause, YOU_BOTH_OWN_AND_CONTROL_PHRASE)
        && let Some(exile_word_idx) = surface::find(display_clause, EXILE_THEM_PHRASE)
        && let Some(exile_token_idx) = display_clause
            .words()
            .token_span_for_words(exile_word_idx, exile_word_idx + 1)
            .map(|range| range.start)
    {
        display_tokens.truncate(exile_token_idx);
    }

    display_tokens
}

pub(super) fn predicate_diagnostic_text(tokens: &[OwnedLexToken]) -> String {
    render_token_slice(&predicate_diagnostic_tokens(tokens))
}

pub(super) fn render_unsupported_predicate_message(tokens: &[OwnedLexToken]) -> String {
    format!(
        "unsupported predicate (predicate: '{}')",
        predicate_diagnostic_text(tokens)
    )
}

fn parse_source_regenerated_this_turn_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if !surface::exact_any(
        clause,
        &[
            &["this", "creature", "regenerated", "this", "turn"][..],
            &["this", "permanent", "regenerated", "this", "turn"][..],
            &["it", "regenerated", "this", "turn"][..],
        ],
    ) {
        return None;
    }
    Some(PredicateAst::ValueComparison {
        left: Value::SourceRegeneratedThisTurnCount,
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    })
}

fn parse_source_only_creature_card_in_your_graveyard_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    if !surface::exact(
        clause,
        &[
            "this",
            "card",
            "is",
            "the",
            "only",
            "creature",
            "card",
            "in",
            "your",
            "graveyard",
        ],
    ) {
        return None;
    }

    let mut creature_cards = ObjectFilter::creature()
        .in_zone(Zone::Graveyard)
        .owned_by(PlayerFilter::You);
    creature_cards.set_explicit_card_noun(true);
    creature_cards.set_explicit_card_type_noun(Some(CardType::Creature));
    Some(PredicateAst::ValueComparison {
        left: Value::Count(creature_cards),
        operator: ValueComparisonOperator::Equal,
        right: Value::Fixed(1),
    })
}

fn parse_each_global_greatest_power_predicate(tokens: &[OwnedLexToken]) -> Option<PredicateAst> {
    let clause = LexedClause::new(tokens);
    let words = clause.word_refs();
    if words.len() < 3
        || !surface::exact_words(&words[..2], &["you", "control"])
        || words[2] != "each"
        || !is_creature_on_battlefield_with_greatest_power(
            &words[3..]
                .iter()
                .map(|word| (*word).to_string())
                .collect::<Vec<_>>(),
        )
    {
        return None;
    }

    let filter_tokens = clause
        .tokens()
        .get(clause.words().token_span_for_words(2, words.len())?)?;
    let mut global_creatures = parse_object_filter(filter_tokens, false).ok()?;
    global_creatures.controller = None;
    global_creatures.zone = Some(Zone::Battlefield);
    let mut greatest_creatures = global_creatures.clone();
    greatest_creatures.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
        Value::GreatestPower(global_creatures.clone()),
    )));
    let mut controlled = greatest_creatures.clone();
    controlled.controller = Some(PlayerFilter::You);

    Some(PredicateAst::ValueComparison {
        left: Value::Count(controlled),
        operator: crate::effect::ValueComparisonOperator::Equal,
        right: Value::Count(greatest_creatures),
    })
}

fn parse_a_global_greatest_power_control_predicate(
    tokens: &[OwnedLexToken],
) -> Option<PredicateAst> {
    let words = LexedClause::new(tokens).word_refs();
    if !surface::exact_words(
        &words,
        &[
            "you",
            "control",
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
    ) {
        return None;
    }

    let global_creatures = ObjectFilter::creature().in_zone(Zone::Battlefield);
    let mut controlled_greatest_creature =
        global_creatures.clone().controlled_by(PlayerFilter::You);
    controlled_greatest_creature.power = Some(crate::filter::Comparison::EqualExpr(Box::new(
        Value::GreatestPower(global_creatures),
    )));

    Some(PredicateAst::PlayerControls {
        player: PlayerAst::You,
        filter: controlled_greatest_creature,
    })
}

pub fn parse_predicate(tokens: &[OwnedLexToken]) -> Result<PredicateAst, CardTextError> {
    let predicate_tokens = if token_slice_first_is(tokens, "if") {
        &tokens[1..]
    } else {
        tokens
    };

    if !predicate_tokens.iter().any(|token| {
        token
            .as_word()
            .is_some_and(|_| !is_article(token.parser_text()))
    }) {
        return Err(CardTextError::ParseError(
            "empty predicate in if clause".to_string(),
        ));
    }

    // Repeated articles on both sides of "and" are independent existential
    // requirements. Preserve that relationship before the broad conjunction
    // parsers can merge both card types into one disjunctive filter.
    if let Some(predicate) = parse_conjoined_cards_in_your_graveyard_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    // Keep independently articulated control conjunctions ahead of the broad
    // phase-step control gate, whose generic object-filter parser would merge
    // them into one filter (for example, "an artifact and a creature").
    if let Some(predicate) =
        parse_you_control_or_returned_to_hand_this_way_predicate(predicate_tokens).transpose()?
    {
        return Ok(predicate);
    }
    if let Some(predicate) =
        parse_you_control_or_graveyard_predicate(predicate_tokens).transpose()?
    {
        return Ok(predicate);
    }
    // Split independently articulated player predicates before the broad
    // control-object conjunction parser. The latter is intentionally for
    // phrases such as "you control an artifact and a creature"; if it sees
    // "you control no permanents ... and have no cards in hand" first, it
    // treats both authored negatives as object-filter text and inverts them.
    if let Some(predicate) = parse_implicit_subject_and_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if non_article_token_words_starts_with_any(predicate_tokens, YOU_CONTROL_PREFIXES)
        && let Some(predicate) =
            parse_you_control_conjoined_predicate(predicate_tokens).transpose()?
    {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_each_global_greatest_power_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_a_global_greatest_power_control_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = phase_step_gates::parse_phase_step_gate_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_triggering_spell_ordinal_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_regenerated_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) =
        parse_source_only_creature_card_in_your_graveyard_predicate(predicate_tokens)
    {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_turn_history_intervening_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_triggering_object_first_tap_this_turn_predicate(predicate_tokens)
    {
        return Ok(predicate);
    }
    if let Some(predicate) =
        parse_triggering_object_first_counters_this_turn_predicate(predicate_tokens)
    {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_repeated_if_or_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_repeated_and_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    {
        let simple_words = non_article_token_word_refs(predicate_tokens);
        if [
            &["this", "creature", "is", "suspected"][..],
            &["this", "permanent", "is", "suspected"][..],
            &["it", "is", "suspected"][..],
            &["its", "suspected"][..],
        ]
        .iter()
        .any(|expected| surface::exact_words(&simple_words, expected))
        {
            return Ok(PredicateAst::SourceSuspected);
        }
    }
    if let Some(predicate) = parse_secret_choices_match_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_vote_result_predicate(predicate_tokens, true)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_passive_this_way_tagged_object_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_active_this_way_discard_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_active_this_way_battlefield_predicate(predicate_tokens)? {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_passive_this_way_battlefield_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_this_ability_resolution_count_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_stack_object_targets_only_source_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_stack_object_targets_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    // Spell-context comparisons are exact typed predicates.  Parse them
    // before broader control/object predicates can accept only the leading
    // "you control ..." portion and discard the relative spell controller.
    if let Some(predicate) = parse_spell_context_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_exploited_triggering_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_graveyard_cards_above_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_zone_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_exiled_with_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_happily_style_conjoined_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_colors_among_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_among_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_revealed_or_controlled_subtype_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_graveyard_threshold_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_in_your_graveyard_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_quantified_objects_in_graveyard_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_empty_battlefield_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_life_total_at_least_starting_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_life_total_at_least_last_noted_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_more_than_each_other_player_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_fewer_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_counted_objects_have_counter_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_counted_source_exiled_objects_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_controlled_creatures_total_power_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    // Qualitative life comparisons such as "you have the most life" share
    // the `you have ... life` prefix with fixed life-total bounds. Give the
    // typed relation parser first refusal so the numeric parser does not turn
    // an ordinary nonnumeric amount into a hard parse error.
    if let Some(predicate) = parse_player_life_relation_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_you_life_total_at_most_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_object_keyword_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_opponent_controls_tagged_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_opponent_controls_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_vote_result_predicate(predicate_tokens, false)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_attacking_you_own_control_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_you_both_own_and_control_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_while_conjoined_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_tagged_state_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_simple_state_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_crewed_by_exactly_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_attachment_count_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_identity_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_keyword_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_triggering_object_keyword_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) =
        parse_source_did_not_attack_or_enter_control_this_turn_shape(predicate_tokens)
    {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_no_counters_on_source_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_doesnt_have_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_has_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_has_counted_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_verbless_counted_counter_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_triggering_object_had_counter_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_source_counters_at_least_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_source_power_threshold_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_basic_land_types_among_lands_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_there_are_objects_on_battlefield_predicate(tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_card_types_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_half_starting_life_total_threshold_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_player_cards_in_graveyard_predicate(tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_player_controls_more_than_each_other_player_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_fewer_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_controls_more_than_you_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_relation_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_tie_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_count_parity_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_total_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_relation_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_cards_in_hand_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_turn_event_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_would_action_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_turn_timing_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_life_change_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_descended_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_object_death_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_change_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_battlefield_entry_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_combat_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_spell_lifecycle_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_paid_cost_label_predicate(predicate_tokens) {
        return Ok(predicate);
    }
    if let Some(predicate) = parse_mana_spent_capture_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_attached_tagged_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_additional_cost_object_state_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_tagged_exiled_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    let demonstrative_reference = demonstrative_reference_kind(predicate_tokens);
    let is_it = demonstrative_reference == Some(DemonstrativeReferenceKind::It);

    if let Some(predicate) = parse_triggering_object_source_stat_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_value_reference_comparison_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if is_it {
        if let Some(predicate) = parse_demonstrative_mana_value_predicate(predicate_tokens)? {
            return Ok(predicate);
        }
        if let Some(predicate) =
            parse_demonstrative_total_power_toughness_predicate(predicate_tokens)?
        {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_power_or_toughness_predicate(predicate_tokens)?
        {
            return Ok(predicate);
        }
    }

    if demonstrative_reference.is_some()
        && predicate_tokens
            .iter()
            .any(|token| token_word_is(token, OR_WORD))
        && !contains_most_common_color_among_all_permanents_clause(predicate_tokens)
    {
        // Let a single demonstrative copula own its complete coordinated
        // descriptor before the broad boolean splitter sees the conjunction.
        // In particular, negation in "it isn't A or B" scopes over A or B.
        if let Some(predicate) = parse_demonstrative_or_descriptor_predicate(predicate_tokens)? {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_or_predicate(predicate_tokens)? {
            return Ok(predicate);
        }
    }

    if demonstrative_reference.is_some() {
        if let Some(predicate) = parse_demonstrative_power_or_toughness_predicate(predicate_tokens)?
        {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_shares_predicate(predicate_tokens) {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_or_descriptor_predicate(predicate_tokens)? {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_toxic_predicate(predicate_tokens) {
            return Ok(predicate);
        }
        if let Some(predicate) = parse_demonstrative_keyword_predicate(predicate_tokens) {
            return Ok(predicate);
        }
        if let Some((
            descriptor_tokens,
            negative,
            has_card,
            tagged_that_enchantment,
            mut match_time,
        )) = demonstrative_descriptor_filter_tokens(predicate_tokens)
        {
            let antecedent_surface = demonstrative_antecedent_surface(predicate_tokens);
            // "was blocked this turn" is a passive historical-event predicate,
            // not a copular last-known-characteristics predicate. It already
            // has dedicated turn-history semantics and surface rendering.
            let descriptor_clause = LexedClause::new(&descriptor_tokens);
            if surface::exact(descriptor_clause, &["blocked", "this", "turn"]) {
                match_time = DemonstrativeMatchTime::Current;
            }
            if surface::exact(descriptor_clause, &["permanent", "spell"]) {
                let mut filter =
                    crate::grammar::permission_facts::subject_filters::permanent_spell_filter();
                filter.zone = Some(Zone::Stack);
                filter.stack_kind = Some(StackObjectKind::Spell);
                if antecedent_surface.is_some() {
                    filter.set_demonstrative_antecedent_surface(antecedent_surface);
                }
                let predicate = demonstrative_match_predicate(filter, match_time);
                return Ok(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                });
            }
            if let Some(mut filter) =
                parse_single_card_type_card_descriptor_tokens(&descriptor_tokens)
            {
                if antecedent_surface.is_some() {
                    filter.set_demonstrative_antecedent_surface(antecedent_surface);
                }
                let predicate = if filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    if match_time == DemonstrativeMatchTime::LastKnown {
                        PredicateAst::ItMatchedLastKnown(filter)
                    } else {
                        PredicateAst::ItIsLandCard
                    }
                } else {
                    demonstrative_match_predicate(filter, match_time)
                };
                return Ok(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                });
            }
            if let Ok(mut filter) = parse_object_filter_lexed(&descriptor_tokens, false)
                && filter != ObjectFilter::default()
            {
                if antecedent_surface.is_some() {
                    filter.set_demonstrative_antecedent_surface(antecedent_surface);
                }
                if has_card
                    && filter.card_types.len() == 1
                    && filter.card_types[0] == CardType::Land
                    && filter.subtypes.is_empty()
                    && !filter.nontoken
                    && filter.excluded_card_types.is_empty()
                {
                    let predicate = if match_time == DemonstrativeMatchTime::LastKnown {
                        PredicateAst::ItMatchedLastKnown(filter)
                    } else {
                        PredicateAst::ItIsLandCard
                    };
                    return Ok(if negative {
                        PredicateAst::Not(Box::new(predicate))
                    } else {
                        predicate
                    });
                }
                if tagged_that_enchantment && match_time == DemonstrativeMatchTime::Current {
                    return Ok(PredicateAst::TaggedMatches(
                        crate::tag::CompilerReferenceTag::Triggering.key(),
                        filter,
                    ));
                }
                let predicate = demonstrative_match_predicate(filter, match_time);
                return Ok(if negative {
                    PredicateAst::Not(Box::new(predicate))
                } else {
                    predicate
                });
            }
        }
    }

    if let Some(predicate) = parse_player_controls_no_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    if let Some(predicate) =
        parse_you_control_or_graveyard_predicate(predicate_tokens).transpose()?
    {
        return Ok(predicate);
    }

    if non_article_token_words_starts_with_any(predicate_tokens, YOU_CONTROL_PREFIXES) {
        if let Some(predicate) =
            parse_you_control_conjoined_predicate(predicate_tokens).transpose()?
        {
            return Ok(predicate);
        }

        if let Some(predicate) = parse_player_controls_predicate(
            predicate_tokens,
            PlayerAst::You,
            Some(PlayerFilter::You),
            2,
            true,
            true,
        )? {
            return Ok(predicate);
        }
    }

    if non_article_token_words_starts_with_any(predicate_tokens, THAT_PLAYER_CONTROLS_PREFIXES) {
        let prefix_len = if predicate_tokens
            .first()
            .is_some_and(|token| token_word_is(token, "they"))
        {
            2
        } else {
            3
        };
        if let Some(predicate) = parse_player_controls_predicate(
            predicate_tokens,
            PlayerAst::That,
            None,
            prefix_len,
            false,
            false,
        )? {
            return Ok(predicate);
        }
    }

    if let Some(predicate) = parse_negative_put_tagged_object_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_achievement_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_ring_bearer_temptation_predicate(tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_status_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_world_state_or_timing_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_combat_damage_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_player_spell_cast_this_turn_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_x_value_comparison_predicate(predicate_tokens) {
        return Ok(predicate);
    }

    if let Some(predicate) = parse_or_predicate(predicate_tokens)? {
        return Ok(predicate);
    }

    Err(CardTextError::ParseError(
        render_unsupported_predicate_message(predicate_tokens),
    ))
}
