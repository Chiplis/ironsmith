use super::*;

#[test]
fn finalize_definition_rehomes_nonpermanent_delayed_battlefield_trigger() {
    let original_builder = CardDefinitionBuilder::new(CardId::new(), "Delayed Safety Net Probe")
        .card_types(vec![CardType::Instant]);
    let mut definition = original_builder.clone().build();
    definition.spell_effect = Some(ResolutionProgram::from_effects(vec![Effect::draw(1)]));
    definition.abilities.push(Ability::triggered(
        Trigger::beginning_of_upkeep(PlayerFilter::You),
        vec![Effect::unless_pays(
            vec![Effect::lose_the_game()],
            PlayerFilter::You,
            vec![ManaSymbol::Generic(2), ManaSymbol::Green, ManaSymbol::Green],
        )],
    ));

    let finalized =
        finalize_definition(definition, &original_builder, "").expect("definition should finalize");

    assert!(
        finalized.abilities.is_empty(),
        "battlefield-only delayed trigger should be removed from instant abilities"
    );
    let spell_debug = format!("{:?}", finalized.spell_effect);
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect")
            && spell_debug.contains("start_next_turn: true"),
        "delayed trigger should be rewritten into spell effects, got {spell_debug}"
    );
}

#[test]
fn finalize_definition_keeps_stack_triggered_spell_abilities() {
    let original_builder = CardDefinitionBuilder::new(CardId::new(), "Stack Trigger Probe")
        .card_types(vec![CardType::Instant]);
    let mut definition = original_builder.clone().build();
    definition.spell_effect = Some(ResolutionProgram::from_effects(vec![Effect::draw(1)]));
    definition.abilities.push(
        Ability::triggered(Trigger::you_cast_this_spell(), vec![Effect::draw(1)])
            .in_zones(vec![Zone::Stack]),
    );

    let finalized =
        finalize_definition(definition, &original_builder, "").expect("definition should finalize");

    assert_eq!(
        finalized.abilities.len(),
        1,
        "non-battlefield triggered abilities should remain untouched"
    );
    let spell_debug = format!("{:?}", finalized.spell_effect);
    assert!(
        !spell_debug.contains("ScheduleDelayedTriggerEffect"),
        "stack trigger should not be rewritten into a delayed spell effect"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_delayed_next_draw_step_unless_payment_builds_draw_step_schedule() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Glass Asp Variant")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Whenever this creature deals damage to a player, that player loses 2 life at the beginning of their next draw step unless they pay {2} before that step.",
            )
            .expect("delayed draw-step payment should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ScheduleDelayedTriggerEffect")
            && abilities_debug.contains("BeginningOfDrawStep")
            && abilities_debug.contains("UnlessPaysEffect"),
        "expected delayed draw-step schedule in ability debug, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_delayed_next_upkeep_unless_payment_keeps_payment_player_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quenchable Fire Variant")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Quenchable Fire deals 3 damage to target player or planeswalker. It deals an additional 3 damage to that player or planeswalker at the beginning of your next upkeep step unless that player or that planeswalker's controller pays {U} before that step.",
            )
            .expect("delayed upkeep payment should parse");

    let spell_debug = format!("{:?}", def.spell_effect);
    assert!(
        spell_debug.contains("ScheduleDelayedTriggerEffect")
            && spell_debug.contains("BeginningOfUpkeep")
            && spell_debug.contains("TargetPlayerOrControllerOfTarget")
            && spell_debug.contains("UnlessPaysEffect"),
        "expected delayed upkeep schedule in spell debug, got {spell_debug}"
    );
}
