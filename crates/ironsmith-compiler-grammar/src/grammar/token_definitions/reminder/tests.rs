use crate::lexer::lex_line;
use crate::model::token_definition::{
    CreatureTokenShape, TokenDefinitionSpec, TokenEmbeddedRuleShape,
};
use crate::{color::ColorSet, types::CardType};

use super::*;

fn creature_definition() -> TokenDefinitionSpec {
    TokenDefinitionSpec::Creature(CreatureTokenShape {
        name: "Spirit".to_string(),
        card_types: vec![CardType::Creature],
        subtypes: Vec::new(),
        power_toughness: (0, 0),
        legendary: false,
        colors: ColorSet::WHITE,
        use_source_chosen_color: false,
        use_source_chosen_creature_type: false,
        keywords: Vec::new(),
        rules: Default::default(),
    })
}

#[test]
fn parses_dynamic_pt_and_lifecycle_reminders() {
    let dynamic = lex_line(
        "Its power is equal to that creature's power and its toughness is equal to that creature's toughness.",
        0,
    )
    .unwrap();
    let facts = parse_token_reminder_facts_tokens(&dynamic);
    assert!(matches!(
        facts.dynamic_power_toughness,
        Some((Value::PowerOf(_), Value::ToughnessOf(_)))
    ));

    let lifecycle = lex_line(
        "Sacrifice the token at the beginning of your next end step.",
        0,
    )
    .unwrap();
    let facts = parse_token_reminder_facts_tokens(&lifecycle);
    assert!(facts.sacrifice_at_next_end_step);
    assert_eq!(facts.next_end_step_player, PlayerFilter::You);

    let inline = lex_line(
        "with power equal to that card's power and toughness equal to that card's toughness",
        0,
    )
    .unwrap();
    let facts = parse_token_reminder_facts_tokens(&inline);
    assert!(matches!(
        facts.dynamic_power_toughness,
        Some((Value::PowerOf(ref power), Value::ToughnessOf(ref toughness)))
            if matches!(power.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::model::token_definition::TOKEN_DYNAMIC_THAT_CARD_TAG)
                && matches!(toughness.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::model::token_definition::TOKEN_DYNAMIC_THAT_CARD_TAG)
    ));

    let embedded = lex_line(
        "green Ooze creature token with \"This token's power and toughness are each equal to the number of slime counters on this enchantment.\"",
        0,
    )
    .unwrap();
    let facts = parse_token_reminder_facts_tokens(&embedded);
    assert!(
        matches!(
            facts.dynamic_power_toughness,
            Some((
                Value::CountersOn(_, Some(crate::CounterType::Named(power_counter))),
                Value::CountersOn(_, Some(crate::CounterType::Named(toughness_counter))),
            )) if power_counter.as_str() == "slime" && toughness_counter.as_str() == "slime"
        ),
        "{facts:#?}"
    );

    let standalone = lex_line(
        "This token's power and toughness are each equal to the number of slime counters on this enchantment.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_token_reminder_sentence_kind_tokens(&standalone),
        Some(TokenReminderSentenceKind::PowerToughness)
    );
}

#[test]
fn parses_unquoted_base_pt_from_the_exact_zone_change_group() {
    let tokens = lex_line(
        "with base power and toughness each equal to the total power of those creatures",
        0,
    )
    .expect("dynamic base-power clause should lex");
    let (power, toughness) = parse_token_dynamic_power_toughness_tokens(&tokens)
        .expect("dynamic base-power clause should parse");
    for value in [&power, &toughness] {
        let Value::TotalPower(filter) = value else {
            panic!("expected total-power value, got {value:#?}");
        };
        assert_eq!(filter.card_types, [CardType::Creature]);
        assert_eq!(filter.zone, None);
        assert!(filter.tagged_constraints.iter().any(|constraint| {
            constraint.relation == crate::target::TaggedOpbjectRelation::IsTaggedObject
                && constraint.tag.as_str() == ironsmith_core::ZONE_CHANGE_GROUP_TAG
        }));
    }

    let near_miss = lex_line(
        "with base power and toughness each equal to the total power of those artifacts",
        0,
    )
    .expect("near-miss base-power clause should lex");
    assert_eq!(
        parse_token_dynamic_power_toughness_tokens(&near_miss),
        None,
        "a different demonstrative subject must not claim the creature death group"
    );
}

#[test]
fn merges_quoted_rule_facts_without_relexing_a_definition_name() {
    let tokens = lex_line(
        "It has \"This token's power and toughness are each equal to the number of creatures you control.\"",
        0,
    )
    .unwrap();
    let facts = parse_token_reminder_facts_tokens(&tokens);
    let mut definition = creature_definition();
    super::super::reminder_merge::merge_token_reminder_definition(&mut definition, &facts);
    let TokenDefinitionSpec::Creature(creature) = definition else {
        panic!("expected creature definition");
    };
    assert_eq!(
        creature.rules.token_rules.embedded_rules,
        vec![TokenEmbeddedRuleShape::PowerToughnessEqualCreaturesYouControl]
    );
}

#[test]
fn merging_quoted_equipment_grant_preserves_existing_equip_line() {
    let mut definition =
        crate::grammar::token_definitions::parse_token_definition_shape_text(
            "colorless Equipment artifact token named Rock with \"Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'\" and equip {1}.",
        )
        .expect("complete Equipment token definition");
    let quoted = lex_line(
        "Equipped creature has '{1}, {T}, Sacrifice Rock: This creature deals 2 damage to any target'",
        0,
    )
    .expect("quoted Equipment grant should lex");
    let facts = parse_token_reminder_facts_tokens(&quoted);

    super::super::reminder_merge::merge_token_reminder_definition(&mut definition, &facts);

    let TokenDefinitionSpec::Artifact(artifact) = definition else {
        panic!("expected artifact token definition");
    };
    let rules = artifact
        .equipment_rules
        .expect("Equipment token should retain typed rules");
    assert_eq!(rules.lines.len(), 2, "{rules:#?}");
    assert!(rules.lines.iter().any(|line| matches!(
        line,
        crate::model::token_definition::EquipmentRuleLineShape::GrantedDamage { .. }
    )));
    assert!(rules.lines.iter().any(|line| matches!(
        line,
        crate::model::token_definition::EquipmentRuleLineShape::Equip(
            crate::model::token_definition::TokenEquipShape { amount: 1 }
        )
    )));
}

#[test]
fn classifies_capitalized_quoted_ability_and_typed_lifecycle_reminders() {
    let mana = lex_line("It has \"{T}: Add {G}.\"", 0).unwrap();
    assert_eq!(
        parse_token_reminder_sentence_kind_tokens(&mana),
        Some(TokenReminderSentenceKind::GrantedAbility)
    );
    assert_eq!(
        parse_token_reminder_facts_tokens(&mana)
            .definition
            .creature_rules
            .tap_mana_ability,
        Some(crate::model::token_definition::TokenTapManaAbilityShape {
            mana: vec![crate::mana::ManaSymbol::Green],
            restrictions: Vec::new(),
        })
    );

    let delayed = lex_line(
        "Sacrifice the token at the beginning of the next end step.",
        0,
    )
    .unwrap();
    assert_eq!(
        parse_token_reminder_sentence_kind_tokens(&delayed),
        Some(TokenReminderSentenceKind::DelayedLifecycle)
    );

    let unrelated = lex_line("Sacrifice an artifact: Draw a card.", 0).unwrap();
    assert_eq!(parse_token_reminder_sentence_kind_tokens(&unrelated), None);
}

#[test]
fn quoted_inner_gain_does_not_replace_the_outer_have_verb() {
    let have = lex_line("They have \"When this token dies, you gain 1 life.\"", 0).unwrap();
    let gain = lex_line("They gain haste.", 0).unwrap();

    assert!(!token_ability_sentence_uses_gain_verb(&have));
    assert!(token_ability_sentence_uses_gain_verb(&gain));
}
