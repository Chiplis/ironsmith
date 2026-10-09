//! UNVALIDATED implementation-first coverage (cf8 p09): "Choose a counter on
//! <objects>. Put a counter of that kind on <recipients>" chooses one counter
//! (object and kind) and puts one counter of that kind on each recipient;
//! "each other" excludes the chosen object, "if it doesn't have a counter of
//! that kind on it" skips recipients that already have one (CR 122.1).
use ironsmith::decision::SelectFirstDecisionMaker;
use ironsmith::effect::Effect;
use ironsmith::effects::{EffectContext, PutCounterOfKindChosenFromEffect, execute_effect};
use ironsmith::object::CounterType;
use ironsmith::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use ironsmith::{GameState, PlayerId, Zone};

#[path = "p09_common/mod.rs"]
mod common;

const A: PlayerId = PlayerId::from_index(0);

fn rows() -> Vec<serde_json::Value> {
    common::rows(include_str!("../../../fixtures/chosen_counter_kinds.json.fixture"))
}

fn chosen_kind(definition: &ironsmith::cards::CardDefinition) -> PutCounterOfKindChosenFromEffect {
    common::all_effects(definition)
        .iter()
        .find_map(|effect| effect.downcast_ref::<PutCounterOfKindChosenFromEffect>().cloned())
        .expect("chosen-kind put")
}

#[test]
fn aven_courier_targets_a_permanent_without_that_kind() {
    for definition in common::definitions(common::row(&rows(), "Aven Courier")) {
        let effect = chosen_kind(&definition);
        assert!(effect.recipients.is_target());
        assert!(effect.only_if_absent);
        assert!(!effect.exclude_kind_object);
        assert_eq!(effect.kind_source.controller, Some(PlayerFilter::You));
    }
}

#[test]
fn contractual_safeguard_puts_on_each_other_creature() {
    for definition in common::definitions(common::row(&rows(), "Contractual Safeguard")) {
        let effect = chosen_kind(&definition);
        assert!(matches!(effect.recipients, ChooseSpec::All(_)));
        assert!(effect.exclude_kind_object);
        assert!(!effect.only_if_absent);
    }
}

fn bear(game: &mut GameState, name: &str) -> ironsmith::ObjectId {
    let definition = ironsmith_compiler_runtime::compile_to_runtime_definition(
        name,
        "Mana cost: {1}\nType: Creature — Bear\nPower/Toughness: 2/2",
        false,
    )
    .unwrap();
    game.create_object_from_definition(&definition, A, Zone::Battlefield)
}

#[test]
fn each_other_creature_gets_one_counter_of_the_chosen_kind() {
    let mut game = GameState::new(vec!["Alice".into(), "Bob".into()], 20);
    let first = bear(&mut game, "First Bear");
    let second = bear(&mut game, "Second Bear");
    let third = bear(&mut game, "Third Bear");
    game.object_mut(first)
        .unwrap()
        .counters
        .insert(CounterType::Shield, 1);
    let creatures = ObjectFilter::creature().controlled_by(PlayerFilter::You);
    let effect = PutCounterOfKindChosenFromEffect::new(creatures.clone(), ChooseSpec::All(creatures))
        .excluding_kind_object(true);
    let source = game.new_object_id();
    let mut dm = SelectFirstDecisionMaker;
    let mut ctx = EffectContext::new(source, A, &mut dm);
    execute_effect(&mut game, &Effect::new(effect), &mut ctx).unwrap();
    let shields = |game: &GameState, id| {
        game.object(id)
            .and_then(|object| object.counters.get(&CounterType::Shield).copied())
            .unwrap_or(0)
    };
    assert_eq!(shields(&game, first), 1, "the chosen creature gets none");
    assert_eq!(shields(&game, second), 1);
    assert_eq!(shields(&game, third), 1);
}
