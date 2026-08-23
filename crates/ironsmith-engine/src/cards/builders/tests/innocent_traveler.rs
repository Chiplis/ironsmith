#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

const INNOCENT_TRAVELER_TEXT: &str = "At the beginning of your upkeep, any opponent may sacrifice a creature of their choice. If no one does, transform this creature.";

#[test]
fn innocent_traveler_keeps_the_optional_opponent_result_set() {
    let definition = parse_oracle_card_definition("Innocent Traveler");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Innocent Traveler should have an upkeep trigger");
    let effects = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .collect::<Vec<_>>();
    let with_id = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<WithIdEffect>())
        .expect("the complete opponent offer should be tracked as one result set");
    let for_players = with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("the tracked offer should iterate the eligible opponents");

    assert_eq!(for_players.filter, PlayerFilter::Opponent);
    assert!(for_players.starting_with_controller);
    assert!(for_players.stop_after_first_happened);
    let [may_effect] = for_players.effects.as_slice() else {
        panic!("expected one optional action per offered opponent: {for_players:#?}");
    };
    let may = may_effect
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("each opponent should receive the sacrifice choice");
    assert_eq!(may.decider, Some(PlayerFilter::IteratedPlayer));
    let choice = may
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<ChooseObjectsEffect>())
        .expect("the offered opponent should choose their creature");
    assert_eq!(choice.chooser, PlayerFilter::IteratedPlayer);
    assert_eq!(choice.filter.controller, Some(PlayerFilter::IteratedPlayer));
    let sacrifice = may
        .effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<crate::effects::zones::SacrificePlayerEffect>())
        .expect("the same offered opponent should perform the sacrifice");
    assert_eq!(sacrifice.player, PlayerFilter::IteratedPlayer);
    assert_eq!(sacrifice.count, crate::Value::Fixed(1));

    let conditional = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<IfEffect>())
        .expect("the transform should depend on the aggregate offer result");
    assert_eq!(conditional.condition, with_id.id);
    assert_eq!(
        conditional.predicate,
        crate::effect::EffectPredicate::DidNotHappen
    );
    assert!(conditional.else_.is_empty());
    assert!(conditional.then.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::TransformEffect>()
            .is_some()
    }));
}

#[test]
fn innocent_traveler_renders_the_authored_no_one_condition_exactly() {
    let definition = parse_oracle_card_definition("Innocent Traveler");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [INNOCENT_TRAVELER_TEXT]
    );
}
