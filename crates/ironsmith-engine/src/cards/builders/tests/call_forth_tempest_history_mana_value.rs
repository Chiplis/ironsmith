#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const ORACLE: &str = "Cascade, cascade\nCall Forth the Tempest deals damage to each creature your opponents control equal to the total mana value of other spells you've cast this turn.";

fn record_cast(
    game: &mut crate::GameState,
    caster: PlayerId,
    name: &str,
    mana_value: u8,
) -> ObjectId {
    let definition = CardDefinitionBuilder::new(CardId::new(), name)
        .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
            crate::mana::ManaSymbol::Generic(mana_value),
        ]]))
        .card_types(vec![CardType::Instant])
        .build();
    let spell = game.create_object_from_definition(&definition, caster, Zone::Stack);
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(spell, caster, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&event);
    spell
}

fn durable_creature(name: &str) -> CardDefinition {
    CardDefinitionBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(20, 20))
        .build()
}

#[test]
fn call_forth_keeps_grouped_cascade_and_typed_spell_history_mana_value() {
    let definition = parse_oracle_card_definition("Call Forth the Tempest");

    let debug = format!("{:#?}", definition.spell_effect);
    assert!(
        debug.contains("TotalManaValueOfSpellsCastThisTurnMatching"),
        "damage amount must retain the history aggregate: {debug}"
    );
    assert!(debug.contains("exclude_source: true"), "{debug}");
    assert!(
        definition.abilities.iter().any(|ability| {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                return false;
            };
            matches!(
                static_ability.compiled_model().map(|model| &model.payload),
                Some(
                    ironsmith_core::StaticAbilityPayload::SourceLineKeywordGroup {
                        keyword_count: 2
                    }
                )
            )
        }),
        "the authored repeated keyword line must retain its grouping marker"
    );
    assert_eq!(canonical_compiled_lines(&definition).join("\n"), ORACLE);
}

#[test]
fn call_forth_sums_mana_values_of_only_its_controllers_other_spells() {
    let definition = parse_oracle_card_definition("Call Forth the Tempest");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    game.turn.active_player = alice;

    let yours = game.create_object_from_definition(
        &durable_creature("Alice Creature"),
        alice,
        Zone::Battlefield,
    );
    let theirs = game.create_object_from_definition(
        &durable_creature("Bob Creature"),
        bob,
        Zone::Battlefield,
    );

    record_cast(&mut game, alice, "Two-Mana Spell", 2);
    record_cast(&mut game, alice, "Four-Mana Spell", 4);
    record_cast(&mut game, bob, "Opponent Nine-Mana Spell", 9);

    let source = game.create_object_from_definition(&definition, alice, Zone::Stack);
    let source_event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::spells::SpellCastEvent::new(source, alice, Zone::Hand),
        crate::provenance::ProvNodeId::default(),
    );
    game.record_turn_history_event(&source_event);

    let program = definition
        .spell_effect
        .as_ref()
        .expect("Call Forth should have a spell program");
    let mut context = crate::effects::ExecutionContext::new_default(source, alice);
    crate::game_loop::execute_resolution_program(
        &mut game,
        &mut context,
        alice,
        source,
        program,
        None,
        &[],
    )
    .expect("Call Forth should resolve");

    assert_eq!(
        game.damage_on(theirs),
        6,
        "2 + 4 mana value should be dealt"
    );
    assert_eq!(
        game.damage_on(yours),
        0,
        "your own creatures are not damaged"
    );
}
