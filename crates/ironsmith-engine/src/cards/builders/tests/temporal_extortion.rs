#![cfg(ironsmith_runtime_parser_tests)]

use super::shard_16::parse_oracle_card_definition;
use super::*;

#[test]
fn temporal_extortion_preserves_the_sequential_payer_relative_offer() {
    let definition = parse_oracle_card_definition("Temporal Extortion");
    let triggered = definition
        .abilities
        .iter()
        .find_map(|ability| match &ability.kind {
            AbilityKind::Triggered(triggered) => Some(triggered),
            _ => None,
        })
        .expect("Temporal Extortion should have a cast trigger");
    let effects = triggered
        .effects
        .segments
        .iter()
        .flat_map(|segment| &segment.default_effects)
        .collect::<Vec<_>>();
    let with_id = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<WithIdEffect>())
        .expect("the complete payment offer should have one result identity");
    let players = with_id
        .effect
        .downcast_ref::<crate::effects::ForPlayersEffect>()
        .expect("the offer should visit players in turn order");

    assert_eq!(players.filter, PlayerFilter::Any);
    assert!(players.starting_with_controller);
    assert!(players.stop_after_first_happened);
    let [may] = players.effects.as_slice() else {
        panic!("expected one optional payment per offered player: {players:#?}");
    };
    let may = may
        .downcast_ref::<crate::effects::MayEffect>()
        .expect("each player should choose whether to pay");
    assert_eq!(may.decider, Some(PlayerFilter::IteratedPlayer));
    let [payment] = may.effects.as_slice() else {
        panic!("expected one half-life payment: {may:#?}");
    };
    let payment = payment
        .downcast_ref::<crate::effects::PayLifeEffect>()
        .expect("the optional action should pay life rather than lose life");
    assert_eq!(
        payment.player,
        ChooseSpec::Player(PlayerFilter::IteratedPlayer)
    );
    assert_eq!(
        payment.amount,
        crate::Value::HalfLifeTotalRoundedUp(PlayerFilter::IteratedPlayer)
    );

    let conditional = effects
        .iter()
        .find_map(|effect| effect.downcast_ref::<IfEffect>())
        .expect("the counter action should depend on the payment result");
    assert_eq!(conditional.condition, with_id.id);
    assert_eq!(
        conditional.predicate,
        crate::effect::EffectPredicate::Happened
    );
    assert!(conditional.else_.is_empty());
    assert!(conditional.then.iter().any(|effect| {
        effect
            .downcast_ref::<crate::effects::CounterEffect>()
            .is_some()
    }));
}

#[test]
fn temporal_extortion_renders_the_public_card_exactly() {
    let definition = parse_oracle_card_definition("Temporal Extortion");
    assert_eq!(
        canonical_compiled_lines(&definition),
        [
            "When you cast this spell, any player may pay half their life, rounded up. If a player does, counter this spell.",
            "You take an extra turn after this one.",
        ]
    );
}
