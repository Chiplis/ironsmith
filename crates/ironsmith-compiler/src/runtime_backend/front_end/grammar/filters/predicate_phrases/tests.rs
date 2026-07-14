use super::*;
use crate::CounterType;
use crate::effect::{ChoiceCount, ValueComparisonOperator};
use crate::filter::StackObjectKind;
use crate::runtime_backend::front_end::lexer::lex_line;

const IF_WORD: &str = "if";

fn predicate_tokens_after_if(tokens: &[OwnedLexToken]) -> Vec<OwnedLexToken> {
    tokens
        .iter()
        .filter(|token| !token_word_is(token, IF_WORD))
        .cloned()
        .collect()
}

#[test]
fn parse_predicate_paid_cost_labels_use_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If this spells surge cost was paid",
            PredicateAst::ThisSpellPaidLabel("Surge".into()),
        ),
        (
            "If this creature's spectacle cost was paid instead discard your hand",
            PredicateAst::ThisSpellPaidLabel("Spectacle".into()),
        ),
        (
            "If {U} cost was paid",
            PredicateAst::ThisSpellPaidLabel("{U}".into()),
        ),
        (
            "If {2}{G} cost wasn't paid",
            PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("{2}{G}".into()))),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_supports_opponent_would_begin_extra_turn() -> Result<(), CardTextError> {
    let tokens = lex_line("If an opponent would begin an extra turn", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::PlayerWouldBeginExtraTurn {
            player: PlayerAst::Opponent,
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_x_value_comparison_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, operator, amount) in [
        ("If X is 3", ValueComparisonOperator::Equal, 3),
        (
            "If X is less than or equal to two",
            ValueComparisonOperator::LessThanOrEqual,
            2,
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ValueComparison {
                left: Value::X,
                operator,
                right: Value::Fixed(amount),
            },
            "{text}"
        );
    }

    let tokens = lex_line(
        "If X is greater than or equal to the number of cards in your library",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let PredicateAst::ValueComparison {
        left: Value::X,
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Count(filter),
    } = parsed
    else {
        panic!("expected X-to-library-count comparison, got {parsed:?}");
    };
    assert_eq!(filter.zone, Some(Zone::Library));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    Ok(())
}

#[test]
fn parse_predicate_vote_results_use_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If death gets more votes",
            PredicateAst::VoteOptionGetsMoreVotes {
                option: "death".to_string(),
            },
        ),
        (
            "If torture gets more votes or the vote is tied",
            PredicateAst::VoteOptionGetsMoreVotesOrTied {
                option: "torture".to_string(),
            },
        ),
        (
            "If no creatures got votes",
            PredicateAst::NoVoteObjectsMatched {
                filter: ObjectFilter::creature(),
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_secret_choices_match_uses_capture_parser() -> Result<(), CardTextError> {
    for text in ["If they match", "If those choices match"] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::SecretChoicesMatch, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_source_identity_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If this enchantment isn't a creature", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert_eq!(
        parsed,
        PredicateAst::Not(Box::new(PredicateAst::SourceMatches(
            ObjectFilter::creature()
        )))
    );

    let tokens = lex_line("If this source is not an artifact", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert_eq!(
        parsed,
        PredicateAst::Not(Box::new(PredicateAst::SourceMatches(
            ObjectFilter::artifact()
        )))
    );

    let tokens = lex_line("If this permanent is red", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    match parsed {
        PredicateAst::SourceMatches(filter) => {
            assert!(filter.colors.is_some(), "{filter:?}");
        }
        other => panic!("expected source identity predicate, got {other:?}"),
    }
    Ok(())
}

#[test]
fn parse_predicate_source_attachment_count_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If this creature is enchanted by two or more Auras", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    match parsed {
        PredicateAst::SourceHasAttachmentsMatching {
            filter,
            comparison,
            display,
        } => {
            assert_eq!(
                comparison,
                crate::effect::Comparison::GreaterThanOrEqual(2),
                "{display}"
            );
            assert!(filter.subtypes.contains(&Subtype::Aura), "{filter:?}");
            assert_eq!(display, "this creature is enchanted by two or more auras");
        }
        other => panic!("expected source attachment predicate, got {other:?}"),
    }
    Ok(())
}

#[test]
fn parse_predicate_player_object_keywords_use_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If creatures you control have flying", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    match parsed {
        PredicateAst::PlayerControls { player, filter } => {
            assert_eq!(player, PlayerAst::You);
            assert_eq!(filter.controller, Some(PlayerFilter::You));
            assert!(
                filter.card_types.contains(&CardType::Creature),
                "{filter:?}"
            );
            assert!(
                filter
                    .static_abilities
                    .contains(&crate::static_abilities::StaticAbilityId::Flying),
                "{filter:?}"
            );
        }
        other => panic!("expected player-controls keyword predicate, got {other:?}"),
    }

    let tokens = lex_line("If nonland cards in your graveyard have escape", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    match parsed {
        PredicateAst::PlayerControls { player, filter } => {
            assert_eq!(player, PlayerAst::You);
            assert_eq!(filter.zone, Some(Zone::Graveyard));
            assert_eq!(filter.owner, Some(PlayerFilter::You));
            assert_eq!(
                filter.alternative_cast,
                Some(crate::filter::AlternativeCastKind::Escape),
                "{filter:?}"
            );
        }
        other => panic!("expected graveyard keyword predicate, got {other:?}"),
    }
    Ok(())
}

#[test]
fn parse_predicate_you_control_that_creature_keeps_tagged_reference() -> Result<(), CardTextError> {
    let tokens = lex_line("If you control that creature", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    match parsed {
        PredicateAst::PlayerControls { player, filter } => {
            assert_eq!(player, PlayerAst::You);
            assert_eq!(filter.controller, Some(PlayerFilter::You));
            assert!(
                filter.card_types.contains(&CardType::Creature),
                "{filter:?}"
            );
            assert!(
                filter.tagged_constraints.iter().any(|constraint| {
                    constraint.tag.as_str() == IT_TAG
                        && constraint.relation == TaggedOpbjectRelation::IsTaggedObject
                }),
                "{filter:?}"
            );
        }
        other => panic!("expected player-controls tagged predicate, got {other:?}"),
    }
    Ok(())
}

#[test]
fn parse_predicate_opponent_controls_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_filter) in [
        (
            "If opponent controls artifact",
            ObjectFilter {
                controller: Some(PlayerFilter::Opponent),
                card_types: vec![CardType::Artifact],
                ..Default::default()
            },
        ),
        (
            "If an opponent controls another creature",
            ObjectFilter {
                controller: Some(PlayerFilter::Opponent),
                card_types: vec![CardType::Creature],
                other: true,
                ..Default::default()
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerControls {
                player: PlayerAst::Opponent,
                filter: expected_filter,
            },
            "{text}"
        );
    }

    let tokens = lex_line("If an opponent controls more creatures than you", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert!(
        matches!(parsed, PredicateAst::PlayerControlsMoreThanYou { .. }),
        "{parsed:?}"
    );
    Ok(())
}

#[test]
fn parse_predicate_opponent_controls_tagged_object_uses_capture_parser() -> Result<(), CardTextError>
{
    for (text, filter) in [
        (
            "If an opponent controls it",
            ObjectFilter {
                controller: Some(PlayerFilter::Opponent),
                ..Default::default()
            },
        ),
        (
            "If opponent controls that creature",
            ObjectFilter {
                controller: Some(PlayerFilter::Opponent),
                card_types: vec![CardType::Creature],
                ..Default::default()
            },
        ),
        (
            "If an opponent controls that permanent",
            ObjectFilter {
                controller: Some(PlayerFilter::Opponent),
                ..Default::default()
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, PredicateAst::ItMatches(filter), "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_demonstrative_permanent_card_strips_article() -> Result<(), CardTextError> {
    let tokens = lex_line("If it's a permanent card", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    assert_eq!(
        parsed,
        PredicateAst::ItMatches(ObjectFilter::permanent_card())
    );
    Ok(())
}

#[test]
fn parse_predicate_preserves_last_known_copula_and_negation() -> Result<(), CardTextError> {
    let creature = ObjectFilter::creature();
    let horror = ObjectFilter::default().with_subtype(Subtype::Horror);
    let demon = ObjectFilter::default().with_subtype(Subtype::Demon);

    for (text, expected) in [
        (
            "If it was a creature",
            PredicateAst::ItMatchedLastKnown(creature),
        ),
        (
            "If that creature was a Horror",
            PredicateAst::ItMatchedLastKnown(horror),
        ),
        (
            "If it wasn't a Demon",
            PredicateAst::Not(Box::new(PredicateAst::ItMatchedLastKnown(demon))),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(parsed, expected, "{text}");
    }

    let tokens = lex_line("If its power was 3 or greater", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let PredicateAst::ItMatchedLastKnown(filter) = parsed else {
        panic!("expected last-known power predicate, got {parsed:?}");
    };
    assert_eq!(
        filter.power,
        Some(ironsmith_core::FilterComparison::GreaterThanOrEqual(3))
    );
    Ok(())
}

#[test]
fn parse_predicate_demonstrative_negated_land_card_keeps_it_reference() -> Result<(), CardTextError>
{
    for text in ["If it isn't a land card", "If it is not a land card"] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::ItIsLandCard)),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_turn_timing_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        ("If it's your turn", PredicateAst::YourTurn),
        ("If your turn", PredicateAst::YourTurn),
        (
            "If it's not your turn",
            PredicateAst::Not(Box::new(PredicateAst::YourTurn)),
        ),
        (
            "If not your turn",
            PredicateAst::Not(Box::new(PredicateAst::YourTurn)),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_world_state_timing_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you or player you're attacking has initiative",
            PredicateAst::Or(
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::Defending,
                }),
            ),
        ),
        (
            "If you or a player you're attacking has the initiative",
            PredicateAst::Or(
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerHasInitiative {
                    player: PlayerAst::Defending,
                }),
            ),
        ),
        ("If it's night", PredicateAst::ItIsNight),
        ("If it is night", PredicateAst::ItIsNight),
        ("If it night", PredicateAst::ItIsNight),
        (
            "If it's the first combat phase of the turn",
            PredicateAst::FirstCombatPhaseOfTurn,
        ),
        (
            "If it first combat phase of turn",
            PredicateAst::FirstCombatPhaseOfTurn,
        ),
        (
            "If you cast this spell during your main phase",
            PredicateAst::ThisSpellPaidLabel("CastDuringYourMainPhase".into()),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_object_on_battlefield_uses_capture_parser() -> Result<(), CardTextError> {
    for text in [
        "If an artifact is on the battlefield",
        "If creatures are on battlefield",
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        match parsed {
            PredicateAst::ValueComparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(operator, ValueComparisonOperator::GreaterThan, "{text}");
                assert_eq!(right, Value::Fixed(0), "{text}");
                match left {
                    Value::Count(filter) => {
                        assert_eq!(filter.zone, Some(Zone::Battlefield), "{text}")
                    }
                    other => panic!("expected count for {text}, got {other:?}"),
                }
            }
            other => panic!("expected battlefield count predicate for {text}, got {other:?}"),
        }
    }
    Ok(())
}

#[test]
fn parse_predicate_counted_battlefield_objects_uses_capture_parser() -> Result<(), CardTextError> {
    for text in [
        "If there are three or more artifacts on the battlefield",
        "If there are two or more other creatures on battlefield",
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        match parsed {
            PredicateAst::ValueComparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(
                    operator,
                    ValueComparisonOperator::GreaterThanOrEqual,
                    "{text}"
                );
                match right {
                    Value::Fixed(value) => assert!(value >= 2, "{text}"),
                    other => panic!("expected fixed count for {text}, got {other:?}"),
                }
                match left {
                    Value::Count(filter) => {
                        assert_eq!(filter.zone, Some(Zone::Battlefield), "{text}")
                    }
                    other => panic!("expected count for {text}, got {other:?}"),
                }
            }
            other => panic!("expected battlefield count predicate for {text}, got {other:?}"),
        }
    }
    Ok(())
}

#[test]
fn parse_predicate_empty_battlefield_uses_capture_parser() -> Result<(), CardTextError> {
    for text in [
        "If no creatures are on the battlefield",
        "If no creature is on battlefield",
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ValueComparison {
                left: Value::Count(ObjectFilter::creature().in_zone(Zone::Battlefield)),
                operator: ValueComparisonOperator::Equal,
                right: Value::Fixed(0),
            },
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_conjoined_control_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If you control an artifact and a creature", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    let PredicateAst::And(left, right) = parsed else {
        panic!("expected conjoined control predicate");
    };
    assert_eq!(
        *left,
        PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::artifact().controlled_by(PlayerFilter::You),
        }
    );
    assert_eq!(
        *right,
        PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_control_or_graveyard_uses_capture_parser() -> Result<(), CardTextError> {
    for text in [
        "If you control a creature or there is a creature card in your graveyard",
        "If you control an artifact or artifact card in your graveyard",
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let PredicateAst::PlayerControlsOrHasCardInGraveyard {
            player,
            control_filter,
            graveyard_filter,
        } = parsed
        else {
            panic!("expected control-or-graveyard predicate for {text}");
        };
        assert_eq!(player, PlayerAst::You, "{text}");
        assert_eq!(control_filter.controller, Some(PlayerFilter::You), "{text}");
        assert_eq!(graveyard_filter.zone, Some(Zone::Graveyard), "{text}");
        assert_eq!(graveyard_filter.owner, Some(PlayerFilter::You), "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_repeated_or_if_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If you have the initiative or if you're monarch", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    assert_eq!(
        parsed,
        PredicateAst::Or(
            Box::new(PredicateAst::PlayerHasInitiative {
                player: PlayerAst::You,
            }),
            Box::new(PredicateAst::PlayerIsMonarch {
                player: PlayerAst::You,
            }),
        )
    );
    Ok(())
}

#[test]
fn parse_predicate_repeated_or_if_supports_value_reference_comparison() -> Result<(), CardTextError>
{
    let tokens = lex_line(
        "If that creature's power is 2 or less or if you control another Lizard",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let PredicateAst::Or(left, right) = parsed else {
        panic!("expected or predicate");
    };
    assert!(matches!(
        *left,
        PredicateAst::ValueComparison {
            left: Value::PowerOf(_),
            operator: ValueComparisonOperator::LessThanOrEqual,
            right: Value::Fixed(2),
        }
    ));
    let PredicateAst::PlayerControls { player, filter } = *right else {
        panic!("expected player-controls predicate");
    };
    assert_eq!(player, PlayerAst::You);
    assert!(filter.subtypes.contains(&Subtype::Lizard), "{filter:?}");
    Ok(())
}

#[test]
fn parse_predicate_supports_most_common_color_constraint_clause() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If it shares a color with the most common color among all permanents or a color tied for most common",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let PredicateAst::ItMatches(filter) = parsed else {
        panic!("expected it-matches predicate");
    };
    assert!(
        filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == TaggedOpbjectRelation::SharesMostCommonPermanentColor
        }),
        "expected most-common-color relation, got {filter:?}"
    );
    Ok(())
}

#[test]
fn parse_predicate_preserves_shared_creature_type_with_source() -> Result<(), CardTextError> {
    let tokens = lex_line("If it shares a creature type with this creature", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let mut expected = ObjectFilter::creature();
    expected.shares_creature_type_with_source = true;
    assert_eq!(parsed, PredicateAst::ItMatches(expected));
    Ok(())
}

#[test]
fn parse_predicate_source_counter_or_cards_in_hand_uses_capture_parser() -> Result<(), CardTextError>
{
    let tokens = lex_line(
        "If there are twenty or more counters on it or you have twenty or more cards in hand",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    assert_eq!(
        parsed,
        PredicateAst::Or(
            Box::new(PredicateAst::SourceHasCountersAtLeast(20)),
            Box::new(PredicateAst::PlayerCardsInHandOrMore {
                player: PlayerAst::You,
                count: 20,
            }),
        )
    );
    Ok(())
}

#[test]
fn parse_predicate_player_statuses_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you're monarch",
            PredicateAst::PlayerIsMonarch {
                player: PlayerAst::You,
            },
        ),
        (
            "If you have the initiative",
            PredicateAst::PlayerHasInitiative {
                player: PlayerAst::You,
            },
        ),
        (
            "If you have maximum speed",
            PredicateAst::ValueComparison {
                left: Value::Speed(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(4),
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_controlled_creatures_total_power_uses_shared_capture_parser()
-> Result<(), CardTextError> {
    let tokens = lex_line("If creatures you control have total power 8 or greater", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::ValueComparison {
            left: Value::TotalPower(ObjectFilter::creature().you_control()),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(8),
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_control_conditions_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you control three or more artifacts",
            PredicateAst::PlayerHasAtLeast {
                player: PlayerAst::You,
                filter: ObjectFilter::artifact().controlled_by(PlayerFilter::You),
                count: 3,
            },
        ),
        (
            "If you control three or more creatures with different powers",
            PredicateAst::PlayerHasAtLeastWithDifferentPowers {
                player: PlayerAst::You,
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::You),
                count: 3,
            },
        ),
        (
            "If that player controls exactly two lands",
            PredicateAst::PlayerControlsExactly {
                player: PlayerAst::That,
                filter: ObjectFilter::land(),
                count: 2,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_each_global_greatest_power_compares_the_complete_set()
-> Result<(), CardTextError> {
    let tokens = lex_line(
        "If you control each creature on the battlefield with the greatest power",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let PredicateAst::ValueComparison {
        left: Value::Count(controlled),
        operator: ValueComparisonOperator::Equal,
        right: Value::Count(global),
    } = parsed
    else {
        panic!("expected greatest-power set comparison, got {parsed:?}");
    };
    assert_eq!(controlled.controller, Some(PlayerFilter::You));
    assert_eq!(global.controller, None);
    assert_eq!(controlled.card_types, vec![CardType::Creature]);
    assert_eq!(global.card_types, vec![CardType::Creature]);
    assert_eq!(controlled.zone, Some(Zone::Battlefield));
    assert_eq!(global.zone, Some(Zone::Battlefield));
    assert!(matches!(
        &controlled.power,
        Some(crate::filter::Comparison::EqualExpr(value))
            if matches!(value.as_ref(), Value::GreatestPower(filter)
                if filter.controller.is_none()
                    && filter.card_types == vec![CardType::Creature]
                    && filter.zone == Some(Zone::Battlefield))
    ));
    assert_eq!(controlled.power, global.power);
    Ok(())
}

#[test]
fn parse_predicate_source_attack_control_gate_uses_capture_parser() -> Result<(), CardTextError> {
    for text in [
        "If this creature didn't attack or come under your control this turn",
        "If this creature didn't attack or came under your control this turn",
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::And(
                Box::new(PredicateAst::Not(Box::new(
                    PredicateAst::SourceAttackedThisTurn,
                ))),
                Box::new(PredicateAst::Not(Box::new(
                    PredicateAst::SourceCameUnderYourControlThisTurn,
                ))),
            ),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_source_states_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        ("If this tapped", PredicateAst::SourceIsTapped),
        (
            "If this creature is untapped",
            PredicateAst::Not(Box::new(PredicateAst::SourceIsTapped)),
        ),
        (
            "If this creature is enchanted",
            PredicateAst::SourceIsEnchanted,
        ),
        (
            "If this creature isn't equipped",
            PredicateAst::Not(Box::new(PredicateAst::SourceIsEquipped)),
        ),
        (
            "If this permanent is saddled",
            PredicateAst::SourceIsSaddled,
        ),
        (
            "If it isn't saddled",
            PredicateAst::Not(Box::new(PredicateAst::SourceIsSaddled)),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_negative_control_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you control no artifacts",
            PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter: ObjectFilter::artifact().controlled_by(PlayerFilter::You),
            },
        ),
        (
            "If a player controls no creatures",
            PredicateAst::PlayerControlsNo {
                player: PlayerAst::Any,
                filter: ObjectFilter::creature().controlled_by(PlayerFilter::Any),
            },
        ),
        (
            "If you do not control another creature",
            PredicateAst::PlayerControlsNo {
                player: PlayerAst::You,
                filter: ObjectFilter {
                    other: true,
                    ..ObjectFilter::creature().controlled_by(PlayerFilter::You)
                },
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_neither_control_keeps_tagged_relation() -> Result<(), CardTextError> {
    let tokens = lex_line("If you control neither creature", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    let mut expected_filter = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    expected_filter =
        expected_filter.match_tagged(TagKey::from(IT_TAG), TaggedOpbjectRelation::IsTaggedObject);
    assert_eq!(
        parsed,
        PredicateAst::PlayerControlsNo {
            player: PlayerAst::You,
            filter: expected_filter,
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_player_achievements_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you have city's blessing",
            PredicateAst::PlayerHasCitysBlessing {
                player: PlayerAst::You,
            },
        ),
        (
            "If you've completed a dungeon",
            PredicateAst::PlayerCompletedDungeon {
                player: PlayerAst::You,
                dungeon_name: None,
            },
        ),
        (
            "If you have completed Lost Mine of Phandelver",
            PredicateAst::PlayerCompletedDungeon {
                player: PlayerAst::You,
                dungeon_name: Some("Lost Mine of Phandelver".to_string()),
            },
        ),
        (
            "If you haven't completed Lost Mine of Phandelver",
            PredicateAst::Not(Box::new(PredicateAst::PlayerCompletedDungeon {
                player: PlayerAst::You,
                dungeon_name: Some("Lost Mine of Phandelver".to_string()),
            })),
        ),
        ("If you have a full party", PredicateAst::YouHaveFullParty),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_inherits_it_for_bare_or_descriptor_tail() -> Result<(), CardTextError> {
    let tokens = lex_line("If it's a creature or planeswalker card", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    match parsed {
        PredicateAst::Or(left, right) => {
            assert!(
                matches!(*left, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Creature]),
                "expected creature left predicate, got {left:?}"
            );
            assert!(
                matches!(*right, PredicateAst::ItMatches(ref filter) if filter.card_types == vec![CardType::Planeswalker]),
                "expected planeswalker right predicate, got {right:?}"
            );
        }
        other => panic!("expected inherited-reference or predicate, got {other:?}"),
    }
    Ok(())
}

#[test]
fn parse_predicate_keeps_mana_value_constraint_on_only_its_or_branch() -> Result<(), CardTextError>
{
    let tokens = lex_line(
        "If it's a land card or a creature card with mana value less than or equal to the number of loyalty counters on this planeswalker",
        0,
    )?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    let PredicateAst::Or(left, right) = parsed else {
        panic!("expected independent disjunctive filters, got {parsed:?}");
    };
    assert!(
        matches!(left.as_ref(), PredicateAst::ItMatches(filter)
            if filter.card_types == vec![CardType::Land]
                && filter.mana_value.is_none()),
        "expected unconstrained land branch, got {left:?}"
    );
    assert!(
        matches!(right.as_ref(), PredicateAst::ItMatches(filter)
            if filter.card_types == vec![CardType::Creature]
                && filter.mana_value.is_some()),
        "expected mana-value-constrained creature branch, got {right:?}"
    );
    Ok(())
}

#[test]
fn parse_predicate_keeps_comma_type_list_disjunctive() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If it's an artifact, creature, enchantment, or land card",
        0,
    )?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    match parsed {
        PredicateAst::Or(left, right) => {
            assert!(
                matches!(left.as_ref(), PredicateAst::ItMatches(filter)
                        if filter.card_types == vec![
                            CardType::Artifact,
                            CardType::Creature,
                            CardType::Enchantment,
                        ] && filter.all_card_types.is_empty()),
                "expected disjunctive permanent-type list on left, got {left:?}"
            );
            assert!(
                matches!(right.as_ref(), PredicateAst::ItMatches(filter)
                        if filter.card_types == vec![CardType::Land]
                            && filter.all_card_types.is_empty()),
                "expected land-card filter on right, got {right:?}"
            );
        }
        other => panic!("expected inherited-reference type-list predicate, got {other:?}"),
    }
    Ok(())
}

#[test]
fn parse_predicate_supports_if_you_dont_put_card_into_your_hand() -> Result<(), CardTextError> {
    let tokens = lex_line("If you don't put the card into your hand", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default().in_zone(Zone::Hand),
        }))
    );
    Ok(())
}

#[test]
fn parse_predicate_negative_put_tagged_object_uses_shared_capture_parser()
-> Result<(), CardTextError> {
    for (text, zone) in [
        ("If you did not put card into your hand", Zone::Hand),
        (
            "If you didn't put that card onto the battlefield",
            Zone::Battlefield,
        ),
        ("If you don't put it onto battlefield", Zone::Battlefield),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default().in_zone(zone),
            })),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_combat_damage_this_turn_uses_shared_capture_parser() -> Result<(), CardTextError>
{
    for (text, expected) in [
        (
            "if it dealt combat damage to a player this turn",
            PredicateAst::SourceDealtCombatDamageToPlayerThisTurn,
        ),
        (
            "if a player was dealt combat damage by a Zombie this turn",
            PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
                player: PlayerAst::Any,
                subtype: parse_subtype_word("zombie").expect("known subtype"),
            },
        ),
        (
            "if an opponent was dealt combat damage by a Dragon this turn",
            PredicateAst::PlayerWasDealtCombatDamageByCreatureSubtypeThisTurn {
                player: PlayerAst::Opponent,
                subtype: parse_subtype_word("dragon").expect("known subtype"),
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_supports_if_you_dont_put_it_into_your_hand() -> Result<(), CardTextError> {
    let tokens = lex_line("If you don't put it into your hand", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::Not(Box::new(PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter: ObjectFilter::default().in_zone(Zone::Hand),
        }))
    );
    Ok(())
}

#[test]
fn parse_predicate_passive_battlefield_this_way_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, filter_text) in [
        (
            "If an Equipment is put onto the battlefield this way",
            "an Equipment",
        ),
        ("If an Aura is put onto the battlefield this way", "an Aura"),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;
        let filter_tokens = lex_line(filter_text, 0)?;
        let mut filter = parse_object_filter(&filter_tokens, false)?;
        filter.zone = Some(Zone::Battlefield);

        assert_eq!(
            parsed,
            PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_chosen_name_milled_this_way_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If a card with the chosen name was milled this way", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let mut filter = ObjectFilter::default();
    filter.tagged_constraints.push(TaggedObjectConstraint {
        tag: TagKey::from(CHOSEN_NAME_TAG),
        relation: TaggedOpbjectRelation::SameNameAsTagged,
    });
    assert_eq!(
        parsed,
        PredicateAst::TaggedMatches(TagKey::from(IT_TAG), filter)
    );
    Ok(())
}

#[test]
fn parse_predicate_passive_sacrifice_keeps_event_reference() -> Result<(), CardTextError> {
    let tokens = lex_line("If a Saproling was sacrificed this way", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let PredicateAst::TaggedMatches(tag, filter) = parsed else {
        panic!("expected tagged sacrifice predicate");
    };
    assert_eq!(tag, TagKey::from(THIS_WAY_SACRIFICED_TAG));
    assert_eq!(filter.subtypes, vec![Subtype::Saproling]);
    Ok(())
}

#[test]
fn parse_predicate_supports_you_put_filtered_object_onto_battlefield_this_way()
-> Result<(), CardTextError> {
    let tokens = lex_line("If you put an artifact onto the battlefield this way", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let filter_tokens = lex_line("an artifact", 0)?;
    let mut filter = parse_object_filter(&filter_tokens, false)?;
    filter.zone = Some(Zone::Battlefield);
    assert_eq!(
        parsed,
        PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::You,
            tag: TagKey::from(IT_TAG),
            filter,
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_supports_that_player_discards_filtered_card_this_way()
-> Result<(), CardTextError> {
    let tokens = lex_line("If that player discards an artifact card this way", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;
    let artifact_filter_tokens = lex_line("an artifact card", 0)?;
    let mut artifact_filter = parse_object_filter(&artifact_filter_tokens, false)?;
    artifact_filter.zone = None;

    assert_eq!(
        parsed,
        PredicateAst::PlayerTaggedObjectMatches {
            player: PlayerAst::That,
            tag: TagKey::from(IT_TAG),
            filter: artifact_filter,
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_supports_you_would_draw_card() -> Result<(), CardTextError> {
    let tokens = lex_line("If you would draw a card", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;
    assert_eq!(
        parsed,
        PredicateAst::PlayerWouldDrawCard {
            player: PlayerAst::You
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_player_would_actions_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you would draw a card",
            PredicateAst::PlayerWouldDrawCard {
                player: PlayerAst::You,
            },
        ),
        (
            "If an opponent would draw card",
            PredicateAst::PlayerWouldDrawCard {
                player: PlayerAst::Opponent,
            },
        ),
        (
            "If opponent would proliferate",
            PredicateAst::PlayerWouldProliferate {
                player: PlayerAst::Opponent,
            },
        ),
        (
            "If an opponent would begin an extra turn",
            PredicateAst::PlayerWouldBeginExtraTurn {
                player: PlayerAst::Opponent,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_attacking_own_control_meld_uses_capture_parser() -> Result<(), CardTextError> {
    for text in [
        "If this creature and a creature named Midnight Scavengers are attacking and you both own and control them",
        "If this and creature named Phyrexian Dragon Engine are attacking, and you both own and control them, exile them",
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::And(left, right) = parsed else {
            panic!("expected attacking own-control conjoined predicate for {text}");
        };
        for side in [left, right] {
            let PredicateAst::PlayerControls { player, filter } = *side else {
                panic!("expected controls predicate for {text}");
            };
            assert_eq!(player, PlayerAst::You, "{text}");
            assert_eq!(filter.controller, Some(PlayerFilter::You), "{text}");
            assert!(filter.attacking, "{text}");
        }
    }
    Ok(())
}

#[test]
fn parse_predicate_you_both_own_and_control_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If you both own and control this creature and a creature named Midnight Scavengers",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let PredicateAst::And(left, right) = parsed else {
        panic!("expected own-and-control conjoined predicate");
    };
    let PredicateAst::PlayerControls {
        player: left_player,
        filter: left_filter,
    } = *left
    else {
        panic!("expected left controls predicate");
    };
    let PredicateAst::PlayerControls {
        player: right_player,
        filter: right_filter,
    } = *right
    else {
        panic!("expected right controls predicate");
    };
    assert_eq!(left_player, PlayerAst::You);
    assert_eq!(right_player, PlayerAst::You);
    assert_eq!(left_filter.controller, Some(PlayerFilter::You));
    assert_eq!(right_filter.controller, Some(PlayerFilter::You));
    Ok(())
}

#[test]
fn parse_predicate_implicit_subject_and_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_right) in [
        (
            "If you're monarch and you have the initiative",
            PredicateAst::PlayerHasInitiative {
                player: PlayerAst::You,
            },
        ),
        (
            "If you're monarch and have the initiative",
            PredicateAst::PlayerHasInitiative {
                player: PlayerAst::You,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        assert_eq!(
            parsed,
            PredicateAst::And(
                Box::new(PredicateAst::PlayerIsMonarch {
                    player: PlayerAst::You,
                }),
                Box::new(expected_right),
            ),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_while_conjoined_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If you would draw a card while you have no cards in hand",
        0,
    )?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::And(
            Box::new(PredicateAst::PlayerWouldDrawCard {
                player: PlayerAst::You,
            }),
            Box::new(PredicateAst::YouHaveNoCardsInHand),
        )
    );
    Ok(())
}

#[test]
fn parse_predicate_cards_in_hand_counts_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you have no cards in hand",
            PredicateAst::YouHaveNoCardsInHand,
        ),
        (
            "If you have one or fewer cards in hand",
            PredicateAst::PlayerCardsInHandOrFewer {
                player: PlayerAst::You,
                count: 1,
            },
        ),
        (
            "If an opponent has three or more cards in hand",
            PredicateAst::PlayerCardsInHandOrMore {
                player: PlayerAst::Opponent,
                count: 3,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_cards_in_hand_relations_use_shared_capture_parser() -> Result<(), CardTextError>
{
    for (text, expected) in [
        (
            "If an opponent has more cards in hand than you",
            PredicateAst::PlayerHasMoreCardsInHandThanYou {
                player: PlayerAst::Opponent,
            },
        ),
        (
            "If a player has more cards in hand than each other player",
            PredicateAst::PlayerHasMoreCardsInHandThanEachOtherPlayer {
                player: PlayerAst::Any,
            },
        ),
        (
            "If that player has more cards in their hand than you do",
            PredicateAst::PlayerHasMoreCardsInHandThanYou {
                player: PlayerAst::That,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_turn_event_counts_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you drew two or more cards this turn",
            PredicateAst::ValueComparison {
                left: Value::MaxCardsDrawnThisTurn(PlayerFilter::You),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(2),
            },
        ),
        (
            "If an opponent has drawn three cards this turn",
            PredicateAst::ValueComparison {
                left: Value::MaxCardsDrawnThisTurn(PlayerFilter::Opponent),
                operator: ValueComparisonOperator::Equal,
                right: Value::Fixed(3),
            },
        ),
        (
            "If that player had two or fewer lands entered battlefield under their control this turn",
            PredicateAst::ValueComparison {
                left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                operator: ValueComparisonOperator::LessThanOrEqual,
                right: Value::Fixed(2),
            },
        ),
        (
            "If that player had two or more lands enter the battlefield under their control this turn",
            PredicateAst::ValueComparison {
                left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(2),
            },
        ),
        (
            "If that player had another land enter the battlefield under their control this turn",
            PredicateAst::ValueComparison {
                left: Value::LandsEnteredBattlefieldThisTurn(PlayerFilter::IteratedPlayer),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(2),
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_spell_context_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If that spells controller poisoned",
            PredicateAst::TargetSpellControllerIsPoisoned,
        ),
        (
            "If no mana was spent to cast that spell",
            PredicateAst::TargetSpellNoManaSpentToCast,
        ),
        (
            "If you control more creatures than its controller",
            PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
        ),
        (
            "If you control more creatures than that spell's controller",
            PredicateAst::YouControlMoreCreaturesThanTargetSpellController,
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_tagged_state_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_filter) in [
        (
            "If that permanent is black",
            ObjectFilter {
                colors: Some(ColorSet::BLACK),
                ..Default::default()
            },
        ),
        (
            "If it's blocking",
            ObjectFilter {
                blocking: true,
                ..Default::default()
            },
        ),
        (
            "If that creature is attacking",
            ObjectFilter {
                attacking: true,
                ..Default::default()
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(parsed, PredicateAst::ItMatches(expected_filter), "{text}");
    }

    for (text, expected) in [
        (
            "If those cards remain exiled",
            PredicateAst::TaggedMatches(
                TagKey::from(IT_TAG),
                ObjectFilter::default().in_zone(Zone::Exile),
            ),
        ),
        (
            "If it is paired with another creature",
            PredicateAst::ItIsSoulbondPaired,
        ),
        (
            "If it's paired with another creature",
            PredicateAst::ItIsSoulbondPaired,
        ),
        (
            "If it's paired with a creature",
            PredicateAst::ItIsSoulbondPaired,
        ),
        (
            "If you controlled that permanent",
            PredicateAst::PlayerTaggedObjectMatches {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
                filter: ObjectFilter::default(),
            },
        ),
        (
            "If that card entered under your control",
            PredicateAst::PlayerTaggedObjectEnteredBattlefieldThisTurn {
                player: PlayerAst::You,
                tag: TagKey::from(IT_TAG),
            },
        ),
        (
            "If that creature was not blocking",
            PredicateAst::TaggedMatches(
                TagKey::from(IT_TAG),
                ObjectFilter {
                    nonblocking: true,
                    ..Default::default()
                },
            ),
        ),
        (
            "If that creature was blue or black",
            PredicateAst::TaggedMatches(
                TagKey::from(IT_TAG),
                ObjectFilter {
                    colors: Some(ColorSet::BLUE.union(ColorSet::BLACK)),
                    ..Default::default()
                },
            ),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(parsed, expected, "{text}");
    }

    let tokens = lex_line("If enchanted creature is a Zombie", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    match parsed {
        PredicateAst::TaggedMatches(tag, filter) => {
            assert_eq!(tag, TagKey::from("enchanted"));
            assert!(
                !filter.subtypes.is_empty() || !filter.card_types.is_empty(),
                "{filter:?}"
            );
        }
        other => panic!("expected enchanted tagged predicate, got {other:?}"),
    }
    Ok(())
}

#[test]
fn parse_predicate_attached_tagged_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for text in [
        "If this permanent is attached to a creature",
        "If that permanent attached to an artifact creature",
        "If this permanent attached to an enchantment creature",
        "If that permanent is attached to a land creature",
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        match parsed {
            PredicateAst::TaggedMatches(tag, filter) => {
                assert_eq!(tag, TagKey::from("enchanted"), "{text}");
                assert!(!filter.card_types.is_empty(), "{text}: {filter:?}");
            }
            other => panic!("expected attached tagged predicate for {text}, got {other:?}"),
        }
    }
    Ok(())
}

#[test]
fn parse_predicate_mana_spent_uses_shared_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If {S} was spent to cast this spell", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert!(
        matches!(
            parsed,
            PredicateAst::ManaSpentToCastThisSpellAtLeast {
                amount: 1,
                symbol: Some(_),
            }
        ),
        "{parsed:?}"
    );

    let tokens = lex_line("If {R}{G} was spent to cast this spell", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert!(matches!(parsed, PredicateAst::And(_, _)), "{parsed:?}");

    let tokens = lex_line(
        "If at least three blue mana was spent to cast this spell",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert!(
        matches!(
            parsed,
            PredicateAst::ManaSpentToCastThisSpellAtLeast {
                amount: 3,
                symbol: Some(_),
            }
        ),
        "{parsed:?}"
    );

    let tokens = lex_line("If at least four mana was spent to cast it", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert!(
        matches!(
            parsed,
            PredicateAst::ManaSpentToCastThisSpellAtLeast {
                amount: 4,
                symbol: None,
            }
        ),
        "{parsed:?}"
    );
    Ok(())
}

#[test]
fn parse_predicate_preserves_mana_source_provenance() -> Result<(), CardTextError> {
    let tokens = lex_line("If mana from a Treasure was spent to cast it", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let PredicateAst::ValueComparison {
        left:
            Value::ManaFromSourceSpentToCastThisSpell {
                source_filter,
                include_source_noun,
            },
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    } = parsed
    else {
        panic!("expected a typed mana-source predicate, got {parsed:?}");
    };
    assert!(!include_source_noun);
    assert!(source_filter.subtypes.contains(&Subtype::Treasure));
    Ok(())
}

#[test]
fn parse_predicate_spell_lifecycle_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        ("If you cast this spell", PredicateAst::SourceWasCast),
        (
            "If it was cast",
            PredicateAst::TaggedWasCast(TagKey::from(IT_TAG)),
        ),
        (
            "If this spell was cast from a graveyard",
            PredicateAst::ThisSpellWasCastFromZone(Zone::Graveyard),
        ),
        (
            "If this spell was cast from anywhere other than your hand",
            PredicateAst::ThisSpellWasCastFromNonHand,
        ),
        (
            "If you cast it from your hand",
            PredicateAst::ThisSpellWasCastFromZone(Zone::Hand),
        ),
        (
            "If you cast this spell from anywhere other than your hand",
            PredicateAst::ThisSpellWasCastFromNonHand,
        ),
        (
            "If no spells were cast last turn",
            PredicateAst::NoSpellsWereCastLastTurn,
        ),
        ("If this spell was kicked", PredicateAst::ThisSpellWasKicked),
        (
            "If this spell was bargained",
            PredicateAst::ThisSpellPaidLabel("Bargain".into()),
        ),
        (
            "If it was bargained",
            PredicateAst::ThisSpellPaidLabel("Bargain".into()),
        ),
        (
            "If gift was promised",
            PredicateAst::ThisSpellPaidLabel("Gift".into()),
        ),
        (
            "If the gift was promised",
            PredicateAst::ThisSpellPaidLabel("Gift".into()),
        ),
        (
            "If gift was not promised",
            PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("Gift".into()))),
        ),
        (
            "If tribute was not paid",
            PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("Tribute".into()))),
        ),
        (
            "If tribute wasn't paid",
            PredicateAst::Not(Box::new(PredicateAst::ThisSpellPaidLabel("Tribute".into()))),
        ),
        ("If that was kicked", PredicateAst::TargetWasKicked),
        ("If that spell was kicked", PredicateAst::TargetWasKicked),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_conjoins_cast_origin_with_existential_count() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If you cast it from your hand and there are five or more other creatures on the battlefield",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    assert!(
        matches!(
            &parsed,
            PredicateAst::And(left, right)
                if matches!(**left, PredicateAst::ThisSpellWasCastFromZone(Zone::Hand))
                    && matches!(**right, PredicateAst::ValueComparison { .. })
        ),
        "{parsed:?}"
    );
    Ok(())
}

#[test]
fn parse_predicate_combat_turn_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you attacked this turn",
            PredicateAst::YouAttackedThisTurn,
        ),
        (
            "If that creature had to attack this combat",
            PredicateAst::TriggeringObjectHadToAttackThisCombat,
        ),
        (
            "If you attacked with exactly two other creatures this combat",
            PredicateAst::YouAttackedWithExactlyNOtherCreaturesThisCombat(2),
        ),
        (
            "If this creature attacked or blocked this turn",
            PredicateAst::SourceAttackedOrBlockedThisTurn,
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_negative_attack_history_gates() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If this creature didn't attack this turn",
            PredicateAst::Not(Box::new(PredicateAst::SourceAttackedThisTurn)),
        ),
        (
            "If this creature did not attack this turn",
            PredicateAst::Not(Box::new(PredicateAst::SourceAttackedThisTurn)),
        ),
        (
            "If you didn't attack with a creature this turn",
            PredicateAst::Not(Box::new(PredicateAst::YouAttackedThisTurn)),
        ),
        (
            "If you did not attack with a creature this turn",
            PredicateAst::Not(Box::new(PredicateAst::YouAttackedThisTurn)),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_spell_cast_this_turn_uses_shared_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If you cast another spell this turn", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert_eq!(
        parsed,
        PredicateAst::PlayerCastSpellsThisTurnOrMore {
            player: PlayerAst::You,
            count: 2,
        }
    );

    let tokens = lex_line("If opponent has cast a creature spell this turn", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let PredicateAst::ValueComparison {
        left:
            Value::SpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source,
            },
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    } = parsed
    else {
        panic!("expected spell-cast matching predicate, got {parsed:?}");
    };
    assert_eq!(player, PlayerFilter::Opponent);
    assert!(!exclude_source);
    assert!(filter.card_types.contains(&CardType::Creature));

    let tokens = lex_line("If you didnt cast a noncreature spell this turn", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert!(
        matches!(&parsed, PredicateAst::Not(inner) if matches!(
            inner.as_ref(),
            PredicateAst::ValueComparison {
                left: Value::SpellsCastThisTurnMatching { player: PlayerFilter::You, .. },
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            }
        )),
        "expected negated spell-cast matching predicate, got {parsed:?}"
    );

    let tokens = lex_line("If you haven't cast a spell from your hand this turn", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let PredicateAst::Not(inner) = parsed else {
        panic!("expected negated hand-origin spell-cast predicate, got {parsed:?}");
    };
    let PredicateAst::ValueComparison {
        left:
            Value::SpellsCastThisTurnMatching {
                player,
                filter,
                exclude_source,
            },
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(1),
    } = *inner
    else {
        panic!("expected hand-origin spell-cast value comparison, got {inner:?}");
    };
    assert_eq!(player, PlayerFilter::You);
    assert_eq!(filter.zone, Some(Zone::Hand));
    assert!(!exclude_source);

    Ok(())
}

#[test]
fn parse_predicate_supports_you_would_proliferate() -> Result<(), CardTextError> {
    let tokens = lex_line("If you would proliferate", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;
    assert_eq!(
        parsed,
        PredicateAst::PlayerWouldProliferate {
            player: PlayerAst::You
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_supports_you_have_more_life_than_opponent() -> Result<(), CardTextError> {
    let tokens = lex_line("if you have more life than an opponent", 0)?;

    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::PlayerHasLessLifeThanYou {
            player: PlayerAst::Opponent,
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_life_relations_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "if an opponent has more life than you",
            PredicateAst::PlayerHasMoreLifeThanYou {
                player: PlayerAst::Opponent,
            },
        ),
        (
            "if you have more life than each opponent",
            PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                player: PlayerAst::You,
            },
        ),
        (
            "if no opponent has more life than that player",
            PredicateAst::PlayerHasNoOpponentWithMoreLifeThan {
                player: PlayerAst::That,
            },
        ),
        (
            "if a player has more life than each other player",
            PredicateAst::PlayerHasMoreLifeThanEachOtherPlayer {
                player: PlayerAst::Any,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_life_totals_use_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you have five or less life",
            PredicateAst::ValueComparison {
                left: crate::effect::Value::LifeTotal(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: crate::effect::Value::Fixed(5),
            },
        ),
        (
            "If your life total is five or less",
            PredicateAst::ValueComparison {
                left: crate::effect::Value::LifeTotal(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::LessThanOrEqual,
                right: crate::effect::Value::Fixed(5),
            },
        ),
        (
            "If an opponent has ten or more life",
            PredicateAst::ValueComparison {
                left: crate::effect::Value::LifeTotal(PlayerFilter::Opponent),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: crate::effect::Value::Fixed(10),
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_life_change_this_turn_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If you gained life this turn",
            PredicateAst::PlayerGainedLifeThisTurnOrMore {
                player: PlayerAst::You,
                count: 1,
            },
        ),
        (
            "If you gained three or more life this turn",
            PredicateAst::PlayerGainedLifeThisTurnOrMore {
                player: PlayerAst::You,
                count: 3,
            },
        ),
        (
            "If you lost two or more life this turn",
            PredicateAst::ValueComparison {
                left: Value::LifeLostThisTurn(PlayerFilter::You),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(2),
            },
        ),
        (
            "If one or more opponents lost life this turn",
            PredicateAst::OpponentLostLifeThisTurn,
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_ring_bearer_temptation_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If this creature is your Ring-bearer",
            PredicateAst::SourceIsRingBearer {
                player: PlayerAst::You,
            },
        ),
        (
            "If Ring has tempted you one or more time this game",
            PredicateAst::PlayerRingTemptedThisGameOrMore {
                player: PlayerAst::You,
                count: 1,
            },
        ),
        (
            "If this is your Ring-bearer and the Ring has tempted you two or more times this game",
            PredicateAst::And(
                Box::new(PredicateAst::SourceIsRingBearer {
                    player: PlayerAst::You,
                }),
                Box::new(PredicateAst::PlayerRingTemptedThisGameOrMore {
                    player: PlayerAst::You,
                    count: 2,
                }),
            ),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_supports_creature_card_put_into_your_graveyard_this_turn()
-> Result<(), CardTextError> {
    let tokens = lex_line(
        "If a creature card was put into your graveyard from anywhere this turn",
        0,
    )?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn
    );
    Ok(())
}

#[test]
fn parse_predicate_supports_descended_this_turn() -> Result<(), CardTextError> {
    for (text, expected_player) in [
        ("If you descended this turn", PlayerAst::You),
        ("If that player descended this turn", PlayerAst::That),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        assert_eq!(
            parse_predicate(&predicate_tokens)?,
            PredicateAst::PlayerDescendedThisTurn {
                player: expected_player,
            },
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_battlefield_change_this_turn_uses_shared_capture_parser()
-> Result<(), CardTextError> {
    let cases = [
        (
            "If no permanents left battlefield this turn",
            PredicateAst::Not(Box::new(PredicateAst::PermanentLeftBattlefieldThisTurn)),
        ),
        (
            "If a permanent left battlefield this turn",
            PredicateAst::PermanentLeftBattlefieldThisTurn,
        ),
        (
            "If a nonland permanent left the battlefield this turn or a spell was warped this turn",
            PredicateAst::Or(
                Box::new(PredicateAst::NonlandPermanentLeftBattlefieldThisTurn),
                Box::new(PredicateAst::SpellWasWarpedThisTurn),
            ),
        ),
        (
            "If creatures left battlefield under your control this turn",
            PredicateAst::PermanentLeftBattlefieldUnderYourControlThisTurn,
        ),
        (
            "If lands you controlled were put into graveyard from battlefield this turn",
            PredicateAst::ObjectPutIntoGraveyardFromBattlefieldThisTurn(
                ObjectFilter::land().controlled_by(PlayerFilter::You),
            ),
        ),
    ];

    for (text, expected) in cases {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_object_death_this_turn_uses_shared_capture_parser() -> Result<(), CardTextError>
{
    let cases = [
        (
            "If a creature died this turn",
            PredicateAst::CreatureDiedThisTurn,
        ),
        (
            "If seven or more creatures died this turn",
            PredicateAst::CreatureDiedThisTurnOrMore(7),
        ),
        (
            "If a creature died under your control this turn",
            PredicateAst::ValueComparison {
                left: Value::CreaturesDiedThisTurnControlledBy(PlayerFilter::You),
                operator: crate::effect::ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(1),
            },
        ),
        (
            "If a creature card was put into your graveyard from anywhere this turn",
            PredicateAst::CreatureCardPutIntoYourGraveyardThisTurn,
        ),
    ];

    for (text, expected) in cases {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_battlefield_entry_uses_shared_capture_parser() -> Result<(), CardTextError> {
    let cases = [
        (
            "If you had another creature entered the battlefield under your control last turn",
            PredicateAst::ObjectEnteredBattlefieldLastTurn(
                ObjectFilter::creature()
                    .controlled_by(PlayerFilter::You)
                    .other(),
            ),
        ),
        (
            "If artifacts entered battlefield under your control this turn",
            PredicateAst::ObjectEnteredBattlefieldThisTurn(
                ObjectFilter::artifact().controlled_by(PlayerFilter::You),
            ),
        ),
        (
            "If you had lands entered battlefield under your control this turn",
            PredicateAst::PlayerHadLandEnterBattlefieldThisTurn {
                player: PlayerAst::You,
            },
        ),
    ];

    for (text, expected) in cases {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_card_in_your_graveyard_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If there is an Elf card in your graveyard", 0)?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    let mut expected_filter = ObjectFilter::default()
        .with_subtype(parse_subtype_word("elf").expect("elf subtype"))
        .in_zone(Zone::Graveyard);
    expected_filter.owner = Some(PlayerFilter::You);
    assert_eq!(
        parsed,
        PredicateAst::PlayerControls {
            player: PlayerAst::You,
            filter: expected_filter,
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_targets_only_source_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_card_types) in [
        (
            "If that spell targets only this creature",
            vec![CardType::Creature],
        ),
        ("If spell targets only this permanent", vec![]),
        ("If it targets only it", vec![]),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::ItMatches(filter) = parsed else {
            panic!("expected spell target predicate for {text}");
        };
        assert_eq!(filter.zone, Some(Zone::Stack), "{text}");
        assert_eq!(filter.stack_kind, Some(StackObjectKind::Spell), "{text}");
        assert_eq!(filter.target_count, Some(ChoiceCount::exactly(1)), "{text}");
        let Some(target_filter) = filter.targets_only_object.as_deref() else {
            panic!("expected targets-only object filter for {text}");
        };
        assert!(target_filter.source, "{text}");
        assert_eq!(target_filter.zone, Some(Zone::Battlefield), "{text}");
        assert_eq!(target_filter.card_types, expected_card_types, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_stack_object_targets_object_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If that spell targets a commander you control", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let PredicateAst::ItMatches(filter) = parsed else {
        panic!("expected spell targeting predicate");
    };
    assert_eq!(filter.zone, Some(Zone::Stack));
    assert_eq!(filter.stack_kind, Some(StackObjectKind::Spell));
    let Some(target_filter) = filter.targets_object.as_deref() else {
        panic!("expected targeted object filter");
    };
    assert!(target_filter.is_commander, "{target_filter:?}");
    assert_eq!(target_filter.controller, Some(PlayerFilter::You));
    Ok(())
}

#[test]
fn parse_predicate_source_zone_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_zone) in [
        ("If this card is in your hand", Zone::Hand),
        ("If this creature is in your graveyard", Zone::Graveyard),
        ("If this is in exile", Zone::Exile),
        ("If this card is in the command zone", Zone::Command),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        assert_eq!(
            parsed,
            PredicateAst::SourceIsInZone(expected_zone),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_behold_or_controlled_subtype_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If you revealed a Dragon card or controlled a Dragon as you cast this spell",
        0,
    )?;
    let predicate_tokens = predicate_tokens_after_if(&tokens);

    let parsed = parse_predicate(&predicate_tokens)?;

    assert_eq!(
        parsed,
        PredicateAst::Or(
            Box::new(PredicateAst::ThisSpellPaidLabel("Behold".into())),
            Box::new(PredicateAst::PlayerControls {
                player: PlayerAst::You,
                filter: ObjectFilter::default()
                    .with_subtype(parse_subtype_word("dragon").expect("dragon subtype")),
            }),
        )
    );
    Ok(())
}

#[test]
fn parse_predicate_triggering_object_counters_use_shared_capture_parser()
-> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If it had no stun counters on it",
            PredicateAst::TriggeringObjectHadNoCounter(CounterType::Stun),
        ),
        (
            "If that creature had a +1/+1 counter on it",
            PredicateAst::TriggeringObjectHadCounterAtLeast {
                counter_type: CounterType::PlusOnePlusOne,
                count: 1,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_controls_more_than_you_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_player, expected_filter) in [
        (
            "If an opponent controls more creatures than you",
            PlayerAst::Opponent,
            ObjectFilter::creature(),
        ),
        (
            "If target opponent controls more artifacts than you do",
            PlayerAst::TargetOpponent,
            ObjectFilter::artifact(),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerControlsMoreThanYou {
                player: expected_player,
                filter: expected_filter,
            },
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_graveyard_card_counts_use_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line("If you have seven or more cards in your graveyard", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert_eq!(
        parsed,
        PredicateAst::PlayerHasAtLeast {
            player: PlayerAst::You,
            filter: ObjectFilter {
                zone: Some(Zone::Graveyard),
                ..Default::default()
            },
            count: 7,
        }
    );

    let tokens = lex_line("If twenty or more creature cards are in your graveyard", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(20),
    } = parsed
    else {
        panic!("expected quantified graveyard object-count predicate, got {parsed:?}");
    };
    assert_eq!(filter.zone, Some(Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));
    assert!(filter.card_types.contains(&CardType::Creature));

    for (text, expected_player, expected_operator, expected_count) in [
        (
            "If an opponent has fewer than three cards in their graveyard",
            PlayerFilter::Opponent,
            ValueComparisonOperator::LessThan,
            3,
        ),
        (
            "If target opponent has exactly two card in their graveyard",
            PlayerFilter::target_opponent(),
            ValueComparisonOperator::Equal,
            2,
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ValueComparison {
                left: Value::CardsInGraveyard(expected_player),
                operator: expected_operator,
                right: Value::Fixed(expected_count),
            },
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_colors_among_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_count) in [
        ("If there are five colors among permanents you control", 5),
        ("If there were one color among permanent you control", 1),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::ValueComparison {
                left: Value::ColorsAmong(ObjectFilter::permanent().you_control()),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(expected_count),
            },
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_counted_source_exiled_objects_uses_capture_parser() -> Result<(), CardTextError>
{
    for (text, expected_count, expected_card_type) in [
        (
            "If three or more cards have been exiled with this artifact",
            3,
            None,
        ),
        (
            "If exactly two creature cards have been exiled with this",
            2,
            Some(CardType::Creature),
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

        let PredicateAst::ValueComparison {
            left: Value::Count(filter),
            right: Value::Fixed(count),
            ..
        } = parsed
        else {
            panic!("expected counted source-exiled predicate for {text}");
        };
        assert_eq!(count, expected_count, "{text}");
        assert_eq!(filter.zone, Some(Zone::Exile), "{text}");
        assert!(
            filter
                .tagged_constraints
                .iter()
                .any(|constraint| constraint.tag.as_str() == crate::tag::SOURCE_EXILED_TAG),
            "{text}"
        );
        if let Some(card_type) = expected_card_type {
            assert!(filter.card_types.contains(&card_type), "{text}");
        }
    }
    Ok(())
}

#[test]
fn parse_predicate_counted_objects_with_counters_uses_capture_parser() -> Result<(), CardTextError>
{
    let tokens = lex_line("If two or more creatures have +1/+1 counters", 0)?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;

    let PredicateAst::ValueComparison {
        left: Value::Count(filter),
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(2),
    } = parsed
    else {
        panic!("expected counted object-with-counter predicate");
    };
    assert_eq!(filter.card_types, vec![CardType::Creature]);
    assert_eq!(filter.zone, Some(Zone::Battlefield));
    assert!(filter.with_counter.is_some());
    Ok(())
}

#[test]
fn parse_predicate_card_types_among_uses_capture_parser() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If there are six or more card types among permanents you control and/or cards in your graveyard",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    let PredicateAst::ValueComparison {
        left: Value::CardTypesAmong(filter),
        operator: ValueComparisonOperator::GreaterThanOrEqual,
        right: Value::Fixed(6),
    } = parsed
    else {
        panic!("expected card-types-among value comparison, got {parsed:#?}");
    };
    assert_eq!(filter.any_of.len(), 2);
    assert!(
        filter
            .any_of
            .contains(&ObjectFilter::permanent().you_control())
    );
    assert!(filter.any_of.iter().any(|filter| {
        filter.zone == Some(Zone::Graveyard) && filter.owner == Some(PlayerFilter::You)
    }));

    let tokens = lex_line(
        "If there are two or more card types among sacrificed permanents",
        0,
    )?;
    let parsed = parse_predicate(&predicate_tokens_after_if(&tokens))?;
    assert_eq!(
        parsed,
        PredicateAst::ValueComparison {
            left: Value::CardTypesAmong(ObjectFilter::tagged("sacrificed_0")),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_graveyard_card_types_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_player, expected_count) in [
        (
            "If there are six or more card types among cards in your graveyard",
            PlayerAst::You,
            6,
        ),
        (
            "If you have four or more card types among cards in your graveyard",
            PlayerAst::You,
            4,
        ),
        (
            "If there are three or more card type among card in target player's graveyard",
            PlayerAst::Target,
            3,
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::PlayerHasCardTypesInGraveyardOrMore {
                player: expected_player,
                count: expected_count,
            },
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_basic_land_types_uses_capture_parser() -> Result<(), CardTextError> {
    for (text, expected) in [
        (
            "If there are two or more basic land types among lands you control",
            PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                player: PlayerAst::You,
                count: 2,
            },
        ),
        (
            "If there are three basic land types among lands that player controls",
            PredicateAst::PlayerControlsBasicLandTypesAmongLandsOrMore {
                player: PlayerAst::That,
                count: 3,
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }
    Ok(())
}

#[test]
fn parse_predicate_source_counters_use_shared_capture_parser() -> Result<(), CardTextError> {
    let counted_counter_tokens = lex_line("If it three or more +1/+1 counters on it", 0)?;
    assert_eq!(
        parse_source_verbless_counted_counter_predicate(&predicate_tokens_after_if(
            &counted_counter_tokens
        )),
        Some(PredicateAst::ValueComparison {
            left: Value::CountersOn(
                Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                Some(CounterType::PlusOnePlusOne),
            ),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(3),
        })
    );

    for (text, expected) in [
        (
            "If this has no stun counters on it",
            PredicateAst::SourceHasNoCounter(CounterType::Stun),
        ),
        (
            "If there are no more scream counters on it",
            PredicateAst::SourceHasNoCounter(CounterType::Named("scream")),
        ),
        (
            "If there are two counters on this creature",
            PredicateAst::SourceHasCountersAtLeast(2),
        ),
        (
            "If there are three stun counters on this",
            PredicateAst::SourceHasCounterAtLeast {
                counter_type: CounterType::Stun,
                count: 3,
                surface: crate::SourceCounterThresholdSurface::ThereAreOn(
                    crate::target::SourceReferenceSurface::ThisPermanentType("this".to_string()),
                ),
            },
        ),
        (
            "If this creature has a +1/+1 counter on it",
            PredicateAst::SourceHasCounterAtLeast {
                counter_type: CounterType::PlusOnePlusOne,
                count: 1,
                surface: crate::SourceCounterThresholdSurface::SourceHas,
            },
        ),
        (
            "If this creature doesn't have a flying counter on it",
            PredicateAst::SourceHasNoCounter(CounterType::Flying),
        ),
        (
            "If this creature has two stun counters on it",
            PredicateAst::SourceHasCounterAtLeast {
                counter_type: CounterType::Stun,
                count: 2,
                surface: crate::SourceCounterThresholdSurface::SourceHas,
            },
        ),
        (
            "If it has three or more +1/+1 counters on it",
            PredicateAst::ValueComparison {
                left: Value::CountersOn(
                    Box::new(crate::target::ChooseSpec::Tagged(TagKey::from(IT_TAG))),
                    Some(CounterType::PlusOnePlusOne),
                ),
                operator: ValueComparisonOperator::GreaterThanOrEqual,
                right: Value::Fixed(3),
            },
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(parsed, expected, "{text}");
    }

    crate::runtime_backend::front_end::shared::util::with_source_reference_context(
        "Sarulf, Realm Eater",
        || {
            let tokens = lex_line("If Sarulf has one or more +1/+1 counters on it", 0)?;
            let predicate_tokens = predicate_tokens_after_if(&tokens);

            let parsed = parse_predicate(&predicate_tokens)?;

            assert_eq!(
                parsed,
                PredicateAst::SourceHasCounterAtLeast {
                    counter_type: CounterType::PlusOnePlusOne,
                    count: 1,
                    surface: crate::SourceCounterThresholdSurface::SourceHas,
                }
            );
            Ok::<(), CardTextError>(())
        },
    )?;
    Ok(())
}

#[test]
fn parse_predicate_source_power_uses_shared_capture_parser() -> Result<(), CardTextError> {
    for (text, expected_count) in [
        ("If this has power 7 or greater", 7),
        ("If this creature's power is 1 or more", 1),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        assert_eq!(
            parsed,
            PredicateAst::SourcePowerAtLeast(expected_count),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_supports_source_has_keyword() -> Result<(), CardTextError> {
    for (text, ability) in [
        (
            "If this creature has defender",
            crate::static_abilities::StaticAbilityId::Defender,
        ),
        (
            "If this source has flying",
            crate::static_abilities::StaticAbilityId::Flying,
        ),
    ] {
        let tokens = lex_line(text, 0)?;
        let predicate_tokens = predicate_tokens_after_if(&tokens);

        let parsed = parse_predicate(&predicate_tokens)?;

        let mut expected_filter = ObjectFilter::default();
        expected_filter.static_abilities.push(ability);
        assert_eq!(
            parsed,
            PredicateAst::SourceMatches(expected_filter),
            "{text}"
        );
    }
    Ok(())
}

#[test]
fn parse_predicate_supports_player_life_tie_count() -> Result<(), CardTextError> {
    let tokens = lex_line("If two or more players are tied for lowest life total", 0)?;
    assert_eq!(
        parse_predicate(&predicate_tokens_after_if(&tokens))?,
        PredicateAst::ValueComparison {
            left: Value::CountPlayers(PlayerFilter::LowestLifeTied),
            operator: ValueComparisonOperator::GreaterThanOrEqual,
            right: Value::Fixed(2),
        }
    );
    Ok(())
}

#[test]
fn parse_predicate_compares_object_count_with_source_counter_count() -> Result<(), CardTextError> {
    let tokens = lex_line(
        "If the number of attacking creatures is greater than the number of quest counters on this creature",
        0,
    )?;
    let mut attacking_creatures = ObjectFilter::creature();
    attacking_creatures.attacking = true;

    let PredicateAst::ValueComparison {
        left,
        operator,
        right,
    } = parse_predicate(&predicate_tokens_after_if(&tokens))?
    else {
        panic!("expected a value comparison predicate");
    };
    assert_eq!(left, Value::Count(attacking_creatures));
    assert_eq!(operator, ValueComparisonOperator::GreaterThan);
    let Value::CountersOn(spec, Some(CounterType::Quest)) = right else {
        panic!("expected quest counters on the source, got {right:?}");
    };
    assert!(matches!(spec.unhinted(), crate::target::ChooseSpec::Source));
    Ok(())
}
