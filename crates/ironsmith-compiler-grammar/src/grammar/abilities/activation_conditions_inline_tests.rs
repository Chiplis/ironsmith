use super::super::super::super::lexer::lex_line;
use super::*;
use crate::cards::builders::PredicateAst;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn timing_and_frequency_shapes_return_typed_values() {
    assert_eq!(
        parse_activate_only_timing_lexed(&lex("Activate only during combat.")),
        Some(ActivationTiming::DuringCombat)
    );
    assert_eq!(
        parse_triggered_times_each_turn_from_words(&[
            "this", "ability", "triggers", "only", "two", "times", "each", "turn"
        ]),
        Some(2)
    );
    assert_eq!(
        parse_activation_count_per_turn(&["three", "times", "each", "turn"]),
        Some(3)
    );
}

#[test]
fn activation_conditions_preserve_existing_semantics() {
    assert_eq!(
        parse_activation_condition_lexed(&lex(
            "Activate only if creatures you control have total power 8 or greater."
        )),
        Some(PredicateAst::ControlCreaturesTotalPowerAtLeast(8))
    );
    assert_eq!(
        parse_activation_condition_lexed(&lex("Activate only twice each turn.")),
        Some(PredicateAst::MaxActivationsPerTurn(2))
    );
    assert!(matches!(
        parse_activation_condition_lexed(&lex(
            "Activate only if there are three or more brick counters on this artifact."
        )),
        Some(PredicateAst::SourceHasCounterAtLeast {
            counter_type: crate::CounterType::Named(counter_name),
            count: 3,
            ..
        }) if counter_name.as_str() == "brick"
    ));
    assert_eq!(
        parse_activation_condition_lexed(&lex("Activate only if this permanent is a creature.")),
        Some(PredicateAst::SourceMatches(ObjectFilter::creature()))
    );
}

#[test]
fn combined_once_and_turn_timing_keeps_both_constraints() {
    assert_eq!(
        parse_activation_condition_lexed(&lex(
            "Activate only during your turn and only once each turn."
        )),
        Some(PredicateAst::And(
            Box::new(PredicateAst::MaxActivationsPerTurn(1)),
            Box::new(PredicateAst::ActivationTiming(
                ActivationTiming::DuringYourTurn
            )),
        ))
    );
    assert_eq!(
        parse_activation_condition_lexed(&lex("Activate only once each turn.")),
        Some(PredicateAst::MaxActivationsPerTurn(1))
    );
}

#[test]
fn combined_once_and_owned_graveyard_threshold_keeps_both_constraints() {
    let parsed = parse_activation_condition_lexed(&lex(
        "Activate only once each turn and only if there are seven or more cards in your graveyard.",
    ))
    .expect("combined frequency and graveyard threshold should parse");
    let PredicateAst::And(frequency, threshold) = parsed else {
        panic!("expected a typed conjunction: {parsed:#?}");
    };
    assert_eq!(*frequency, PredicateAst::MaxActivationsPerTurn(1));
    let PredicateAst::PlayerHasAtLeast {
        player,
        filter,
        count,
    } = *threshold
    else {
        panic!("expected an owned-graveyard cardinality condition: {threshold:#?}");
    };
    assert_eq!(player, PlayerAst::You);
    assert_eq!(count, 7);
    assert_eq!(filter.zone, Some(crate::zone::Zone::Graveyard));
    assert_eq!(filter.owner, Some(PlayerFilter::You));

    assert!(
        parse_activation_condition_lexed(&lex(
            "Activate only once each turn and only if there are seven cards in a graveyard.",
        ))
        .is_none(),
        "the owned-graveyard threshold must not claim a different zone-owner surface"
    );
}

#[test]
fn activation_condition_composes_repeated_or_if_with_typed_source_and_basic_land() {
    let parsed = parse_activation_condition_lexed(&lex(
        "Activate only if this land entered this turn or if you control a basic land.",
    ))
    .expect("repeated or-if activation condition should parse");

    let PredicateAst::Or(left, right) = parsed else {
        panic!("expected a typed disjunction");
    };
    let PredicateAst::ObjectEnteredBattlefieldThisTurn(source_filter) = left.as_ref() else {
        panic!("expected source-entered-this-turn left branch, got {left:?}");
    };
    assert!(source_filter.source);
    assert_eq!(
        source_filter.source_surface,
        Some(crate::target::SourceReferenceSurface::ThisPermanentType(
            "this land".to_string()
        ))
    );

    let PredicateAst::YouControl(basic_land_filter) = right.as_ref() else {
        panic!("expected basic-land control right branch, got {right:?}");
    };
    assert!(
        basic_land_filter
            .card_types
            .contains(&crate::types::CardType::Land)
    );
    assert!(
        basic_land_filter
            .supertypes
            .contains(&crate::types::Supertype::Basic)
    );
}

#[test]
fn activation_condition_or_if_composition_reuses_existing_branch_parsers() {
    let parsed = parse_activation_condition_lexed(&lex(
        "Activate only if you control an artifact or if you control a creature.",
    ))
    .expect("generic repeated or-if control branches should parse");

    assert!(matches!(parsed, PredicateAst::Or(_, _)));
    assert!(matches!(
        parse_activation_condition_lexed(&lex("Activate only if you control a Plains or a Swamp.")),
        Some(PredicateAst::PlayerHasAtLeast { .. }) | Some(PredicateAst::Or(_, _))
    ));
}
