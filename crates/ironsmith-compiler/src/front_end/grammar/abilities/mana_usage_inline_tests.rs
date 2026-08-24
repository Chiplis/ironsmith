use super::super::super::super::lexer::lex_line;
use super::*;

fn lex(raw: &str) -> Vec<OwnedLexToken> {
    lex_line(raw, 0).unwrap()
}

#[test]
fn mana_usage_shapes_return_existing_typed_restrictions() {
    assert_eq!(
        parse_mana_usage_restriction_sentence_lexed(&lex(
            "Spend this mana only to cast a creature spell."
        )),
        Some(ManaUsageRestriction::CastSpell {
            card_types: vec![CardType::Creature],
            subtype_requirement: None,
            restrict_to_matching_spell: true,
            grant_uncounterable: false,
            enters_with_counters: vec![],
            granted_abilities: vec![],
        })
    );
    assert_eq!(
        parse_mana_usage_restriction_sentence_lexed(&lex(
            "Spend this mana only to activate an ability."
        )),
        Some(ManaUsageRestriction::ActivateAbility)
    );
}

#[test]
fn mana_spend_bonus_shapes_preserve_haste_and_counter_facts() {
    let haste = parse_mana_spend_bonus_sentence_lexed(&lex(
        "If this mana is spent to cast a creature spell, it gains haste.",
    ))
    .unwrap();
    assert!(matches!(
        haste,
        ManaUsageRestriction::CastSpell { granted_abilities, .. }
            if granted_abilities == [StaticAbilityId::Haste]
    ));

    let counter = parse_mana_spend_bonus_sentence_lexed(&lex(
        "If this mana is spent to cast a creature spell, that creature enters with an additional +1/+1 counter on it.",
    ));
    assert!(counter.is_some());
}

#[test]
fn mana_spend_bonus_preserves_riot_as_a_runtime_keyword_grant() {
    let riot = parse_mana_spend_bonus_sentence_lexed(&lex(
        "If that mana is spent on a creature spell, it gains riot.",
    ));
    assert!(matches!(
        riot,
        Some(ManaUsageRestriction::CastSpellWithManaBonus {
            condition: ManaSpendBonusCondition::IfThatManaIsSpentOn,
            granted_keywords,
            ..
        }) if granted_keywords == [ManaSpendGrantedKeyword::Riot]
    ));
}

#[test]
fn u078_parses_arbitrary_payment_purpose_and_cost_predicates() {
    let cumulative = parse_mana_usage_restriction_sentence_lexed(&lex(
        "Spend this mana only to pay cumulative upkeep costs.",
    ));
    assert!(matches!(
        cumulative,
        Some(ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::Purpose(
                ManaPaymentPurpose::CumulativeUpkeep
            )),
            ref on_spend,
        }) if on_spend.is_empty()
    ));

    let contains_x = parse_mana_usage_restriction_sentence_lexed(&lex(
        "Spend this mana only on costs that contain {X}.",
    ));
    assert!(matches!(
        contains_x,
        Some(ManaUsageRestriction::PaymentTransaction {
            restriction: Some(ManaPaymentPredicate::CostContainsX),
            ref on_spend,
        }) if on_spend.is_empty()
    ));
}

#[test]
fn u078_parses_generic_scry_and_copy_on_spend_payloads() {
    for text in [
        "When that mana is spent to cast a creature spell that shares a creature type with your commander, scry 1.",
        "When you spend this mana to cast your commander, scry X, where X is the number of times it's been cast from the command zone this game.",
        "When that mana is spent to cast a red instant or sorcery spell, copy that spell and you may choose new targets for the copy.",
    ] {
        let parsed = parse_mana_spend_bonus_sentence_lexed(&lex(text));
        assert!(
            matches!(
                parsed,
                Some(ManaUsageRestriction::PaymentTransaction {
                    restriction: None,
                    ref on_spend,
                }) if on_spend.len() == 1 && !on_spend[0].effects.is_empty()
            ),
            "failed to parse {text}: {parsed:?}"
        );
    }
}
