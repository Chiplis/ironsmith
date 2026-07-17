use super::*;
use crate::runtime_backend::front_end::lexer::{TokenWordView, lex_line};

#[test]
fn choose_target_shape_keeps_counted_targets() {
    let tokens = lex_line("Choose up to one target creature card in a graveyard.", 0).unwrap();
    let shape = parse_choose_target_shape(&tokens).expect("counted target choice");
    assert_eq!(
        TokenWordView::new(shape.target_tokens).to_word_refs(),
        vec![
            "up",
            "to",
            "one",
            "target",
            "creature",
            "card",
            "in",
            "a",
            "graveyard"
        ]
    );

    let tokens = lex_line("Choose any number of target creatures.", 0).unwrap();
    assert!(parse_choose_target_shape(&tokens).is_some());
}

#[test]
fn choose_target_shape_preserves_authored_chooser_roles() {
    let controller_tokens =
        lex_line("You choose target creature an opponent controls.", 0).unwrap();
    let controller = parse_choose_target_shape(&controller_tokens).expect("controller choice");
    assert_eq!(
        controller.chooser,
        ChooseTargetChooserShape::AbilityController
    );

    let opponent_tokens = lex_line("That opponent chooses target creature.", 0).unwrap();
    let opponent = parse_choose_target_shape(&opponent_tokens).expect("opponent choice");
    assert_eq!(opponent.chooser, ChooseTargetChooserShape::ThatOpponent);
}

#[test]
fn embedded_choose_target_shape_preserves_chooser_and_relative_controller_exclusion() {
    let tokens = lex_line(
        "Its controller chooses target permanent another player controls that shares a card type with it.",
        0,
    )
    .unwrap();
    let shape = parse_embedded_choose_target_shape(&tokens).expect("embedded target choice");

    assert_eq!(shape.chooser, ChooseTargetChooserShape::ItsController);
    assert!(shape.excludes_chooser_controller);
    assert_eq!(
        TokenWordView::new(shape.target_tokens).to_word_refs(),
        vec![
            "target",
            "permanent",
            "another",
            "player",
            "controls",
            "that",
            "shares",
            "a",
            "card",
            "type",
            "with",
            "it"
        ]
    );
}

#[test]
fn parses_direct_and_assign_damage_shapes() {
    let direct = lex_line("The Ring tempts you.", 0).unwrap();
    assert_eq!(
        parse_direct_clause_shape(&direct),
        Some(DirectClauseShape::RingTemptsYou)
    );
    let unpreventable = lex_line("The damage can't be prevented.", 0).unwrap();
    assert_eq!(
        parse_direct_clause_shape(&unpreventable),
        Some(DirectClauseShape::DamageCantBePrevented)
    );
    let assign = lex_line("It assigns no combat damage this turn.", 0).unwrap();
    assert!(matches!(
        parse_assigns_no_combat_damage_shape(&assign),
        Some(AssignsNoCombatDamageShape::Supported {
            source: AssignDamageSourceShape::Tagged,
            duration: Until::EndOfTurn,
        })
    ));
    let source = lex_line("This creature assigns no combat damage this combat.", 0).unwrap();
    assert!(matches!(
        parse_assigns_no_combat_damage_shape(&source),
        Some(AssignsNoCombatDamageShape::Supported {
            source: AssignDamageSourceShape::Source,
            duration: Until::EndOfCombat,
        })
    ));
    let target = lex_line(
        "The attacking creature assigns no combat damage this turn.",
        0,
    )
    .unwrap();
    assert!(matches!(
        parse_assigns_no_combat_damage_shape(&target),
        Some(AssignsNoCombatDamageShape::Supported {
            source: AssignDamageSourceShape::Target(_),
            duration: Until::EndOfTurn,
        })
    ));

    let next_turn = lex_line(
        "That player can't cast creature spells during that player's next turn.",
        0,
    )
    .unwrap();
    assert!(parse_next_turn_cant_shape_tokens(&next_turn).is_some());
}

#[test]
fn parses_protection_choice_shapes() {
    let color = lex_line(
        "Protection from the color of your choice until end of turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_protection_choice_shape(&color),
        Some(ProtectionChoiceShape {
            includes_colorless: false,
            includes_artifacts: false,
            chooser: ProtectionChoiceChooserShape::You,
        })
    );

    let colorless = lex_line(
        "Protection from colorless or from the color of your choice until end of turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_protection_choice_shape(&colorless),
        Some(ProtectionChoiceShape {
            includes_colorless: true,
            includes_artifacts: false,
            chooser: ProtectionChoiceChooserShape::You,
        })
    );

    let artifacts = lex_line(
        "Protection from artifacts or from the color of your choice until end of turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_protection_choice_shape(&artifacts),
        Some(ProtectionChoiceShape {
            includes_colorless: false,
            includes_artifacts: true,
            chooser: ProtectionChoiceChooserShape::You,
        })
    );

    let target_controller = lex_line(
        "Protection from the color of its controller's choice until end of turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_protection_choice_shape(&target_controller),
        Some(ProtectionChoiceShape {
            includes_colorless: false,
            includes_artifacts: false,
            chooser: ProtectionChoiceChooserShape::TargetController,
        })
    );
}
