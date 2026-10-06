//! Source-authored scenarios; intentionally not executed in the recovery pass.
use super::*;
use crate::card::{CardBuilder, PowerToughness};
use crate::effect::{Effect, Value};
use crate::effects::{EffectExecutor, ExecutionContext};
use crate::events::cause::EventCause;
use crate::events::damage::matchers::{
    DamageFromSourceMatcher, DamageToObjectMatcher, DamageToPlayerMatcher,
};
use crate::ids::CardId;
use crate::replacement::{EventModification, RedirectTarget, RedirectWhich};
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::CardType;

fn creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
    let card = CardBuilder::new(CardId::new(), name)
        .card_types(vec![CardType::Creature])
        .power_toughness(PowerToughness::fixed(2, 20))
        .build();
    game.create_object_from_card(&card, owner, Zone::Battlefield)
}

fn game() -> GameState {
    GameState::new(vec!["Alice".into(), "Bob".into(), "Carol".into()], 20)
}

fn assignment(source: ObjectId, target: PlayerId, amount: u32) -> SimultaneousDamageEvent {
    SimultaneousDamageEvent {
        source,
        target: DamageTarget::Player(target),
        amount,
        is_combat: false,
        unpreventable: false,
        cause: EventCause::from_effect(source, PlayerId::from_index(0)),
        source_snapshot: None,
    }
}

fn redirect(
    game: &mut GameState,
    shield_source: ObjectId,
    protected: PlayerId,
    target: RedirectTarget,
) -> ReplacementEffectId {
    game.effect_store.replacement_effects.add_next_damage_occurrence_effect(
        ReplacementEffect::with_matcher(
            shield_source,
            protected,
            DamageToPlayerMatcher::new(PlayerFilter::Specific(protected)),
            ReplacementAction::Redirect { target, which: RedirectWhich::First },
        ),
    )
}

#[test]
fn next_time_redirects_every_sibling_and_retains_actual_source_and_combat_history() {
    for same_source in [false, true] {
        for combat in [false, true] {
            let mut game = game();
            let alice = PlayerId::from_index(0);
            let bob = PlayerId::from_index(1);
            let shield_source = creature(&mut game, bob, "Shield source");
            let first = creature(&mut game, alice, "First damaging source");
            let second = if same_source { first } else {
                creature(&mut game, alice, "Second damaging source")
            };
            let id = redirect(&mut game, shield_source, bob, RedirectTarget::ToSource);
            let mut events = vec![assignment(first, bob, 2), assignment(second, bob, 3)];
            for event in &mut events {
                event.is_combat = combat;
                event.unpreventable = true;
                if combat {
                    event.cause = EventCause::combat_damage(event.source);
                }
            }
            let mut ctx = ExecutionContext::new_default(shield_source, bob);
            let outcome = crate::effects::damage::commit_damage_batch(
                &mut game, &mut ctx, events.clone(), None,
            ).unwrap();
            assert_eq!(outcome.count_or_zero(), 5);
            assert_eq!(game.player(bob).unwrap().life, 20);
            assert_eq!(game.damage_on(shield_source), 0);
            assert_eq!(game.damage_on(first), if same_source { 5 } else { 2 });
            assert_eq!(game.damage_on(second), if same_source { 5 } else { 3 });
            let damage = outcome.events.iter()
                .filter_map(|event| event.downcast::<crate::events::DamageEvent>())
                .collect::<Vec<_>>();
            assert_eq!(damage.len(), 2);
            for (receipt, original) in damage.iter().zip(&events) {
                assert_eq!(receipt.source, original.source);
                assert_eq!(receipt.target, DamageTarget::Object(original.source));
                assert_eq!(receipt.amount, original.amount);
                assert_eq!(receipt.is_combat, combat);
                assert_eq!(receipt.cause, original.cause);
            }
            assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
            let later = crate::effects::damage::commit_damage_batch(
                &mut game, &mut ctx, events, None,
            ).unwrap();
            assert_eq!(later.count_or_zero(), 5);
            assert_eq!(game.player(bob).unwrap().life, 15);
        }
    }
}

#[test]
fn unmatched_and_zero_damage_occurrences_leave_the_next_matching_occurrence_available() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let source = creature(&mut game, alice, "Damage source");
    let shield = creature(&mut game, bob, "Shield source");
    let id = redirect(&mut game, shield, bob, RedirectTarget::ToSource);
    process_simultaneous_damage_assignments_with_event(
        &mut game, &[assignment(source, carol, 2), assignment(source, bob, 0)],
    ).unwrap();
    assert!(game.effect_store.replacement_effects.get_effect(id).is_some());
    let matching = process_damage_assignments_with_event(
        &mut game, source, DamageTarget::Player(bob), 3, false, EventCause::effect(),
    ).unwrap();
    assert_eq!(matching.assignments, vec![ProcessedDamageAssignment {
        target: DamageTarget::Object(source), amount: 3,
    }]);
    assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
}

#[test]
fn next_occurrence_includes_split_fragments_without_reapplying_prior_history() {
    for before_split in [false, true] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let source = creature(&mut game, alice, "Damage source");
        let mut split = ReplacementEffect::with_matcher(
            source, alice, DamageToPlayerMatcher::new(PlayerFilter::Specific(bob)),
            ReplacementAction::RedirectDamageAmount {
                target: RedirectTarget::ToPlayer(carol),
                which: RedirectWhich::First,
                amount: 1,
            },
        );
        let mut multiplier = ReplacementEffect::with_matcher(
            source, alice, DamageFromSourceMatcher::new(ObjectFilter::specific(source)),
            ReplacementAction::Modify(EventModification::Multiply(2)),
        );
        if before_split {
            multiplier.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
        } else {
            split.priority_override = Some(crate::events::ReplacementPriority::SelfReplacement);
        }
        game.effect_store.replacement_effects.add_resolution_effect(split);
        let id = game.effect_store.replacement_effects.add_next_damage_occurrence_effect(multiplier);
        let outcome = crate::effects::DealDamageEffect::new(4, ChooseSpec::SpecificPlayer(bob))
            .execute(&mut game, &mut ExecutionContext::new_default(source, alice)).unwrap();
        assert_eq!(outcome.count_or_zero(), 8);
        assert_eq!(game.player(bob).unwrap().life, if before_split { 13 } else { 14 });
        assert_eq!(game.player(carol).unwrap().life, if before_split { 19 } else { 18 });
        assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
    }
}

#[test]
fn consumed_outer_occurrence_is_unavailable_to_nested_damage_then_returns_for_siblings() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let source = creature(&mut game, alice, "Damage source");
    let shield_source = creature(&mut game, bob, "Shield source");
    let id = redirect(&mut game, shield_source, bob, RedirectTarget::ToSource);
    let payload = game.effect_store.replacement_effects.add_one_shot_effect(
        ReplacementEffect::with_matcher(
            shield_source, bob, DamageToObjectMatcher::new(ObjectFilter::specific(source)),
            ReplacementAction::Instead(vec![Effect::new(
                crate::effects::DealDamageEffect::new(1, ChooseSpec::SpecificPlayer(bob)),
            )]),
        ),
    );
    let results = process_simultaneous_damage_assignments_with_event(
        &mut game, &[assignment(source, bob, 2), assignment(source, bob, 3)],
    ).unwrap();
    assert!(results[0].assignments.is_empty());
    assert_eq!(game.player(bob).unwrap().life, 19, "nested damage starts a new occurrence");
    assert_eq!(results[1].assignments, vec![ProcessedDamageAssignment {
        target: DamageTarget::Object(source), amount: 3,
    }]);
    assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
    assert!(game.effect_store.replacement_effects.get_effect(payload).is_none());
}

struct Pause { pending: bool }
impl DecisionMaker for Pause {
    fn decide_boolean(
        &mut self,
        _: &GameState,
        _: &crate::decisions::context::BooleanContext,
    ) -> bool {
        self.pending = true;
        false
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

#[test]
fn later_sibling_failure_or_unanswered_payload_restores_occurrence_and_source_identity() {
    for pause in [false, true] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = creature(&mut game, alice, "First source");
        let second = creature(&mut game, alice, "Second source");
        let shield = creature(&mut game, bob, "Shield source");
        let id = redirect(&mut game, shield, bob, RedirectTarget::ToSource);
        let key = game.effect_store.replacement_effects.get_effect(id).unwrap().application_key();
        let mut effects = vec![Effect::gain_life(2)];
        effects.push(if pause { Effect::may(vec![Effect::gain_life(1)]) }
            else { Effect::gain_life(Value::X) });
        let failure = game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                shield, bob, DamageToObjectMatcher::new(ObjectFilter::specific(second)),
                ReplacementAction::Instead(effects),
            ),
        );
        game.take_pending_trigger_events();
        let events = [assignment(first, bob, 2), assignment(second, bob, 3)];
        let mut dm = Pause { pending: false };
        let result = process_simultaneous_damage_assignments_with_event_with_dm(
            &mut game, &events, &mut dm,
        );
        if pause {
            assert!(result.unwrap().is_empty());
            assert!(dm.pending);
        } else {
            assert!(matches!(result.unwrap_err().error, crate::effects::ExecutionError::UnresolvableValue(_)));
        }
        assert_eq!(game.player(bob).unwrap().life, 20);
        assert_eq!(game.damage_on(first), 0);
        assert_eq!(game.damage_on(second), 0);
        assert!(game.take_pending_trigger_events().is_empty());
        let restored = game.effect_store.replacement_effects.get_effect(id).unwrap();
        assert_eq!(restored.source, shield);
        assert_eq!(restored.application_key(), key);
        game.effect_store.replacement_effects.remove_effect(failure);
        let replay = process_simultaneous_damage_assignments_with_event(&mut game, &events).unwrap();
        assert_eq!(replay[0].assignments, vec![ProcessedDamageAssignment {
            target: DamageTarget::Object(first), amount: 2,
        }]);
        assert_eq!(replay[1].assignments, vec![ProcessedDamageAssignment {
            target: DamageTarget::Object(second), amount: 3,
        }]);
        assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
    }
}

#[test]
fn impossible_redirect_destinations_preserve_original_damage_and_unused_shield() {
    for case in 0..6 {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let source = creature(&mut game, alice, "Damage source");
        let destination = creature(&mut game, bob, "Redirect destination");
        let target = match case {
            0 => RedirectTarget::ToObject(game.new_object_id()),
            1 => {
                game.move_object(destination, Zone::Graveyard, EventCause::effect()).unwrap();
                RedirectTarget::ToObject(destination)
            }
            2 => {
                game.object_mut(destination).unwrap().card_types = vec![CardType::Artifact].into();
                RedirectTarget::ToObject(destination)
            }
            3 => {
                game.phase_out(destination);
                RedirectTarget::ToObject(destination)
            }
            4 => {
                game.player_mut(carol).unwrap().has_left_game = true;
                RedirectTarget::ToPlayer(carol)
            }
            _ => {
                game.move_object(source, Zone::Graveyard, EventCause::effect()).unwrap();
                RedirectTarget::ToSource
            }
        };
        let id = redirect(&mut game, destination, bob, target);
        let result = process_damage_assignments_with_event(
            &mut game, source, DamageTarget::Player(bob), 3, false, EventCause::effect(),
        ).unwrap();
        assert_eq!(result.assignments, vec![ProcessedDamageAssignment {
            target: DamageTarget::Player(bob), amount: 3,
        }], "case {case}");
        assert!(game.effect_store.replacement_effects.get_effect(id).is_some(), "case {case}");
    }
}

#[test]
fn source_controller_redirection_uses_exact_departure_lki_before_stale_event_snapshot() {
    for supplied in [false, true] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let source = creature(&mut game, alice, "Departed damage source");
        let shield = creature(&mut game, bob, "Shield source");
        let snapshot = crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(source).unwrap(), &game,
        );
        game.set_current_controller(source, carol).unwrap();
        let id = redirect(&mut game, shield, bob, RedirectTarget::ToSourceController);
        game.move_object(source, Zone::Graveyard, EventCause::effect()).unwrap();
        assert_eq!(game.turn_store.turn_history.source_last_known_snapshot(source).unwrap().controller, carol);
        let result = process_damage_assignments_with_event_with_source_snapshot(
            &mut game, source, DamageTarget::Player(bob), 3, false, EventCause::effect(),
            supplied.then_some(&snapshot),
        ).unwrap();
        assert_eq!(result.assignments, vec![ProcessedDamageAssignment {
            target: DamageTarget::Player(carol), amount: 3,
        }]);
        assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
    }
}
