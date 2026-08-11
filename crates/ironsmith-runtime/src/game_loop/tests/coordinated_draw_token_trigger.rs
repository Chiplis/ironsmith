use super::*;

const RULES_TEXT: &str = "You draw two cards and you lose 2 life. Create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\"\nWizards you control get +1/+0 and gain lifelink until end of turn.";

fn definition() -> crate::cards::CardDefinition {
    CardDefinitionBuilder::new(CardId::from_raw(920_001), "Coordinated Wizard Spell")
        .card_types(vec![CardType::Sorcery])
        .parse_text(RULES_TEXT)
        .expect("coordinated draw/token fixture should parse")
}

fn queue_spell_cast(
    game: &mut GameState,
    caster: PlayerId,
    card_types: Vec<CardType>,
) -> TriggerQueue {
    let mut builder =
        CardBuilder::new(CardId::new(), "Trigger Probe Spell").card_types(card_types.clone());
    if card_types.contains(&CardType::Creature) {
        builder = builder.power_toughness(PowerToughness::fixed(1, 1));
    }
    let spell = game.create_object_from_card(&builder.build(), caster, Zone::Stack);
    let event = TriggerEvent::new_with_provenance(
        SpellCastEvent::new(spell, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    let mut queue = TriggerQueue::new();
    queue_triggers_from_event(game, &mut queue, event, false);
    queue
}

#[test]
fn coordinated_draw_token_program_keeps_all_typed_parts() {
    let debug = format!("{:#?}", definition());
    for expected in [
        "SequenceEffect",
        "DrawCardsEffect",
        "LoseLifeEffect",
        "CreateTokenEffect",
        "SpellCastTrigger",
        "ForPlayersEffect",
        "ApplyContinuousEffect",
        "Lifelink",
    ] {
        assert!(
            debug.contains(expected),
            "compiled definition should contain {expected}: {debug}"
        );
    }
    assert!(debug.contains("excluded_card_types") && debug.contains("Creature"));
    assert!(debug.contains("IteratedPlayer"));
}

#[test]
fn coordinated_draw_token_program_has_exact_compiled_surface() {
    assert_eq!(
        crate::compiled_text::compiled_text_lines(&definition()),
        vec![
            "You draw two cards and you lose 2 life. Create a 0/1 black Wizard creature token with \"Whenever you cast a noncreature spell, this token deals 1 damage to each opponent.\"".to_string(),
            "Wizards you control get +1/+0 and gain lifelink until end of turn.".to_string(),
        ]
    );
}

#[test]
fn coordinated_draw_token_program_executes_draw_loss_token_buff_and_trigger() {
    let mut game = setup_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    for index in 0..2 {
        let card = CardBuilder::new(CardId::new(), format!("Draw Card {index}"))
            .card_types(vec![CardType::Land])
            .build();
        game.create_object_from_card(&card, alice, Zone::Library);
    }

    let definition = definition();
    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let mut ctx = crate::effects::ExecutionContext::new_default(source, alice);
    execute_resolution_program(
        &mut game,
        &mut ctx,
        alice,
        source,
        definition
            .spell_effect
            .as_ref()
            .expect("fixture should have a spell program"),
        None,
        &[],
    )
    .expect("coordinated draw/token program should resolve");

    assert_eq!(game.player(alice).expect("alice exists").hand.len(), 2);
    assert_eq!(game.player(alice).expect("alice exists").life, 18);

    let wizard = game
        .battlefield
        .iter()
        .copied()
        .find(|id| {
            game.object(*id).is_some_and(|object| {
                object.kind == ObjectKind::Token
                    && object.subtypes.contains(&Subtype::Wizard)
                    && game.controller_of(object) == alice
            })
        })
        .expect("resolution should create the Wizard token");
    assert_eq!(game.calculated_power(wizard), Some(1));
    assert_eq!(game.calculated_toughness(wizard), Some(1));
    assert!(
        game.current_has_static_ability_id(
            wizard,
            crate::static_abilities::StaticAbilityId::Lifelink,
        )
    );

    let mut creature_queue = queue_spell_cast(&mut game, alice, vec![CardType::Creature]);
    assert!(
        creature_queue.entries.is_empty(),
        "the embedded trigger must reject creature spells"
    );

    let mut noncreature_queue = queue_spell_cast(&mut game, alice, vec![CardType::Instant]);
    assert_eq!(noncreature_queue.entries.len(), 1);
    let mut dm = SelectFirstDecisionMaker;
    put_triggers_on_stack_with_dm(&mut game, &mut noncreature_queue, &mut dm)
        .expect("embedded trigger should go on the stack");
    resolve_stack_entry_with(&mut game, &mut dm).expect("embedded trigger should resolve");
    assert_eq!(game.player(alice).expect("alice exists").life, 18);
    assert_eq!(game.player(bob).expect("bob exists").life, 19);
}
