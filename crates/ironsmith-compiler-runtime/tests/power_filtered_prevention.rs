use ironsmith::card::{CardBuilder, PowerToughness};
use ironsmith::cards::CardDefinition;
use ironsmith::effects::PreventAllDamageEffect;
use ironsmith::events::DamageTarget;
use ironsmith::events::cause::EventCause;
use ironsmith::events::processing::process_damage_assignments_with_event;
use ironsmith::filter::Comparison;
use ironsmith::game_state::StackEntry;
use ironsmith::object::CounterType;
use ironsmith::{CardId, CardType, GameState, ObjectId, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_builder_to_artifact;

const VINE_SNARE: &str =
    "Prevent all combat damage that would be dealt this turn by creatures with power 4 or less.";
const FOG_OF_WAR: &str = "You gain 1 life for each creature on the battlefield. Prevent all combat damage that would be dealt this turn by creatures with power 3 or less.";

fn compile(name: &str, text: &str) -> CardDefinition {
    let (_, definition) = compile_builder_to_artifact(
        ironsmith_compiler::CardDefinitionBuilder::new(CardId::new(), name),
        format!("Mana cost: {{2}}{{G}}\nType: Instant\n{text}"),
        false,
    )
    .expect("filtered prevention must strict-compile and materialize from its artifact");
    definition
}

fn add_creature(game: &mut GameState, owner: PlayerId, power: i32) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), format!("Power {power} creature"))
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(power, 8))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

fn resolve(game: &mut GameState, definition: &CardDefinition) {
    let caster = PlayerId::from_index(0);
    let spell = game.create_object_from_definition(definition, caster, Zone::Stack);
    game.push_to_stack(StackEntry::new(spell, caster));
    ironsmith::game_loop::resolve_stack_entry(game).expect("prevention spell should resolve");
}

fn remaining_damage(game: &mut GameState, source: ObjectId, combat: bool) -> u32 {
    let result = process_damage_assignments_with_event(
        game,
        source,
        DamageTarget::Player(PlayerId::from_index(0)),
        2,
        combat,
        EventCause::effect(),
    )
    .expect("damage should be processed with prevention");
    result
        .assignments
        .iter()
        .map(|assignment| assignment.amount)
        .sum()
}

#[test]
fn power_filtered_prevention_strict_artifact_preserves_typed_source_filter() {
    for (name, text, threshold) in [
        ("Vine Snare", VINE_SNARE, 4),
        ("Fog of War", FOG_OF_WAR, 3),
        ("Synthetic Meadow Shelter", VINE_SNARE, 4),
    ] {
        let definition = compile(name, text);
        let prevention = definition
            .spell_effect
            .as_ref()
            .expect("spell has a resolution program")
            .all_effects()
            .into_iter()
            .find_map(|effect| effect.downcast_ref::<PreventAllDamageEffect>())
            .expect("qualified combat damage must become a filtered prevention effect");
        assert!(prevention.damage_filter.combat_only);
        assert!(!prevention.damage_filter.noncombat_only);
        assert!(prevention.damage_filter.from_specific_source.is_none());
        let sources = prevention.damage_filter.from_source.as_ref().unwrap();
        assert_eq!(sources.card_types, vec![CardType::Creature]);
        assert_eq!(sources.power, Some(Comparison::LessThanOrEqual(threshold)));
        assert_eq!(prevention.until, ironsmith::effect::Until::EndOfTurn);
    }
}

#[test]
fn power_filtered_prevention_checks_threshold_at_damage_time_and_includes_later_creatures() {
    for (name, text, threshold) in [
        ("Vine Snare", VINE_SNARE, 4),
        ("Fog of War", FOG_OF_WAR, 3),
        ("Synthetic Meadow Shelter", VINE_SNARE, 4),
    ] {
        let definition = compile(name, text);
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let below = add_creature(&mut game, alice, threshold - 1);
        let boundary = add_creature(&mut game, bob, threshold);
        let above = add_creature(&mut game, bob, threshold + 1);
        resolve(&mut game, &definition);

        let expected_life = if text == FOG_OF_WAR { 23 } else { 20 };
        assert_eq!(game.player(alice).unwrap().life, expected_life);
        assert_eq!(remaining_damage(&mut game, below, true), 0);
        assert_eq!(remaining_damage(&mut game, boundary, true), 0);
        assert_eq!(remaining_damage(&mut game, above, true), 2);
        assert_eq!(remaining_damage(&mut game, boundary, false), 2);

        game.add_counters(boundary, CounterType::PlusOnePlusOne, 1)
            .expect("the boundary creature should grow above the threshold");
        assert_eq!(remaining_damage(&mut game, boundary, true), 2);
        let later = add_creature(&mut game, bob, threshold);
        assert_eq!(remaining_damage(&mut game, later, true), 0);

        game.effect_store.prevention_effects.cleanup_end_of_turn();
        assert_eq!(remaining_damage(&mut game, later, true), 2);
    }
}

#[test]
fn power_filtered_prevention_does_not_prevent_unpreventable_damage() {
    let definition = compile("Synthetic Meadow Shelter", VINE_SNARE);
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let creature = add_creature(&mut game, PlayerId::from_index(1), 4);
    resolve(&mut game, &definition);
    let result = ironsmith::events::processing::process_damage_assignments_with_event_with_source_snapshot_opts(
        &mut game,
        creature,
        DamageTarget::Player(PlayerId::from_index(0)),
        2,
        true,
        true,
        EventCause::effect(),
        None,
    )
    .expect("unpreventable damage should be processed");
    assert_eq!(
        result
            .assignments
            .iter()
            .map(|assignment| assignment.amount)
            .sum::<u32>(),
        2
    );
}
