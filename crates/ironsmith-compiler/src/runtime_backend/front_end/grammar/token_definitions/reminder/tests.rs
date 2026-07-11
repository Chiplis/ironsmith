use crate::runtime_backend::front_end::lexer::lex_line;
use crate::runtime_backend::token_definition::{
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
            if matches!(power.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::runtime_backend::token_definition::TOKEN_DYNAMIC_THAT_CARD_TAG)
                && matches!(toughness.as_ref(), ChooseSpec::Tagged(tag) if tag.as_str() == crate::runtime_backend::token_definition::TOKEN_DYNAMIC_THAT_CARD_TAG)
    ));
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
        Some(
            crate::runtime_backend::token_definition::TokenTapManaAbilityShape {
                mana: vec![crate::mana::ManaSymbol::Green],
                restrictions: Vec::new(),
            }
        )
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
