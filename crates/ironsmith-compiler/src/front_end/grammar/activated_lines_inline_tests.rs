use super::*;
use crate::lexer::{TokenKind, lex_line};

#[test]
fn splits_activated_line_and_classifies_primary_mana_shape() {
    let tokens = lex_line("{T}: Target player adds one mana of any color.", 0).unwrap();
    let split = parse_activated_line_split_tokens(&tokens).unwrap();
    assert!(
        split
            .before_colon
            .iter()
            .any(|token| token.kind == TokenKind::ManaGroup)
    );

    let spec = parse_primary_mana_clause_tokens(split.after_colon).unwrap();
    assert_eq!(spec.kind, PrimaryManaClauseKind::Standard);
    assert!(spec.subject_tokens.is_some());
    assert!(spec.requires_general_effect);
}

#[test]
fn parses_devotion_owner_and_color_into_value() {
    let tokens = lex_line("an amount equal to your devotion to green", 0).unwrap();
    assert!(matches!(
        parse_activated_devotion_value_tokens(&tokens).unwrap(),
        Some(Value::Devotion {
            player: PlayerFilter::You,
            color: Color::Green,
        })
    ));

    let tokens = lex_line("that player's devotion to that color", 0).unwrap();
    assert!(matches!(
        parse_activated_devotion_value_tokens(&tokens).unwrap(),
        Some(Value::DevotionToChosenColor(PlayerFilter::Target(_)))
    ));
}

#[test]
fn classifies_enters_tapped_variants() {
    for (raw, expected) in [
        ("This enters tapped.", EntersTappedLineShape::EntersTapped),
        (
            "This enters tapped and attacking.",
            EntersTappedLineShape::AttackingVariant,
        ),
        (
            "This enters tapped with a counter.",
            EntersTappedLineShape::UnsupportedTrailing,
        ),
    ] {
        let tokens = lex_line(raw, 0).unwrap();
        assert_eq!(parse_enters_tapped_line_shape(&tokens), expected);
    }
}

#[test]
fn exposes_cost_reduction_head_and_typed_tail() {
    let tokens = lex_line(
        "This ability costs {2} less to activate if it targets one creature.",
        0,
    )
    .unwrap();
    let CostReductionLineHead::ThisAbility { amount_tokens } =
        parse_cost_reduction_line_head_tokens(&tokens).unwrap()
    else {
        panic!("expected this-ability head");
    };
    let tail = &amount_tokens[1..];
    assert!(matches!(
        parse_this_ability_reduction_remainder_tokens(tail),
        ThisAbilityReductionRemainder::Targets { .. }
    ));
}

#[test]
fn distinguishes_explicit_minimum_one_mana_cost_reduction() {
    let unbounded = lex_line("less to activate.", 0).unwrap();
    assert_eq!(
        parse_activated_abilities_reduction_remainder_tokens(&unbounded),
        Some(ActivatedAbilitiesReductionRemainder::Unbounded)
    );

    let minimum_one = lex_line(
        "less to activate. This effect can't reduce the mana in that cost to less than one mana.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_activated_abilities_reduction_remainder_tokens(&minimum_one),
        Some(ActivatedAbilitiesReductionRemainder::MinimumOneMana)
    );

    let ability_activation_cost = lex_line(
            "less to activate. This effect can't reduce the mana in that ability's activation cost to less than one mana.",
            0,
        )
        .unwrap();
    assert_eq!(
        parse_activated_abilities_reduction_remainder_tokens(&ability_activation_cost),
        Some(ActivatedAbilitiesReductionRemainder::MinimumOneManaAbilityActivationCost)
    );
}

#[test]
fn parses_next_spell_reduction_spans() {
    let tokens = lex_line(
        "The next creature spell you cast this turn costs {2} less to cast.",
        0,
    )
    .unwrap();
    let spec = parse_next_spell_cost_reduction_tokens(&tokens).unwrap();
    assert_eq!(
        spec.spell_filter.card_types,
        vec![crate::types::CardType::Creature]
    );
    assert_eq!(spec.reduction.to_oracle(), "{2}");
}

#[test]
fn normalizes_once_per_turn_restriction_without_surface_probes() {
    let tokens = lex_line(
        "Activate only once each turn and only if you control a creature and only once each turn.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_once_per_turn_restriction_normalization_tokens(&tokens),
        OncePerTurnRestrictionNormalization::Residual("only if you control a creature".to_string())
    );
}
