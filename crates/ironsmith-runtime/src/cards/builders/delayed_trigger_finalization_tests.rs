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
    let def = CardDefinitionBuilder::new(CardId::new(), "Glass Asp")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "Whenever this creature deals combat damage to a player, that player loses 2 life at the beginning of their next draw step unless they pay {2} before that step.",
            )
            .expect("delayed draw-step payment should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ScheduleDelayedTriggerEffect")
            && abilities_debug.contains("BeginningOfDrawStep")
            && abilities_debug.contains("prepayment: Some")
            && !abilities_debug.contains("UnlessPaysEffect"),
        "expected a prepayable delayed draw-step schedule, got {abilities_debug}"
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&def),
        vec![
            "Whenever this creature deals combat damage to a player, that player loses 2 life at the beginning of their next draw step unless they pay {2} before that draw step."
                .to_string(),
        ],
        "delayed draw-step structure: {abilities_debug}",
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_sabertooth_cobra_uses_a_prepayable_upkeep_registration() {
    let oracle = "Whenever this creature deals damage to a player, that player gets a poison counter. The player gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step. (A player with ten or more poison counters loses the game.)";
    let def = CardDefinitionBuilder::new(CardId::new(), "Sabertooth Cobra")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Sabertooth Cobra should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ScheduleDelayedTriggerEffect")
            && abilities_debug.contains("BeginningOfUpkeep")
            && abilities_debug.contains("prepayment: Some")
            && abilities_debug.contains("PoisonCountersEffect")
            && !abilities_debug.contains("UnlessPaysEffect"),
        "expected a prepayable delayed upkeep registration, got {abilities_debug}"
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&def),
        vec![
            "Whenever this creature deals damage to a player, that player gets a poison counter. The player gets another poison counter at the beginning of their next upkeep unless they pay {2} before that step."
        ]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_nafs_asp_uses_a_prepayable_draw_step_registration() {
    let oracle = "Whenever this creature deals damage to a player, that player loses 1 life at the beginning of their next draw step unless they pay {1} before that draw step.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Nafs Asp")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("Nafs Asp should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ScheduleDelayedTriggerEffect")
            && abilities_debug.contains("BeginningOfDrawStep")
            && abilities_debug.contains("prepayment: Some")
            && abilities_debug.contains("LoseLifeEffect")
            && !abilities_debug.contains("UnlessPaysEffect"),
        "expected a prepayable delayed draw-step registration, got {abilities_debug}"
    );
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&def),
        vec![oracle]
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn delayed_unless_payment_without_before_wording_stays_on_the_delayed_resolution() {
    let oracle = "Whenever this creature deals damage to a player, that player loses 1 life at the beginning of their next draw step unless they pay {1}.";
    let def = CardDefinitionBuilder::new(CardId::new(), "Delayed Payment Probe")
        .card_types(vec![CardType::Creature])
        .parse_text(oracle)
        .expect("ordinary delayed unless-payment should parse");

    let abilities_debug = format!("{:?}", def.abilities);
    assert!(
        abilities_debug.contains("ScheduleDelayedTriggerEffect")
            && abilities_debug.contains("BeginningOfDrawStep")
            && abilities_debug.contains("prepayment: None")
            && abilities_debug.contains("UnlessPaysEffect"),
        "payment without explicit before-step wording must stay in the delayed effect, got {abilities_debug}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_delayed_next_upkeep_unless_payment_keeps_payment_player_choice() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Quenchable Fire")
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
            && spell_debug.contains("prepayment: Some")
            && !spell_debug.contains("UnlessPaysEffect"),
        "expected prepayable delayed upkeep schedule in spell debug, got {spell_debug}"
    );
    assert_eq!(
        crate::compiled_text::canonical_compiled_lines(&def).join(" "),
        "Quenchable Fire deals 3 damage to target player or planeswalker. It deals an additional 3 damage to that player or planeswalker at the beginning of your next upkeep step unless that player or that planeswalker's controller pays {U} before that step."
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_delayed_dynamic_token_creation_keeps_resolution_time_characteristics() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Delayed Sand Warrior Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "When this creature enters, create X 1/1 Sand Warrior creature tokens that are red, green, and white at the beginning of your next upkeep, where X is the number of lands you control at that time.\nWhen this creature leaves the battlefield, exile all Sand Warriors.",
        )
        .expect("delayed dynamic token creation should parse");

    let rendered = crate::compiled_text::canonical_compiled_lines(&def).join(" ");
    let rendered_lower = rendered.to_ascii_lowercase();
    assert!(
        rendered_lower.contains("sand warrior")
            && rendered_lower.contains("red")
            && rendered_lower.contains("green")
            && rendered_lower.contains("white")
            && rendered_lower.contains("at the beginning of your next upkeep")
            && rendered_lower.contains("at that time")
            && rendered_lower.contains("exile all sand warriors"),
        "expected delayed Sand Warrior identity, colors, timing, and cleanup, got {rendered}"
    );
    let create_position = rendered_lower
        .find("create x")
        .expect("rendered create instruction");
    let timing_position = rendered_lower
        .find("at the beginning of your next upkeep")
        .expect("rendered delayed timing");
    assert!(
        create_position < timing_position,
        "expected action-first delayed surface, got {rendered}"
    );
}

#[cfg(ironsmith_runtime_parser_tests)]
#[test]
fn parse_last_chosen_player_combat_static_persists_choice_and_filter() {
    let def = CardDefinitionBuilder::new(CardId::new(), "Combat Choice Variant")
        .card_types(vec![CardType::Creature])
        .parse_text(
            "At the beginning of combat on your turn, choose an opponent.\nCreatures attacking the last chosen player have menace.",
        )
        .expect("last-chosen-player static should parse");

    let debug = format!("{:#?}", def.abilities);
    assert!(
        debug.contains("remember_as_chosen_player: true")
            && debug.contains("attacking_player_or_planeswalker_controlled_by: Some")
            && debug.contains("ChosenPlayer"),
        "expected persistent player choice and attacking-player filter, got {debug}"
    );
    let rendered = crate::compiled_text::canonical_compiled_lines(&def).join(" ");
    assert!(
        rendered.contains("Creatures attacking the last chosen player have menace"),
        "expected chosen-player combat static surface, got {rendered}"
    );
}
