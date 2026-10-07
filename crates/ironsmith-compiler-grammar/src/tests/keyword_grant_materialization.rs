use super::*;
use crate::cards::builders::{KeywordAction, StaticAbilityAst};

#[test]
fn keyword_grant_materialization_preserves_typed_anthem_actions_and_filters() {
    for (keyword, expected) in [
        ("melee", KeywordAction::Melee),
        ("myriad", KeywordAction::Myriad),
        ("afflict 3", KeywordAction::Afflict(3)),
    ] {
        let tokens = lex_line(&format!("Other creatures you control have {keyword}."), 0).unwrap();
        let parsed = crate::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
            .unwrap()
            .expect("supported triggered keywords are valid continuous grants");
        let [
            StaticAbilityAst::GrantKeywordAction {
                filter,
                action,
                condition,
            },
        ] = parsed.as_slice()
        else {
            panic!("expected a typed keyword grant: {parsed:#?}")
        };
        assert_eq!(action, &expected);
        assert_eq!(filter.controller, Some(crate::target::PlayerFilter::You));
        assert!(filter.card_types.contains(&CardType::Creature));
        assert!(filter.other);
        assert!(condition.is_none());
    }
}

#[test]
fn keyword_grant_materialization_preserves_typed_equipment_actions() {
    for (keyword, expected) in [
        ("melee", KeywordAction::Melee),
        ("myriad", KeywordAction::Myriad),
        ("afflict 1", KeywordAction::Afflict(1)),
    ] {
        let tokens = lex_line(&format!("Equipped creature has {keyword}."), 0).unwrap();
        let parsed = crate::keyword_static::parse_static_ability_ast_line_lexed(&tokens)
            .unwrap()
            .expect("supported triggered keywords are valid equipment grants");
        let [StaticAbilityAst::EquipmentKeywordActionsGrant { actions }] = parsed.as_slice() else {
            panic!("expected a typed equipment grant: {parsed:#?}")
        };
        assert_eq!(actions, &[expected]);
    }
}

#[test]
fn keyword_grant_materialization_does_not_accept_alternative_costs_as_static_grants() {
    assert!(
        !KeywordAction::Suspend {
            time: ironsmith_core::SuspendTime::Fixed(3),
            cost: crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Blue]),
        }
        .lowers_to_static_ability()
    );
    assert!(
        !KeywordAction::Dash(crate::mana::ManaCost::from_symbols(vec![ManaSymbol::Red]))
            .lowers_to_static_ability()
    );
}
