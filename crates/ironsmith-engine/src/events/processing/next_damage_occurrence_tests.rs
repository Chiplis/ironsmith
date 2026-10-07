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
            let outcome = crate::effects::damage::execute_damage_batch(
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
            let later = crate::effects::damage::execute_damage_batch(
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

fn finite_redirect(
    game: &mut GameState,
    shield: ObjectId,
    protected: PlayerId,
    destination: PlayerId,
    amount: u32,
) -> ReplacementEffectId {
    game.effect_store.replacement_effects.add_one_shot_effect(
        ReplacementEffect::with_matcher(
            shield, protected, DamageToPlayerMatcher::new(PlayerFilter::Specific(protected)),
            ReplacementAction::RedirectDamageAmount {
                target: RedirectTarget::ToPlayer(destination), which: RedirectWhich::First, amount,
            },
        ),
    )
}

struct AllocateBudget {
    prefer: &'static str,
    values: std::collections::VecDeque<u32>,
    offered: Vec<(PlayerId, u32, u32)>,
    pause: bool,
    invalid: bool,
    pending: bool,
}
impl AllocateBudget {
    fn new(prefer: &'static str, values: impl IntoIterator<Item = u32>) -> Self {
        Self {
            prefer, values: values.into_iter().collect(), offered: Vec::new(),
            pause: false, invalid: false, pending: false,
        }
    }
}
impl DecisionMaker for AllocateBudget {
    fn decide_options(
        &mut self,
        _: &GameState,
        context: &crate::decisions::context::SelectOptionsContext,
    ) -> Vec<usize> {
        vec![context.options.iter().find(|option| option.description.starts_with(self.prefer))
            .unwrap_or(&context.options[0]).index]
    }
    fn decide_number(
        &mut self,
        _: &GameState,
        context: &crate::decisions::context::NumberContext,
    ) -> u32 {
        assert!(context.description.starts_with("Choose how much redirected damage")
            || context.description.starts_with("Choose how much prevented damage"));
        self.offered.push((context.player, context.min, context.max));
        if self.pause {
            self.pending = true;
            return context.min;
        }
        if self.invalid {
            return context.max + 1;
        }
        self.values.pop_front().expect("unexpected damage allocation")
    }
    fn awaiting_choice(&self) -> bool { self.pending }
}

fn damage_to(result: &ProcessedDamageResult, player: PlayerId) -> u32 {
    result.assignments.iter().filter(|assignment| assignment.target == DamageTarget::Player(player))
        .map(|assignment| assignment.amount).sum()
}

#[test]
fn finite_redirect_can_choose_either_simultaneous_source_even_when_damage_is_unpreventable() {
    for first_amount in [0, 3] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let first = creature(&mut game, alice, "First source");
        let second = creature(&mut game, alice, "Second source");
        let shield = creature(&mut game, bob, "Redirect shield");
        let id = finite_redirect(&mut game, shield, bob, carol, 3);
        let mut events = [assignment(first, bob, 3), assignment(second, bob, 3)];
        for event in &mut events { event.unpreventable = true; }
        let mut dm = AllocateBudget::new("Redirect shield", [first_amount]);
        let result = process_simultaneous_damage_assignments_with_event_with_dm(
            &mut game, &events, &mut dm,
        ).unwrap();
        assert_eq!(dm.offered, vec![(bob, 0, 3)]);
        assert_eq!(damage_to(&result[0], carol), first_amount);
        assert_eq!(damage_to(&result[0], bob), 3 - first_amount);
        assert_eq!(damage_to(&result[1], carol), 3 - first_amount);
        assert_eq!(damage_to(&result[1], bob), first_amount);
        assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
    }
}

#[test]
fn finite_redirect_allocation_uses_chosen_post_multiplier_amounts() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let first = creature(&mut game, alice, "First source");
    let second = creature(&mut game, alice, "Second source");
    let shield = creature(&mut game, bob, "Redirect shield");
    let amplifier = creature(&mut game, bob, "Amplifier");
    finite_redirect(&mut game, shield, bob, carol, 3);
    game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
        amplifier, bob, DamageToPlayerMatcher::new(PlayerFilter::Specific(bob)),
        ReplacementAction::Modify(EventModification::Multiply(2)),
    ));
    let mut dm = AllocateBudget::new("Amplifier", [1]);
    let result = process_simultaneous_damage_assignments_with_event_with_dm(
        &mut game, &[assignment(first, bob, 1), assignment(second, bob, 1)], &mut dm,
    ).unwrap();
    assert_eq!(dm.offered, vec![(bob, 1, 2)], "raw total 2 was below the 3-point budget");
    assert_eq!((damage_to(&result[0], carol), damage_to(&result[0], bob)), (1, 1));
    assert_eq!((damage_to(&result[1], carol), damage_to(&result[1], bob)), (2, 0));
}

#[test]
fn finite_redirect_rechecks_eligibility_after_another_redirect_changes_the_recipient() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let first = creature(&mut game, alice, "First source");
    let second = creature(&mut game, alice, "Second source");
    let shield = creature(&mut game, bob, "Redirect shield");
    let detour = creature(&mut game, bob, "Detour");
    finite_redirect(&mut game, shield, bob, alice, 2);
    game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
        detour, bob, DamageFromSourceMatcher::new(ObjectFilter::specific(first)),
        ReplacementAction::Redirect { target: RedirectTarget::ToPlayer(carol), which: RedirectWhich::First },
    ));
    let mut dm = AllocateBudget::new("Detour", []);
    let result = process_simultaneous_damage_assignments_with_event_with_dm(
        &mut game, &[assignment(first, bob, 3), assignment(second, bob, 3)], &mut dm,
    ).unwrap();
    assert!(dm.offered.is_empty(), "only the still-protected source is eligible");
    assert_eq!((damage_to(&result[0], carol), damage_to(&result[0], alice)), (3, 0));
    assert_eq!((damage_to(&result[1], alice), damage_to(&result[1], bob)), (2, 1));
}

#[test]
fn multiple_finite_redirects_allocate_against_remaining_source_fragments_without_double_spending() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let first = creature(&mut game, alice, "First source");
    let second = creature(&mut game, alice, "Second source");
    let shield_a = creature(&mut game, bob, "First shield");
    let shield_b = creature(&mut game, bob, "Second shield");
    let a = finite_redirect(&mut game, shield_a, bob, alice, 2);
    let b = finite_redirect(&mut game, shield_b, bob, carol, 2);
    let mut dm = AllocateBudget::new("First shield", [1, 0]);
    let result = process_simultaneous_damage_assignments_with_event_with_dm(
        &mut game, &[assignment(first, bob, 3), assignment(second, bob, 3)], &mut dm,
    ).unwrap();
    assert_eq!(dm.offered, vec![(bob, 0, 2), (bob, 0, 2)]);
    assert_eq!((damage_to(&result[0], alice), damage_to(&result[0], bob), damage_to(&result[0], carol)), (1, 2, 0));
    assert_eq!((damage_to(&result[1], alice), damage_to(&result[1], bob), damage_to(&result[1], carol)), (1, 0, 2));
    assert!(game.effect_store.replacement_effects.get_effect(a).is_none());
    assert!(game.effect_store.replacement_effects.get_effect(b).is_none());
}

#[test]
fn redirect_allocation_pause_or_invalid_answer_restores_earlier_replacement_consumption() {
    for pause in [false, true] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let carol = PlayerId::from_index(2);
        let first = creature(&mut game, alice, "First source");
        let second = creature(&mut game, alice, "Second source");
        let shield = creature(&mut game, bob, "Redirect shield");
        let amplifier = creature(&mut game, bob, "Amplifier");
        let budget = finite_redirect(&mut game, shield, bob, carol, 3);
        let earlier = game.effect_store.replacement_effects.add_one_shot_effect(ReplacementEffect::with_matcher(
            amplifier, bob, DamageFromSourceMatcher::new(ObjectFilter::specific(first)),
            ReplacementAction::Modify(EventModification::Multiply(2)),
        ));
        let mut dm = AllocateBudget::new("Amplifier", []);
        dm.pause = pause;
        dm.invalid = !pause;
        let events = [assignment(first, bob, 2), assignment(second, bob, 2)];
        let outcome = process_simultaneous_damage_assignments_with_event_with_dm(&mut game, &events, &mut dm);
        if pause {
            assert!(outcome.unwrap().is_empty());
            assert!(dm.pending);
        } else {
            assert!(outcome.is_err());
        }
        assert!(game.effect_store.replacement_effects.get_effect(earlier).is_some());
        assert!(matches!(game.effect_store.replacement_effects.get_effect(budget).unwrap().replacement,
            ReplacementAction::RedirectDamageAmount { amount: 3, .. }));
        let mut replay = AllocateBudget::new("Amplifier", [1]);
        let result = process_simultaneous_damage_assignments_with_event_with_dm(&mut game, &events, &mut replay).unwrap();
        assert_eq!((damage_to(&result[0], carol), damage_to(&result[0], bob)), (1, 3));
        assert_eq!((damage_to(&result[1], carol), damage_to(&result[1], bob)), (2, 0));
        assert!(game.effect_store.replacement_effects.get_effect(earlier).is_none());
        assert!(game.effect_store.replacement_effects.get_effect(budget).is_none());
    }
}

#[test]
fn partially_used_redirect_keeps_its_budget_for_the_next_simultaneous_occurrence() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let first = creature(&mut game, alice, "First source");
    let second = creature(&mut game, alice, "Second source");
    let shield = creature(&mut game, bob, "Redirect shield");
    let id = finite_redirect(&mut game, shield, bob, carol, 3);
    process_simultaneous_damage_assignments_with_event(&mut game, &[assignment(first, bob, 1)]).unwrap();
    assert!(matches!(game.effect_store.replacement_effects.get_effect(id).unwrap().replacement,
        ReplacementAction::RedirectDamageAmount { amount: 2, .. }));
    let mut dm = AllocateBudget::new("Redirect shield", [0]);
    let later = process_simultaneous_damage_assignments_with_event_with_dm(
        &mut game, &[assignment(first, bob, 1), assignment(second, bob, 3)], &mut dm,
    ).unwrap();
    assert_eq!((damage_to(&later[0], carol), damage_to(&later[0], bob)), (0, 1));
    assert_eq!((damage_to(&later[1], carol), damage_to(&later[1], bob)), (2, 1));
    assert!(game.effect_store.replacement_effects.get_effect(id).is_none());
}

#[test]
fn redirected_recipient_discovers_its_shield_counter_without_reusing_an_ephemeral_identity() {
    for unpreventable in [false, true] {
        let mut game = game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = creature(&mut game, alice, "First source");
        let second = creature(&mut game, alice, "Second source");
        let recipient = creature(&mut game, bob, "Shielded redirect recipient");
        game.object_mut(recipient).unwrap().counters.insert(CounterType::Shield, 1);
        let budget = game.effect_store.replacement_effects.add_one_shot_effect(
            ReplacementEffect::with_matcher(
                recipient, bob, DamageToPlayerMatcher::new(PlayerFilter::Specific(bob)),
                ReplacementAction::RedirectDamageAmount {
                    target: RedirectTarget::ToObject(recipient), which: RedirectWhich::First, amount: 4,
                },
            ),
        );
        let mut events = [assignment(first, bob, 3), assignment(second, bob, 3)];
        for event in &mut events { event.unpreventable = unpreventable; }
        let mut dm = AllocateBudget::new("Shielded redirect recipient", [1]);
        let results = process_simultaneous_damage_assignments_with_event_with_dm(
            &mut game, &events, &mut dm,
        ).unwrap();
        assert_eq!(damage_to(&results[0], bob), 2);
        assert_eq!(damage_to(&results[1], bob), 0);
        let redirected = results.iter().flat_map(|result| &result.assignments)
            .filter(|assignment| assignment.target == DamageTarget::Object(recipient))
            .map(|assignment| assignment.amount).sum::<u32>();
        assert_eq!(redirected, if unpreventable { 4 } else { 0 });
        assert_eq!(game.counter_count(recipient, CounterType::Shield), 0,
            "one simultaneous damage event removes exactly one shield counter");
        assert!(game.effect_store.replacement_effects.get_effect(budget).is_none());
    }
}

#[test]
fn finite_prevention_and_redirection_allocate_in_the_chosen_replacement_order() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let first = creature(&mut game, alice, "First source");
    let second = creature(&mut game, alice, "Second source");
    let prevention = creature(&mut game, bob, "Prevention shield");
    let redirection = creature(&mut game, bob, "Redirect shield");
    let prevent_id = game.effect_store.prevention_effects.add_shield(
        crate::prevention::PreventionShield::prevent_next_n(
            prevention, bob, crate::prevention::PreventionTarget::Player(bob), 1,
        ),
    );
    let redirect_id = finite_redirect(&mut game, redirection, bob, carol, 2);
    let mut dm = AllocateBudget::new("Prevention shield", [0, 1]);
    let result = process_simultaneous_damage_assignments_with_event_with_dm(
        &mut game, &[assignment(first, bob, 2), assignment(second, bob, 2)], &mut dm,
    ).unwrap();
    assert_eq!(dm.offered, vec![(bob, 0, 1), (bob, 1, 2)]);
    assert_eq!((damage_to(&result[0], carol), damage_to(&result[0], bob)), (1, 1));
    assert_eq!((damage_to(&result[1], carol), damage_to(&result[1], bob)), (1, 0));
    assert_eq!(game.effect_store.prevention_effects.prevented_by_shield(prevent_id), 1);
    assert!(game.effect_store.replacement_effects.get_effect(redirect_id).is_none());
}

#[test]
fn later_damage_multiplier_does_not_spend_redirect_capacity_twice() {
    let mut game = game();
    let alice = PlayerId::from_index(0);
    let bob = PlayerId::from_index(1);
    let carol = PlayerId::from_index(2);
    let first = creature(&mut game, alice, "First source");
    let second = creature(&mut game, alice, "Second source");
    let shield = creature(&mut game, bob, "Redirect shield");
    let amplifier = creature(&mut game, bob, "Amplifier");
    let id = finite_redirect(&mut game, shield, bob, carol, 3);
    game.effect_store.replacement_effects.add_resolution_effect(ReplacementEffect::with_matcher(
        amplifier, bob, DamageFromSourceMatcher::new(ObjectFilter::creature()),
        ReplacementAction::Modify(EventModification::Multiply(2)),
    ));
    let mut dm = AllocateBudget::new("Redirect shield", []);
    let result = process_simultaneous_damage_assignments_with_event_with_dm(
        &mut game, &[assignment(first, bob, 1), assignment(second, bob, 1)], &mut dm,
    ).unwrap();
    assert!(dm.offered.is_empty());
    assert_eq!(damage_to(&result[0], carol), 2);
    assert_eq!(damage_to(&result[1], carol), 2);
    assert!(matches!(game.effect_store.replacement_effects.get_effect(id).unwrap().replacement,
        ReplacementAction::RedirectDamageAmount { amount: 1, .. }));
}
