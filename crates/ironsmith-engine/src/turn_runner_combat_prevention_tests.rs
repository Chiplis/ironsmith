use super::*;
use crate::effect::Until;
use crate::prevention::{
    PreventionEffectManager, PreventionShield, PreventionShieldId, PreventionTarget,
};

fn game_with_combat_shield() -> (GameState, crate::ids::ObjectId, PreventionShieldId) {
    let mut game = crate::tests::test_helpers::setup_two_player_game();
    let alice = PlayerId::from_index(0);
    game.turn.active_player = alice;
    game.turn.phase = Phase::Combat;
    game.turn.step = Some(Step::CombatDamage);
    let source = game.create_object_from_card(
        &crate::card::CardBuilder::new(crate::ids::CardId::new(), "Combat shield source")
            .card_types(vec![crate::types::CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 6))
            .build(),
        alice,
        crate::Zone::Battlefield,
    );
    game.effect_store
        .prevention_effects
        .set_turn(game.turn.turn_number);
    let id = game
        .effect_store
        .prevention_effects
        .add_shield(PreventionShield::new(
            source,
            alice,
            PreventionTarget::You,
            None,
            Until::EndOfCombat,
        ));
    // The restored owning state must have the same lifecycle as fresh shields.
    game.effect_store.prevention_effects = game.effect_store.prevention_effects.clone();
    (game, source, id)
}

fn remaining(game: &mut GameState, source: crate::ids::ObjectId) -> u32 {
    crate::events::processing::process_damage_assignments_with_event(
        game,
        source,
        crate::events::DamageTarget::Player(PlayerId::from_index(0)),
        3,
        true,
        crate::events::cause::EventCause::effect(),
    )
    .unwrap()
    .assignments
    .iter()
    .map(|damage| damage.amount)
    .sum()
}

#[test]
fn combat_shield_expires_before_an_extra_combat_begins() {
    let (mut game, source, id) = game_with_combat_shield();
    assert_eq!(remaining(&mut game, source), 0);
    game.turn.step = Some(Step::EndCombat);
    game.turn_store.combat_phases_started_this_turn = 1;
    game.turn_store.additional_phases.push(Phase::Combat);
    game.turn_store.additional_phase_continuation = Some(Phase::NextMain);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::EndCombatPriority);
    let mut tq = TriggerQueue::new();
    runner.advance(&mut game, &mut tq).unwrap();
    assert!(
        !game
            .effect_store
            .prevention_effects
            .shields()
            .iter()
            .any(|shield| shield.id == id)
    );
    assert!(matches!(runner.state(), TurnState::BeginCombat));
    runner.advance(&mut game, &mut tq).unwrap();
    assert_eq!(game.turn_store.combat_phases_started_this_turn, 2);
    assert_eq!(remaining(&mut game, source), 3);
}

#[test]
fn skipping_end_combat_step_still_expires_the_shield() {
    let (mut game, source, _) = game_with_combat_shield();
    game.skip_next_step(PlayerId::from_index(0), Step::EndCombat);
    let mut runner = TurnRunner::from_state_for_sync(TurnState::EndCombat);
    let mut tq = TriggerQueue::new();
    runner.advance(&mut game, &mut tq).unwrap();
    assert!(matches!(runner.state(), TurnState::SkippedPhaseEndMana));
    runner.advance(&mut game, &mut tq).unwrap();
    assert!(game.effect_store.prevention_effects.shields().is_empty());
    assert_eq!(remaining(&mut game, source), 3);
}

#[test]
fn ending_combat_or_turn_uses_the_same_shield_expiry_boundary() {
    for effect in [
        crate::effect::Effect::end_combat_phase(),
        crate::effect::Effect::end_turn(),
    ] {
        let (mut game, source, _) = game_with_combat_shield();
        let mut context =
            crate::effects::EffectContext::new_default(source, PlayerId::from_index(0));
        crate::effects::execute_effect(&mut game, &effect, &mut context).unwrap();
        let mut runner = TurnRunner::from_state_for_sync(TurnState::CombatDamageRegular);
        let mut tq = TriggerQueue::new();
        runner.advance(&mut game, &mut tq).unwrap();
        assert!(game.effect_store.prevention_effects.shields().is_empty());
        assert_eq!(remaining(&mut game, source), 3);
    }
}
