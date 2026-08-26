use super::*;

fn render(text: &str, card_types: Vec<CardType>) -> String {
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Typed Cost Surface Probe")
            .card_types(card_types)
            .parse_text(text)
            .expect("typed cost surface should compile");
    crate::compiled_text::compiled_text_lines(&definition).join("\n")
}

#[test]
fn activated_ability_cost_increase_uses_its_typed_sacrifice_cost() {
    let text = "Activated abilities of nontoken Rebels cost an additional \"Sacrifice a land\" to activate.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Typed Cost Surface Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text(text)
            .expect("typed cost increase should compile");
    let AbilityKind::Static(ability) = &definition.abilities[0].kind else {
        panic!("cost increase should be static")
    };
    let ironsmith_core::StaticAbilityPayload::ActivatedAbilityCostIncrease { increase, .. } =
        &ability.compiled_model().expect("compiled model").payload
    else {
        panic!("typed activated-ability cost increase payload was lost")
    };
    assert_eq!(describe_total_cost(increase), "Sacrifice a land");
    assert_eq!(
        restore_modeled_value_surface(ability, ability.display()),
        text.trim_end_matches('.')
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}

#[test]
fn graveyard_cast_grant_uses_its_typed_sacrifice_cost() {
    let text = "Once during each of your turns, you may cast an instant or sorcery spell from your graveyard by sacrificing a creature in addition to paying its other costs. If a spell cast this way would be put into your graveyard, exile it instead.";
    assert_eq!(render(text, vec![CardType::Enchantment]), text);
}

#[test]
fn graveyard_cast_grant_uses_its_typed_choose_then_exile_cost() {
    let text = "You may cast this card from your graveyard by exiling four instant and/or sorcery cards from your graveyard in addition to paying its other costs.";
    let rendered = render(text, vec![CardType::Creature]);
    assert_eq!(rendered, text);
    assert!(!rendered.contains("Effect"), "{rendered}");
    assert!(rendered.contains("by exiling four "), "{rendered}");
    assert!(
        rendered.contains("cards from your graveyard in addition to paying its other costs"),
        "{rendered}"
    );
}

#[test]
fn granted_nonmana_ward_uses_the_typed_cost_inside_quotes() {
    let text = "Permanents you control have \"Ward—Sacrifice a permanent.\"";
    assert_eq!(render(text, vec![CardType::Creature]), text);
}

#[test]
fn blink_with_entry_counter_preserves_then_and_the_authored_reference() {
    for text in [
        "Exile target artifact or creature, then return it to the battlefield under its owner's control with a +1/+1 counter on it.",
        "Exile target artifact or creature, then return that card to the battlefield under its owner's control with a +1/+1 counter on it.",
    ] {
        assert_eq!(render(text, vec![CardType::Instant]), text);
    }
}

#[test]
fn prevention_followup_and_delayed_pact_payment_share_the_public_statement_route() {
    let text = "The next time a source of your choice would deal damage to you this turn, prevent that damage. You gain life equal to the damage prevented this way.\nAt the beginning of your next upkeep, pay {1}{W}{W}. If you don't, you lose the game.";
    assert_eq!(render(text, vec![CardType::Instant]), text);
}

#[test]
fn fixed_mana_output_keeps_its_typed_on_spend_copy_program() {
    let text = "{T}: Add {R}. When that mana is spent to cast a red instant or sorcery spell, copy that spell and you may choose new targets for the copy.";
    let definition =
        crate::CardDefinitionBuilder::new(crate::ids::CardId::new(), "Mana Spend Copy Probe")
            .card_types(vec![CardType::Artifact])
            .parse_text(text)
            .expect("typed mana-spend copy surface should compile");
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition).join("\n"),
        text
    );
}
