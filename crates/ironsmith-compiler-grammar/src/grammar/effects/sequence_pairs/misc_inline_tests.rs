use super::*;
use crate::lexer::lex_line;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn parses_directional_and_phase_pair_shapes() {
    let directional = parse_directional_adjacent_player_control_shape(
            &lex("Starting with you and proceeding in the chosen direction, each player chooses a creature controlled by the next player in that direction"),
            &lex("Each player gains control of the creature they chose"),
        )
        .unwrap();
    assert!(!directional.choice_object.is_empty());
    assert!(parse_choose_then_skip_phase_shape(
        &lex("That player chooses draw step, main phase, or combat phase"),
        &lex("That player skips each instance of the chosen step or phase this turn"),
    ));
}

#[test]
fn parses_graveyard_cast_and_return_shapes() {
    let shape = parse_graveyard_cast_replacement_shape(
            &lex("You may cast target artifact, instant, or sorcery card with mana value three or less from your graveyard without paying its mana cost"),
            &lex("If that spell would be put into your graveyard, exile it instead"),
        )
        .unwrap();
    assert_eq!(shape.mana_value_limit, Some(3));
    assert!(shape.includes_artifact);
    assert!(shape.artifact_first);
    assert!(shape.without_paying_mana_cost);
    assert!(!shape.until_end_of_turn);
    assert_eq!(
        shape.mana_spend_mode,
        ironsmith_core::value_model::ManaSpendMode::Normal
    );
    let any_type = parse_graveyard_cast_replacement_shape(
            &lex(
                "You may cast target instant or sorcery card from a graveyard, and mana of any type can be spent to cast that spell",
            ),
            &lex("If that spell would be put into a graveyard, exile it instead"),
        )
        .unwrap();
    assert_eq!(
        any_type.mana_spend_mode,
        ironsmith_core::value_model::ManaSpendMode::AnyType
    );
    assert!(
            parse_graveyard_cast_replacement_shape(
                &lex(
                    "You may cast target instant or sorcery card from a graveyard, and mana of any color can be spent to cast that spell",
                ),
                &lex("If that spell would be put into a graveyard, exile it instead"),
            )
            .is_none()
        );
    let duration = parse_graveyard_cast_replacement_shape(
            &lex(
                "Until end of turn, you may cast target instant or sorcery card from your graveyard without paying its mana cost",
            ),
            &lex("If that spell would be put into your graveyard, exile it instead"),
        )
        .unwrap();
    assert!(duration.until_end_of_turn);
    assert!(
        parse_graveyard_cast_replacement_shape(
            &lex(
                "You may cast target instant card from your graveyard without paying its mana cost"
            ),
            &lex("If that spell would be put into a graveyard, exile it instead"),
        )
        .is_some()
    );
    assert!(
            parse_graveyard_cast_replacement_shape(
                &lex(
                    "You may cast target instant or sorcery card from a graveyard without paying its mana cost"
                ),
                &lex("If that spell would be put into a graveyard, exile it instead"),
            )
            .is_some()
        );
    assert_eq!(
        parse_return_tagged_battlefield_shape(&lex("Return those cards to the battlefield tapped")),
        Some(ReturnTaggedBattlefieldShape { tapped: true })
    );
    assert!(is_filtered_future_exile_return_next_end_step_shape(
        &lex(
            "If a permanent you control would be put into a graveyard from the battlefield this turn, exile it instead"
        ),
        &lex(
            "Return it to the battlefield under its owner's control at the beginning of the next end step"
        ),
    ));
    assert!(is_filtered_future_exile_return_next_end_step_shape(
        &lex(
            "If a permanent you control would be put into a graveyard from the battlefield this turn, exile it instead"
        ),
        &lex(
            "At the beginning of the next end step, return it to the battlefield under its owner's control"
        ),
    ));
    assert!(is_resolving_card_exile_then_return_next_end_step_shape(
        &lex("Exile that card instead of putting it into your graveyard as it resolves"),
        &lex("If you do, return it to your hand at the beginning of the next end step"),
    ));
}
