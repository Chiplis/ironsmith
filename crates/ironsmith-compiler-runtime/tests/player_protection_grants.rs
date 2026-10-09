//! cf8/p07: temporary protection from a player chosen on resolution
//! ("you and planeswalkers you control gain protection from that player",
//! "you gain protection from that player until your next turn").
//! CR 702.16k: protection from a player is protection from everything that
//! player controls; the player is the one this resolution named.
//! Source-authored, deliberately unrun.
#[path = "p07_support/mod.rs"]
mod support;

use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::{Effect, Until};
use ironsmith::effects::{EffectContext, ResolvedTarget, execute_effect};
use ironsmith::game_state::Phase;
use ironsmith::target::ChooseSpec;
use ironsmith::{CounterType, GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

const FIXTURE: &str = include_str!("../../../fixtures/player_protection_grants.json.fixture");
const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);
const C: PlayerId = PlayerId::from_index(2);

#[test]
fn eon_frolicker_protects_you_and_your_planeswalkers_from_the_target_opponent() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Eon Frolicker");
    assert_eq!(row["oracle_id"], "c0a18a53-d520-4795-9b3b-0f93bcf35848");
    for definition in support::definitions(row) {
        let triggers = support::triggered(&definition);
        assert_eq!(triggers.len(), 1);
        let effects = support::triggered_effects(triggers[0]);
        let debug = format!("{effects:?}");
        // The player half: can't be targeted from, damage prevented from,
        // the opponent's objects; the permanent half: a protection grant to
        // planeswalkers you control. All until your next turn.
        assert!(debug.contains("BeTargetedPlayerFrom(You"), "{debug}");
        assert!(debug.contains("Planeswalker"), "{debug}");
        assert!(debug.contains("Protection(Permanents"), "{debug}");
        assert!(debug.matches("YourNextTurn").count() >= 3, "{debug}");
        // "that player" is the targeted opponent, not a loop variable.
        assert!(!debug.contains("IteratedPlayer"), "{debug}");
        assert!(debug.contains("ExtraTurn"), "{debug}");

        // Gameplay: B is the target. B's creature can't damage you or your
        // planeswalker; C's creature still can.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        let frolicker = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let walker = compile_to_runtime_definition(
            "Witness Walker",
            "Mana cost: {3}\nType: Legendary Planeswalker — Witness\nLoyalty: 5",
            false,
        )
        .unwrap();
        let walker = game.create_object_from_definition(&walker, A, Zone::Battlefield);
        let bear = compile_to_runtime_definition(
            "Witness Bear",
            "Mana cost: {1}{G}\nType: Creature — Bear\nPower/Toughness: 2/2",
            false,
        )
        .unwrap();
        let bob_bear = game.create_object_from_definition(&bear, B, Zone::Battlefield);
        let charlie_bear = game.create_object_from_definition(&bear, C, Zone::Battlefield);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(frolicker, A, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in triggers[0].effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state().unwrap();

        let hit = |game: &mut GameState, source, controller, target: ChooseSpec| {
            let mut dm = SelectFirstDecisionMaker;
            execute_effect(
                game,
                &Effect::deal_damage(3, target),
                &mut EffectContext::new(source, controller, &mut dm),
            )
            .unwrap();
        };
        hit(&mut game, bob_bear, B, ChooseSpec::SpecificPlayer(A));
        assert_eq!(game.player(A).unwrap().life, 20, "damage from that player is prevented");
        hit(&mut game, bob_bear, B, ChooseSpec::SpecificObject(walker));
        assert_eq!(game.counter_count(walker, CounterType::Loyalty), 5);
        hit(&mut game, charlie_bear, C, ChooseSpec::SpecificPlayer(A));
        assert_eq!(game.player(A).unwrap().life, 17, "another opponent is unaffected");
        hit(&mut game, charlie_bear, C, ChooseSpec::SpecificObject(walker));
        assert_eq!(game.counter_count(walker, CounterType::Loyalty), 2);
    }
}

#[test]
fn noble_heritage_grants_per_opponent_protection_inside_the_granted_trigger() {
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Noble Heritage");
    assert_eq!(row["oracle_id"], "2e342aa3-2590-4b59-852a-9adb31bafe5c");
    for definition in support::definitions(row) {
        let debug = format!("{:?}", definition.abilities);
        // Each opponent who put the counters is the loop's player; the
        // protection names that player and lasts until your next turn.
        assert!(debug.contains("BeTargetedPlayerFrom(You"), "{debug}");
        assert!(debug.contains("IteratedPlayer"), "{debug}");
        assert!(debug.contains("YourNextTurn"), "{debug}");
        assert!(debug.contains("PreventAllDamage"), "{debug}");
        let _ = Until::YourNextTurn;
    }
}

#[test]
fn a_curse_controlled_by_that_player_falls_off_the_protected_player() {
    use ironsmith::game_loop::check_and_apply_sbas_with;
    use ironsmith::triggers::TriggerQueue;
    let rows = support::rows(FIXTURE);
    let row = support::row(&rows, "Eon Frolicker");
    for definition in support::definitions(row) {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        let curse = compile_to_runtime_definition(
            "Witness Curse",
            "Mana cost: {1}{B}\nType: Enchantment — Aura Curse\nEnchant player",
            false,
        )
        .unwrap();
        let bob_curse = game.create_object_from_definition(&curse, B, Zone::Battlefield);
        let charlie_curse = game.create_object_from_definition(&curse, C, Zone::Battlefield);
        for curse in [bob_curse, charlie_curse] {
            assert!(game.attach_object_to_target(curse, ironsmith::object::AttachmentTarget::Player(A)));
        }
        let bob_stable = game.object(bob_curse).unwrap().stable_id;
        let charlie_stable = game.object(charlie_curse).unwrap().stable_id;
        let frolicker = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        let trigger = support::triggered(&definition)[0];
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(frolicker, A, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in trigger.effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        game.refresh_continuous_state().unwrap();
        let mut queue = TriggerQueue::new();
        let mut dm = SelectFirstDecisionMaker;
        check_and_apply_sbas_with(&mut game, &mut queue, &mut dm).unwrap();
        // CR 702.16e / 704.5m: Bob's Curse can't enchant a player protected
        // from Bob; Charlie's Curse stays.
        let zone_of = |game: &GameState, stable| {
            game.find_object_by_stable_id(stable).and_then(|id| game.object(id)).map(|o| o.zone)
        };
        assert_eq!(zone_of(&game, bob_stable), Some(Zone::Graveyard));
        assert_eq!(zone_of(&game, charlie_stable), Some(Zone::Battlefield));
    }
}
