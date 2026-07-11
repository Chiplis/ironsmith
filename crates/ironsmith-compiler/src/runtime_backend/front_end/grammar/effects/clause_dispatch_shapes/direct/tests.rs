use super::*;
use crate::runtime_backend::front_end::lexer::lex_line;

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
        Some(AssignsNoCombatDamageShape::Supported(
            AssignDamageSourceShape::Tagged
        ))
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
        })
    );
}
