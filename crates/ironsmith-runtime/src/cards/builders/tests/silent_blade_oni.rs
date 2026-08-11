#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;
use crate::decision::{AutoPassDecisionMaker, DecisionMaker, SelectFirstDecisionMaker};

const EXPECTED_TEXT: &str = "Ninjutsu {4}{U}{B}\nWhenever this creature deals combat damage to a player, look at that player's hand. You may cast a spell from among those cards without paying its mana cost.";

fn combat_damage_trigger(definition: &CardDefinition) -> &crate::ability::TriggeredAbility {
    definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Silent-Blade Oni should have its combat-damage trigger")
}

fn resolve_combat_damage_trigger(
    game: &mut crate::GameState,
    definition: &CardDefinition,
    source: ObjectId,
    controller: PlayerId,
    damaged_player: PlayerId,
    decisions: &mut dyn DecisionMaker,
) {
    let event = crate::triggers::TriggerEvent::new_with_provenance(
        crate::events::DamageEvent::with_cause(
            source,
            crate::events::DamageTarget::Player(damaged_player),
            6,
            true,
            crate::events::cause::EventCause::combat_damage(source),
        ),
        crate::provenance::ProvNodeId::default(),
    );
    let mut context = crate::effects::ExecutionContext::new(source, controller, decisions)
        .with_triggering_event(event);
    crate::game_loop::execute_resolution_program(
        game,
        &mut context,
        controller,
        source,
        &combat_damage_trigger(definition).effects,
        None,
        &[],
    )
    .expect("Silent-Blade Oni combat-damage trigger should resolve");
}

fn test_card(name: &str, owner_spell: bool) -> CardDefinition {
    let builder = CardDefinitionBuilder::new(CardId::new(), name);
    if owner_spell {
        builder
            .card_types(vec![CardType::Sorcery])
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(7)]]))
            .build()
    } else {
        builder.card_types(vec![CardType::Land]).build()
    }
}

#[test]
fn silent_blade_oni_keeps_optional_cast_from_the_damaged_players_hand() {
    let definition = parse_oracle_card_definition("Silent-Blade Oni");
    assert_eq!(
        canonical_compiled_lines(&definition).join("\n"),
        EXPECTED_TEXT,
        "{:#?}",
        combat_damage_trigger(&definition).effects,
    );

    let effects = combat_damage_trigger(&definition)
        .effects
        .flattened_default_effects();
    let [look_effect, cast_effect] = effects else {
        panic!("expected look plus one typed optional cast: {effects:#?}");
    };
    let look = look_effect
        .downcast_ref::<crate::effects::LookAtHandEffect>()
        .expect("the first sentence should look at the damaged player's hand");
    assert_eq!(look.target, ChooseSpec::Player(PlayerFilter::DamagedPlayer));
    assert!(!look.reveal);

    let cast = cast_effect
        .downcast_ref::<crate::effects::MayCastMatchingSpellWithoutPayingManaCostEffect>()
        .expect("the second sentence should remain an optional matching-spell cast");
    assert_eq!(cast.player, PlayerFilter::You);
    assert_eq!(cast.zone_owner, PlayerFilter::DamagedPlayer);
    assert_eq!(cast.zone, Zone::Hand);
    assert_eq!(cast.filter, ObjectFilter::nonland().in_zone(Zone::Hand));
    assert_eq!(
        cast.payment,
        ironsmith_core::MayCastMatchingSpellPayment::WithoutPayingManaCost
    );
}

#[test]
fn silent_blade_oni_casts_only_from_the_damaged_players_hand() {
    let definition = parse_oracle_card_definition("Silent-Blade Oni");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);

    let spell = game.create_object_from_definition(
        &test_card("Damaged Player Spell", true),
        bob,
        Zone::Hand,
    );
    let spell_stable = game.object(spell).expect("spell exists").stable_id;
    let land = game.create_object_from_definition(
        &test_card("Damaged Player Land", false),
        bob,
        Zone::Hand,
    );
    let unrelated = game.create_object_from_definition(
        &test_card("Unrelated Exiled Spell", true),
        bob,
        Zone::Exile,
    );

    let mut decisions = SelectFirstDecisionMaker;
    resolve_combat_damage_trigger(&mut game, &definition, source, alice, bob, &mut decisions);

    let cast_spell = game
        .find_object_by_stable_id(spell_stable)
        .and_then(|id| game.object(id))
        .expect("the chosen spell should still exist");
    assert_eq!(cast_spell.zone, Zone::Stack);
    assert_eq!(cast_spell.owner, bob);
    assert_eq!(
        game.stack.last().map(|entry| entry.controller),
        Some(alice),
        "Silent-Blade Oni's controller should control the free cast"
    );
    assert_eq!(game.object(land).expect("land exists").zone, Zone::Hand);
    assert_eq!(
        game.object(unrelated).expect("unrelated spell exists").zone,
        Zone::Exile,
        "an unrelated exiled spell must not enter the looked-hand choice"
    );
}

#[test]
fn silent_blade_oni_optional_cast_can_be_declined() {
    let definition = parse_oracle_card_definition("Silent-Blade Oni");
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = game.create_object_from_definition(&definition, alice, Zone::Battlefield);
    let spell = game.create_object_from_definition(
        &test_card("Declined Hand Spell", true),
        bob,
        Zone::Hand,
    );

    let mut decisions = AutoPassDecisionMaker;
    resolve_combat_damage_trigger(&mut game, &definition, source, alice, bob, &mut decisions);

    assert!(game.stack.is_empty());
    assert_eq!(game.object(spell).expect("spell exists").zone, Zone::Hand);
}
