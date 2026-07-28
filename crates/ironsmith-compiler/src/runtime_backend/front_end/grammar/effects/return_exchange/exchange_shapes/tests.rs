use super::*;
use crate::runtime_backend::front_end::lexer::{lex_line, render_token_slice};

#[test]
fn parses_life_total_partner_shape() {
    let tokens = lex_line("life totals with target opponent", 0).unwrap();
    assert_eq!(
        parse_exchange_clause_shape(&tokens),
        Some(ExchangeClauseShape::LifeTotalsWith(
            PlayerAst::TargetOpponent
        ))
    );
}

#[test]
fn parses_zone_exchange_shape() {
    let tokens = lex_line("your hand and graveyard", 0).unwrap();
    assert!(matches!(
        parse_exchange_clause_shape(&tokens),
        Some(ExchangeClauseShape::Zones {
            player: PlayerAst::You,
            zone1: Zone::Hand,
            zone2: Zone::Graveyard,
        })
    ));
}

#[test]
fn parses_value_operand_shapes() {
    let tokens = lex_line("its power with the toughness of target creature", 0).unwrap();
    let (left, right) = parse_exchange_value_operands(&tokens).expect("operands");
    assert!(matches!(
        left,
        ExchangeValueOperandShape::SourceStat {
            kind: ExchangeValueKindShape::Power,
            ..
        }
    ));
    assert!(matches!(
        right,
        ExchangeValueOperandShape::TargetStat {
            kind: ExchangeValueKindShape::Toughness,
            ..
        }
    ));
}

#[test]
fn parses_named_possessive_source_value_operand() {
    crate::runtime_backend::front_end::shared::util::with_source_reference_context(
        "Evra, Halcyon Witness",
        || {
            let tokens =
                lex_line("your life total with Evra's power.", 0).expect("exchange should lex");
            let (left, right) = parse_exchange_value_operands(&tokens).expect("operands");
            assert!(matches!(
                left,
                ExchangeValueOperandShape::LifeTotal(PlayerAst::You)
            ));
            assert!(matches!(
                right,
                ExchangeValueOperandShape::SourceStat {
                    kind: ExchangeValueKindShape::Power,
                    ..
                }
            ));
        },
    );
}

#[test]
fn rejects_unrelated_named_possessive_source_value_operand() {
    crate::runtime_backend::front_end::shared::util::with_source_reference_context(
        "Evra, Halcyon Witness",
        || {
            let tokens =
                lex_line("your life total with Gerrard's power.", 0).expect("exchange should lex");
            assert!(parse_exchange_value_operands(&tokens).is_none());
        },
    );
}

#[test]
fn parses_heterogeneous_control_exchange_shape() {
    let tokens = lex_line(
        "control of target artifact and target creature that share a card type",
        0,
    )
    .unwrap();
    let Some(ExchangeClauseShape::Control(shape)) = parse_exchange_clause_shape(&tokens) else {
        panic!("expected control shape");
    };
    assert!(shape.heterogeneous.is_some());
    assert_eq!(shape.shared_type, Some(ExchangeSharedTypeShape::CardType));
    assert!(!shape.invalid_shared_type);
}

#[test]
fn parses_heterogeneous_disjunctive_exchange_with_relative_suffix() {
    let tokens = lex_line(
        "control of target artifact or creature and another target permanent that shares one of those types with it",
        0,
    )
    .unwrap();
    let Some(ExchangeClauseShape::Control(shape)) = parse_exchange_clause_shape(&tokens) else {
        panic!("expected control shape");
    };
    let (left, right) = shape.heterogeneous.expect("heterogeneous targets");
    assert_eq!(render_token_slice(left), "target artifact or creature");
    assert_eq!(render_token_slice(right), "another target permanent");
    assert_eq!(shape.shared_type, Some(ExchangeSharedTypeShape::CardType));
    assert!(!shape.invalid_shared_type);
}
