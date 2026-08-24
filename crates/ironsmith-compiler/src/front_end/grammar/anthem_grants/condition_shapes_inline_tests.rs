use super::*;
use crate::lexer::lex_line;

fn lex(text: &str) -> Vec<OwnedLexToken> {
    lex_line(text, 0).expect("lex fixture")
}

#[test]
fn classifies_fixed_condition_surfaces() {
    let not_cast = lex("You haven't cast a spell this turn.");
    assert_eq!(
        parse_fixed_static_condition_kind(&not_cast),
        Some(FixedStaticConditionKind::YouDidNotCastSpellThisTurn)
    );
    let not_your_turn = lex("It's not your turn.");
    assert_eq!(
        parse_fixed_static_condition_kind(&not_your_turn),
        Some(FixedStaticConditionKind::NotYourTurn)
    );
    let crime = lex("You've committed a crime this turn.");
    assert_eq!(
        parse_fixed_static_condition_kind(&crime),
        Some(FixedStaticConditionKind::YouCommittedCrimeThisTurn)
    );
    let kicked = lex("This spell was kicked.");
    assert_eq!(
        parse_fixed_static_condition_kind(&kicked),
        Some(FixedStaticConditionKind::SourceSpellWasKicked)
    );
    let outside_battlefield = lex("This isn't on the battlefield.");
    assert_eq!(
        parse_fixed_static_condition_kind(&outside_battlefield),
        Some(FixedStaticConditionKind::SourceIsNotOnBattlefield)
    );
    let equipment_attached = lex("This Equipment is attached to a creature.");
    assert_eq!(
        parse_fixed_static_condition_kind(&equipment_attached),
        Some(FixedStaticConditionKind::SourceEquipmentAttachedToCreature)
    );
}

#[test]
fn parses_devotion_player_colors_and_comparison() {
    let tokens = lex("Your devotion to white and blue is greater than or equal to three.");
    let parsed = parse_devotion_condition_shape(&tokens)
        .expect("valid devotion")
        .expect("devotion shape");
    assert_eq!(parsed.player, DevotionPlayerKind::You);
    assert_eq!(parsed.colors, vec![Color::White, Color::Blue]);
    assert_eq!(parsed.operator, ValueComparisonOperator::GreaterThanOrEqual);
    assert_eq!(parsed.amount, 3);
}

#[test]
fn parses_existential_counter_and_graveyard_shapes() {
    let graveyard = lex("There are four or more card types among cards in your graveyard.");
    let parsed = parse_existential_condition_shape(&graveyard)
        .expect("valid existential")
        .expect("existential shape");
    assert!(matches!(
        parsed.tail,
        ExistentialConditionTail::CardTypesInYourGraveyard { threshold: 4 }
    ));

    let counters = lex("There are three or more charge counters among artifacts you control.");
    let parsed = parse_existential_condition_shape(&counters)
        .expect("valid existential")
        .expect("existential shape");
    assert!(matches!(
        parsed.tail,
        ExistentialConditionTail::CountersAmong {
            counter_type: CounterType::Charge,
            ..
        }
    ));
}

#[test]
fn captures_conjoined_condition_boundaries() {
    let tokens = lex("It is your turn and you control a creature.");
    let splits = parse_conjoined_condition_splits(&tokens);
    assert_eq!(splits.len(), 1);
    assert!(parse_complete_phrase(
        splits[0].left_tokens,
        &["it", "is", "your", "turn"]
    ));
    assert!(parse_complete_phrase(
        splits[0].right_tokens,
        &["you", "control", "a", "creature"]
    ));
}

#[test]
fn parses_typed_quantity_and_source_relation_shapes() {
    let life = lex("You have five or less life.");
    assert_eq!(parse_life_total_or_less_condition(&life), Some(5));

    let x_value = lex("X is greater than three.");
    assert_eq!(parse_x_value_at_least_condition(&x_value), Some(4));

    let blocking = lex("Two or more creatures are blocking it.");
    assert_eq!(
        parse_blocking_source_condition(&blocking),
        Some(BlockingSourceConditionShape {
            comparison: Comparison::GreaterThanOrEqual(2),
        })
    );

    let source_counter = lex("This creature has three or more charge counters on it.");
    assert_eq!(
        parse_source_counter_condition(&source_counter),
        Ok(Some(SourceCounterConditionShape {
            comparison: Comparison::GreaterThanOrEqual(3),
            counter_type: Some(CounterType::Charge),
            pronoun: None,
        }))
    );

    let named_masculine = lex("This creature has a conqueror counter on him.");
    assert_eq!(
        parse_source_counter_condition(&named_masculine),
        Ok(Some(SourceCounterConditionShape {
            comparison: Comparison::GreaterThanOrEqual(1),
            counter_type: Some(CounterType::Named("conqueror".into())),
            pronoun: Some(ironsmith_core::SourceCounterPronounSurface::Him),
        }))
    );

    let graveyard = lex("This card is in your graveyard.");
    assert!(parse_source_in_graveyard_condition(&graveyard));
}
