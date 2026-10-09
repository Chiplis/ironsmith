//! "For each kind of counter on target permanent or player, give that
//! permanent or player another counter of that kind": one more counter of
//! every kind already there, with no put-or-remove choice; players have
//! counters too (CR 122.1). Frozen complete bodies; source-authored and UNRUN.
use ironsmith::ability::AbilityKind;
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effects::{EffectContext, ForEachCounterKindPutOrRemoveEffect, ResolvedTarget, execute_effect};
use ironsmith::game_state::Phase;
use ironsmith::target::ChooseSpec;
use ironsmith::{CounterType, GameState, PlayerId, Zone};
use ironsmith_compiler_runtime::compile_to_runtime_definition;

#[path = "cf8_p08/support.rs"]
mod support;

const A: PlayerId = PlayerId::from_index(0);
const B: PlayerId = PlayerId::from_index(1);

const BODIES: &[(&str, &str)] = &[
    ("Maulfist Revolutionary", "Mana cost: {1}{G}{G}\nType: Creature — Human Warrior\nPower/Toughness: 3/3\nTrample\nWhen this creature enters or dies, for each kind of counter on target permanent or player, give that permanent or player another counter of that kind."),
    ("Skyship Plunderer", "Mana cost: {1}{U}\nType: Creature — Human Pirate\nPower/Toughness: 2/1\nFlying\nWhenever this creature deals combat damage to a player, for each kind of counter on target permanent or player, give that permanent or player another counter of that kind."),
    ("Powerful Broker", "Mana cost: {2}{G}\nType: Creature — Human Villain\nPower/Toughness: 3/3\n{T}: For each kind of counter on target permanent or player, give that permanent or player another counter of that kind. Activate only as a sorcery."),
];

#[test]
fn every_kind_gets_one_more_without_a_remove_option() {
    for (name, body) in BODIES {
        for definition in support::definitions(name, body) {
            let effects = support::find_all::<ForEachCounterKindPutOrRemoveEffect>(&definition);
            assert_eq!(effects.len(), 1, "{name}");
            let effect = &effects[0];
            assert!(effect.put_only && effect.all_kinds, "{name}: {effect:?}");
            assert!(effect.counter_source.is_none() && effect.fixed_counter_type.is_none());
            assert!(!effect.optional_action && !effect.choose_target_per_kind);
            assert!(effect.target.is_target(), "{name}");
            assert!(matches!(effect.target.base(), ChooseSpec::ObjectOrPlayer(..)), "{name}: {:?}", effect.target);
            let text = support::rendered(&definition);
            assert!(text.contains("another counter of that kind"), "{name}: {text}");
            assert!(!text.contains("remove"), "{name}: {text}");
        }
    }
}

#[test]
fn powerful_broker_adds_one_of_each_kind_to_a_player_or_a_permanent() {
    for definition in support::definitions("Powerful Broker", BODIES[2].1) {
        let AbilityKind::Activated(ability) = &definition
            .abilities
            .iter()
            .find(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .unwrap()
            .kind
        else {
            unreachable!()
        };
        // A player target: poison and energy each grow by exactly one.
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
        game.turn.active_player = A;
        game.turn.phase = Phase::FirstMain;
        let broker = game.create_object_from_definition(&definition, A, Zone::Battlefield);
        game.player_mut(B).unwrap().poison_counters = 2;
        game.player_mut(B).unwrap().energy_counters = 3;
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(broker, A, &mut dm).with_targets(vec![ResolvedTarget::Player(B)]);
        for effect in ability.effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        let bob = game.player(B).unwrap();
        assert_eq!(bob.poison_counters, 3);
        assert_eq!(bob.energy_counters, 4);
        assert_eq!(game.player(A).unwrap().poison_counters, 0);

        // A permanent target: each kind on it, and only on it, grows by one.
        let creature = game.create_object_from_definition(
            &compile_to_runtime_definition("Countered creature", "Type: Creature — Bear\nPower/Toughness: 2/2", false).unwrap(),
            B,
            Zone::Battlefield,
        );
        game.add_counters(creature, CounterType::PlusOnePlusOne, 2);
        game.add_counters(creature, CounterType::Flying, 1);
        let mut dm = SelectFirstDecisionMaker;
        let mut ctx = EffectContext::new(broker, A, &mut dm).with_targets(vec![ResolvedTarget::Object(creature)]);
        for effect in ability.effects.all_effects() {
            execute_effect(&mut game, effect, &mut ctx).unwrap();
        }
        assert_eq!(game.counter_count(creature, CounterType::PlusOnePlusOne), 3);
        assert_eq!(game.counter_count(creature, CounterType::Flying), 2);
        assert_eq!(game.player(B).unwrap().poison_counters, 3, "the player was not the target this time");
    }
}
