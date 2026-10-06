//! Fight effect implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{CompletedEffectOutputs, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::events::processing::SimultaneousDamageEvent;
use crate::events::{DamageTarget, EventKind};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::filter::ObjectFilterExt;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;

/// Effect that makes two creatures fight.
///
/// Each creature deals damage equal to its power to the other.
///
/// # Fields
///
/// * `creature1` - First creature (often "target creature you control")
/// * `creature2` - Second creature (often "target creature you don't control")
///
/// # Example
///
/// ```ignore
/// // Target creature you control fights target creature you don't control
/// let effect = FightEffect::new(
///     ChooseSpec::creature().you_control(),
///     ChooseSpec::creature().opponent_controls(),
/// );
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct FightEffect {
    /// First creature specification.
    pub creature1: ChooseSpec,
    /// Second creature specification.
    pub creature2: ChooseSpec,
    /// The authored clause referred to one chosen collection fighting
    /// pairwise ("those creatures fight each other").
    pub mutual_surface: bool,
}

impl FightEffect {
    /// Create a new fight effect.
    pub fn new(creature1: ChooseSpec, creature2: ChooseSpec) -> Self {
        Self {
            creature1,
            creature2,
            mutual_surface: false,
        }
    }

    pub fn with_mutual_surface(mut self) -> Self {
        self.mutual_surface = true;
        self
    }

    /// Create a fight between a creature you control and one you don't.
    pub fn you_vs_opponent() -> Self {
        Self::new(ChooseSpec::creature(), ChooseSpec::creature())
    }

    fn resolve_fighters(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(crate::ids::ObjectId, crate::ids::ObjectId), ExecutionError> {
        let legacy_operand = |spec: &ChooseSpec| {
            matches!(spec.base(), ChooseSpec::Object(filter)
            if !spec.is_target() && filter.tagged_constraints.is_empty() && !filter.source)
        };
        let legacy_flat_pair = legacy_operand(&self.creature1) && legacy_operand(&self.creature2);
        if ctx.target_assignments.is_empty()
            && legacy_flat_pair
            && let Some(fighters) = ctx.resolve_two_object_targets()
        {
            return Ok(fighters);
        }
        if ctx.target_assignments.is_empty() && legacy_flat_pair && !ctx.targets.is_empty() {
            return Err(ExecutionError::InvalidTarget);
        }

        let candidates = (
            Self::resolve_fighter_candidates(game, ctx, &self.creature1, 0),
            Self::resolve_fighter_candidates(
                game,
                ctx,
                &self.creature2,
                usize::from(self.creature1 == self.creature2),
            ),
        );
        match candidates {
            (Ok(first), Ok(second)) => {
                return Self::select_fighter_pair(
                    &first,
                    &second,
                    !self.mutual_surface
                        && !(self.creature1 == self.creature2
                            && self.creature1.count().max.is_none_or(|max| max > 1)),
                )
                .ok_or(ExecutionError::InvalidTarget);
            }
            (Err(error), _) | (_, Err(error))
                if !matches!(
                    error,
                    ExecutionError::InvalidTarget
                        | ExecutionError::TagNotFound(_)
                        | ExecutionError::ObjectNotFound(_)
                ) =>
            {
                return Err(error);
            }
            _ => {}
        }
        // CR 701.14b: if a targeted fighter is an illegal (or unchosen)
        // target, neither creature fights. Never substitute another creature.
        if self.creature1.is_target() || self.creature2.is_target() {
            return Err(ExecutionError::InvalidTarget);
        }

        let creature1 =
            crate::effects::helpers::resolve_single_object_for_effect(game, ctx, &self.creature1)?;
        let creature2 =
            crate::effects::helpers::resolve_single_object_for_effect(game, ctx, &self.creature2)?;
        Ok((creature1, creature2))
    }

    fn resolve_fighter_candidates(
        game: &GameState,
        ctx: &ExecutionContext,
        spec: &ChooseSpec,
        repeated_slot: usize,
    ) -> Result<Vec<crate::ids::ObjectId>, ExecutionError> {
        if spec.is_target() && !ctx.target_assignments.is_empty() {
            // A saved assignment is authoritative even when its surviving
            // range is empty. Refiltering all other surviving targets would
            // let a second friendly fighter substitute for the illegal first.
            let exact = ctx
                .target_assignments
                .iter()
                .filter(|a| a.spec == *spec)
                .collect::<Vec<_>>();
            let assignments = if exact.is_empty() {
                let compatible = ctx
                    .target_assignments
                    .iter()
                    .filter(|a| {
                        a.spec.base() == spec.base()
                            || crate::targeting::target_spec_matches_chooser_assignment(
                                spec, &a.spec,
                            )
                    })
                    .collect::<Vec<_>>();
                if compatible.len() > 1 {
                    return Err(ExecutionError::UnresolvableValue(
                        "fight operand has no unique saved target assignment".into(),
                    ));
                }
                compatible
            } else {
                exact
            };
            let assignment = if assignments.len() > 1 {
                assignments.get(repeated_slot).copied()
            } else {
                assignments.first().copied()
            }
            .ok_or(ExecutionError::InvalidTarget)?;
            let targets = ctx.targets.get(assignment.range.clone()).ok_or_else(|| {
                ExecutionError::UnresolvableValue(
                    "fight target assignment is outside its saved target frame".into(),
                )
            })?;
            let objects = targets
                .iter()
                .filter_map(|target| match target {
                    ResolvedTarget::Object(object) => Some(*object),
                    ResolvedTarget::Player(_) => None,
                })
                .collect::<Vec<_>>();
            return if objects.is_empty() {
                Err(ExecutionError::InvalidTarget)
            } else {
                Ok(objects)
            };
        }
        if let ChooseSpec::Object(filter) = spec.base() {
            let filter_ctx = ctx.filter_context(game);
            if spec.is_target() {
                let objects = ctx
                    .targets
                    .iter()
                    .filter_map(|target| match target {
                        ResolvedTarget::Object(id)
                            if game.object(*id).is_some_and(|object| {
                                filter.matches(object, &filter_ctx, game)
                            }) =>
                        {
                            Some(*id)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                return if objects.is_empty() {
                    Err(ExecutionError::InvalidTarget)
                } else {
                    Ok(objects)
                };
            }
            // Untargeted object references must not inherit an unrelated
            // announced target. Their own reference/filter owns the identity.
            let zone = filter.zone.unwrap_or(crate::zone::Zone::Battlefield);
            let candidates = game
                .zone_ids(zone)
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| filter.matches(object, &filter_ctx, game))
                })
                .collect::<Vec<_>>();
            return if candidates.is_empty() {
                Err(ExecutionError::InvalidTarget)
            } else {
                Ok(candidates)
            };
        }
        crate::effects::helpers::resolve_objects_from_spec(game, spec, ctx).map(|objects| {
            objects
                .into_iter()
                .filter(|id| {
                    game.object(*id)
                        .is_some_and(|object| object.zone == crate::Zone::Battlefield)
                        && !game.is_phased_out(*id)
                        && game.current_is_creature(*id)
                })
                .collect()
        })
    }

    fn select_fighter_pair(
        creature1_candidates: &[crate::ids::ObjectId],
        creature2_candidates: &[crate::ids::ObjectId],
        allow_self: bool,
    ) -> Option<(crate::ids::ObjectId, crate::ids::ObjectId)> {
        for creature1 in creature1_candidates {
            for creature2 in creature2_candidates {
                if creature1 != creature2 {
                    return Some((*creature1, *creature2));
                }
            }
        }

        if !allow_self {
            return None;
        }
        Some((
            *creature1_candidates.first()?,
            *creature2_candidates.first()?,
        ))
    }

    fn fight_events(
        game: &mut GameState,
        ctx: &ExecutionContext,
        fighters: &[(crate::ids::ObjectId, Option<ObjectSnapshot>)],
    ) -> Vec<TriggerEvent> {
        let mut seen = Vec::new();
        let mut events = Vec::new();
        let batch = game.alloc_child_event_provenance(ctx.provenance, EventKind::KeywordAction);

        for (fighter, snapshot) in fighters {
            if seen.contains(fighter) {
                continue;
            }
            seen.push(*fighter);

            let Some(controller) = game
                .object(*fighter)
                .map(|object| game.controller_of(object))
                .or_else(|| snapshot.as_ref().map(|snapshot| snapshot.controller))
            else {
                continue;
            };

            let event = KeywordActionEvent::new(KeywordActionKind::Fight, controller, *fighter, 1)
                .with_snapshot(snapshot.clone());
            let observation = game.alloc_child_event_provenance(batch, EventKind::KeywordAction);
            events.push(
                TriggerEvent::new_with_provenance(event, observation)
                    .with_simultaneous_batch(batch),
            );
        }

        events
    }
}

impl FightEffect {
    fn execute_bound_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        // Direct executor callers need the same checked frame as execute_effect
        // before any target/type/power query can use legacy infallible adapters.
        game.establish_control_transition_boundary()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        let (creature1_id, creature2_id) = match self.resolve_fighters(game, ctx) {
            Ok(fighters) => fighters,
            Err(ExecutionError::InvalidTarget) => {
                return Ok(CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::target_invalid(),
                ));
            }
            Err(err) => return Err(err),
        };
        let both_valid_fighters = [creature1_id, creature2_id].into_iter().all(|id| {
            game.object(id)
                .is_some_and(|object| object.zone == crate::zone::Zone::Battlefield)
                && !game.is_phased_out(id)
                && game.current_is_creature(id)
        });
        if !both_valid_fighters {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }

        // Use calculated power so continuous effects (pumps/shrinks) are respected.
        let power1 = game.calculated_power(creature1_id).unwrap_or(0).max(0) as u32;
        let power2 = game.calculated_power(creature2_id).unwrap_or(0).max(0) as u32;
        let creature1_snapshot = game
            .object(creature1_id)
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
        let creature2_snapshot = game
            .object(creature2_id)
            .map(|obj| ObjectSnapshot::from_object_with_calculated_characteristics(obj, game));
        let mut fight_events = Self::fight_events(
            game,
            ctx,
            &[
                (creature1_id, creature1_snapshot.clone()),
                (creature2_id, creature2_snapshot.clone()),
            ],
        );

        // CR 701.14c: self-fight is one combined damage assignment. Check
        // representation before capturing observers or changing damage state.
        let assignments = if creature1_id == creature2_id {
            vec![(
                creature1_id,
                creature1_id,
                crate::events::damage::checked_damage_amount(
                    u128::from(power1) * 2,
                    "self-fight damage",
                )?,
                creature1_snapshot,
            )]
        } else {
            vec![
                (creature1_id, creature2_id, power1, creature1_snapshot),
                (creature2_id, creature1_id, power2, creature2_snapshot),
            ]
        };
        let events = assignments
            .into_iter()
            .filter(|(_, _, amount, _)| *amount > 0)
            .map(
                |(source, target, amount, snapshot)| SimultaneousDamageEvent {
                    source,
                    target: DamageTarget::Object(target),
                    amount,
                    is_combat: false,
                    unpreventable: false,
                    cause: ctx.cause.clone(),
                    source_snapshot: snapshot,
                },
            )
            .collect::<Vec<_>>();
        let total = events.iter().map(|event| u128::from(event.amount)).sum();
        crate::events::damage::checked_damage_count(total, "fight damage outcome")?;
        crate::effects::capture_triggers_before_added_program(
            game,
            ctx,
            None,
            fight_events.iter_mut(),
        )?;
        let batch = game.alloc_child_event_provenance(ctx.provenance, EventKind::Damage);
        let mut outputs = crate::effects::damage::execute_damage_batch_with_outputs(
            game,
            ctx,
            events,
            Some(batch),
        )?;
        // Keep the existing aggregate projection and captured notification order.
        // The damage owner supplies participant routing; Fight's notifications
        // belong to the enclosing action rather than an individual assignment.
        let outcome = outputs.outcome.clone().with_events(fight_events.clone());
        outputs.retain_batch_children([CompletedEffectOutputs::aggregate_only(
            EffectOutcome::resolved().with_events(fight_events),
        )]);
        Ok(outputs.project_aggregate(outcome))
    }
}

impl EffectExecutor for FightEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(CompletedEffectOutputs::into_outcome)
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<CompletedEffectOutputs, ExecutionError> {
        let result = crate::effects::tokens::execute_resource_transaction_with_pending_value(
            game,
            ctx,
            || CompletedEffectOutputs::aggregate_only(EffectOutcome::with_objects(Vec::new())),
            |game, ctx| self.execute_bound_with_outputs(game, ctx),
        );
        if ctx.decision_maker.awaiting_choice() && result.is_ok() {
            return Ok(CompletedEffectOutputs::aggregate_only(
                EffectOutcome::count(0),
            ));
        }
        result
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.creature1)
    }

    fn target_description(&self) -> &'static str {
        "creature to fight"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::ContinuousEffect;
    use crate::effect::Until;
    use crate::effects::execute_effect;
    use crate::events::cause::CauseFilter;
    use crate::events::counters::matchers::WouldPutCountersMatcher;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::replacement::{EventModification, ReplacementAction, ReplacementEffect};
    use crate::static_abilities::StaticAbility;
    use crate::tag::TagKey;
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::types::CardType;
    use crate::zone::Zone;
    use std::collections::HashMap;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(
        card_id: u32,
        name: &str,
        power: i32,
        toughness: i32,
    ) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(power, toughness))
            .build()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        power: i32,
        toughness: i32,
        controller: PlayerId,
    ) -> ObjectId {
        let id = game.new_object_id();
        let card = make_creature_card(id.0 as u32, name, power, toughness);
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn add_static_ability(game: &mut GameState, object: ObjectId, ability: StaticAbility) {
        let obj = game.object_mut(object).expect("object exists");
        obj.abilities_mut().push(Ability::static_ability(ability));
    }

    fn add_doubling_season_like_effect(
        game: &mut GameState,
        controller: PlayerId,
        target: ObjectId,
    ) {
        let source = game.new_object_id();
        game.effect_store.replacement_effects.add_resolution_effect(
            ReplacementEffect::with_matcher(
                source,
                controller,
                WouldPutCountersMatcher::new(
                    ObjectFilter::specific(target),
                    Some(CounterType::MinusOneMinusOne),
                )
                .with_cause_filter(CauseFilter::from_effect()),
                ReplacementAction::Modify(EventModification::Multiply(2)),
            ),
        );
    }

    #[test]
    fn test_fight_basic() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let bear = create_creature(&mut game, "Grizzly Bears", 2, 2, alice);
        let goblin = create_creature(&mut game, "Goblin Piker", 2, 1, bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(bear),
            ResolvedTarget::Object(goblin),
        ]);

        let effect = FightEffect::you_vs_opponent();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(4));

        // Bear (2/2) takes 2 damage from Goblin
        assert_eq!(game.damage_on(bear), 2);

        // Goblin (2/1) takes 2 damage from Bear (lethal)
        assert_eq!(game.damage_on(goblin), 2);
    }

    #[test]
    fn test_fight_asymmetric_power() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let big = create_creature(&mut game, "Big Creature", 5, 5, alice);
        let small = create_creature(&mut game, "Small Creature", 1, 1, bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(big),
            ResolvedTarget::Object(small),
        ]);

        let effect = FightEffect::you_vs_opponent();
        effect.execute(&mut game, &mut ctx).unwrap();

        // Big takes 1 damage
        assert_eq!(game.damage_on(big), 1);
        // Small takes 5 damage
        assert_eq!(game.damage_on(small), 5);
    }

    #[test]
    fn test_fight_zero_power() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let wall = create_creature(&mut game, "Wall", 0, 4, alice);
        let attacker = create_creature(&mut game, "Attacker", 3, 3, bob);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(wall),
            ResolvedTarget::Object(attacker),
        ]);

        let effect = FightEffect::you_vs_opponent();
        effect.execute(&mut game, &mut ctx).unwrap();

        // Wall deals 0 damage (0 power)
        assert_eq!(game.damage_on(attacker), 0);
        // Attacker deals 3 damage to wall
        assert_eq!(game.damage_on(wall), 3);
    }

    #[test]
    fn test_fight_insufficient_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let bear = create_creature(&mut game, "Bear", 2, 2, alice);
        let source = game.new_object_id();

        // Only one target
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(bear)]);

        let effect = FightEffect::you_vs_opponent();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_fight_no_targets() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);

        let effect = FightEffect::you_vs_opponent();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
    }

    #[test]
    fn test_fight_clone_box() {
        let effect = FightEffect::you_vs_opponent();
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("FightEffect"));
    }

    #[test]
    fn test_fight_uses_calculated_power_with_continuous_effects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let bear = create_creature(&mut game, "Bear", 2, 2, alice);
        let ogre = create_creature(&mut game, "Ogre", 2, 2, bob);
        let source = game.new_object_id();

        // +2/+0 pump should increase fight damage dealt by Bear.
        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::pump(
                source,
                alice,
                bear,
                2,
                0,
                Until::EndOfTurn,
            ));

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(bear),
            ResolvedTarget::Object(ogre),
        ]);

        let effect = FightEffect::you_vs_opponent();
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.damage_on(ogre), 4);
        assert_eq!(game.damage_on(bear), 2);
    }

    #[test]
    fn test_fight_between_two_objects_from_same_tagged_target_group() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let mammoth = create_creature(&mut game, "Mammoth Spider", 3, 5, alice);
        let hurda = create_creature(&mut game, "Caravan Hurda", 1, 5, alice);
        add_static_ability(&mut game, hurda, StaticAbility::lifelink());
        let source = game.new_object_id();

        let tag = TagKey::from("targeted");
        let tagged_snapshots = vec![
            ObjectSnapshot::from_object(game.object(mammoth).expect("mammoth"), &game),
            ObjectSnapshot::from_object(game.object(hurda).expect("hurda"), &game),
        ];
        let mut tagged_filter = ObjectFilter::creature();
        tagged_filter
            .tagged_constraints
            .push(TaggedObjectConstraint {
                tag: tag.clone(),
                relation: TaggedOpbjectRelation::IsTaggedObject,
            });

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_tagged_objects(HashMap::from([(tag.clone(), tagged_snapshots)]));
        let effect = FightEffect::new(ChooseSpec::Object(tagged_filter), ChooseSpec::Tagged(tag));
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(game.damage_on(mammoth), 1);
        assert_eq!(game.damage_on(hurda), 3);
        assert_eq!(
            game.player(alice).expect("player").life,
            21,
            "lifelink creature in the tagged pair should deal fight damage"
        );
    }

    #[test]
    fn test_fight_uses_fighters_as_damage_sources_and_effect_counters_can_double() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let infector = create_creature(&mut game, "Infector", 2, 2, alice);
        add_static_ability(&mut game, infector, StaticAbility::infect());

        let blocker = create_creature(&mut game, "Blocker", 3, 3, bob);
        add_doubling_season_like_effect(&mut game, bob, blocker);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(infector),
            ResolvedTarget::Object(blocker),
        ]);

        let effect = FightEffect::you_vs_opponent();
        effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(
            game.counter_count(blocker, CounterType::MinusOneMinusOne),
            4
        );
        assert_eq!(game.damage_on(blocker), 0);
        assert_eq!(game.damage_on(infector), 3);
    }

    #[test]
    fn test_fight_does_nothing_if_one_fighter_left_battlefield() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let bear = create_creature(&mut game, "Bear", 3, 3, alice);
        let goblin = create_creature(&mut game, "Goblin", 2, 2, bob);
        let source = game.new_object_id();
        let goblin_graveyard_id = game
            .move_object_by_effect(goblin, Zone::Graveyard)
            .expect("goblin should move to graveyard");

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(bear),
            ResolvedTarget::Object(goblin_graveyard_id),
        ]);

        let effect = FightEffect::you_vs_opponent();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.damage_on(bear), 0);
        assert_eq!(game.damage_on(goblin_graveyard_id), 0);
    }

    #[test]
    fn test_fight_does_nothing_if_one_fighter_is_no_longer_a_creature() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let bear = create_creature(&mut game, "Bear", 3, 3, alice);
        let mimic = create_creature(&mut game, "Mimic", 2, 2, bob);
        let source = game.new_object_id();
        game.object_mut(mimic)
            .expect("mimic should exist")
            .card_types = vec![CardType::Artifact].into();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(bear),
            ResolvedTarget::Object(mimic),
        ]);

        let effect = FightEffect::you_vs_opponent();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(0));
        assert_eq!(game.damage_on(bear), 0);
        assert_eq!(game.damage_on(mimic), 0);
    }

    #[test]
    fn test_fight_with_itself_deals_double_damage_to_itself() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let fighter = create_creature(&mut game, "Solo Fighter", 3, 3, alice);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice).with_targets(vec![
            ResolvedTarget::Object(fighter),
            ResolvedTarget::Object(fighter),
        ]);

        let effect = FightEffect::you_vs_opponent();
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(6));
        assert_eq!(game.damage_on(fighter), 6);
    }

    #[test]
    fn saved_empty_fight_assignments_never_borrow_a_surviving_slot() {
        for same_specs in [false, true] {
            for missing in [0, 1] {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                let first = create_creature(&mut game, "First", 3, 50, alice);
                let second = create_creature(&mut game, "Second", 5, 50, alice);
                let spec1 =
                    ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
                let spec2 = if same_specs {
                    spec1.clone()
                } else {
                    ChooseSpec::target(ChooseSpec::creature())
                };
                let survivor = if missing == 0 { second } else { first };
                let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
                    .with_targets(vec![ResolvedTarget::Object(survivor)])
                    .with_target_assignments(vec![
                        crate::game_state::TargetAssignment {
                            spec: spec1.clone(),
                            range: if missing == 0 { 0..0 } else { 0..1 },
                        },
                        crate::game_state::TargetAssignment {
                            spec: spec2.clone(),
                            range: if missing == 0 { 0..1 } else { 1..1 },
                        },
                    ]);
                let result = FightEffect::new(spec1, spec2)
                    .execute(&mut game, &mut ctx)
                    .unwrap();
                assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
                assert!(result.events.is_empty());
                assert_eq!(game.damage_on(first), 0);
                assert_eq!(game.damage_on(second), 0);
            }
        }
    }

    #[test]
    fn mutual_tagged_pair_with_one_remaining_member_does_not_fight_itself() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature(&mut game, "First", 3, 50, alice);
        let second = create_creature(&mut game, "Second", 5, 50, alice);
        let captured = [first, second]
            .into_iter()
            .map(|id| {
                ObjectSnapshot::from_object_with_calculated_characteristics(
                    game.object(id).unwrap(),
                    &game,
                )
            })
            .collect();
        game.move_object_by_effect(second, Zone::Exile).unwrap();
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        ctx.set_tagged_objects("pair", captured);
        let result = FightEffect::new(
            ChooseSpec::Tagged("pair".into()),
            ChooseSpec::Tagged("pair".into()),
        )
        .with_mutual_surface()
        .execute(&mut game, &mut ctx)
        .unwrap();
        assert_eq!(result.status, crate::effect::OutcomeStatus::TargetInvalid);
        assert!(result.events.is_empty());
        assert_eq!(game.damage_on(first), 0);
    }

    #[test]
    fn self_fight_overflow_is_an_incomplete_error_without_damage_or_keyword_receipts() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let fighter = create_creature(&mut game, "Large fighter", i32::MAX, i32::MAX, alice);
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice);
        game.effect_store
            .replacement_effects
            .add_one_shot_effect(ReplacementEffect::with_matcher(
                fighter,
                alice,
                crate::events::damage::matchers::DamageToObjectMatcher::new(
                    ObjectFilter::specific(fighter),
                ),
                ReplacementAction::Modify(EventModification::Multiply(3)),
            ));
        let history_before = game.turn_store.turn_history.event_records.len();
        let result = FightEffect::new(
            ChooseSpec::SpecificObject(fighter),
            ChooseSpec::SpecificObject(fighter),
        )
        .execute(&mut game, &mut ctx);
        assert!(matches!(
            result,
            Err(ExecutionError::ResourceLimitExceeded { .. })
        ));
        assert_eq!(game.damage_on(fighter), 0);
        assert_eq!(
            game.turn_store.turn_history.event_records.len(),
            history_before
        );
        assert!(!game.effect_store.has_pending_trigger_work());
    }

    #[test]
    fn pending_damage_order_rolls_back_the_entire_fight_then_resumes_one_complete_batch() {
        #[derive(Default)]
        struct Pause {
            pause: bool,
            pending: bool,
            calls: usize,
        }
        impl crate::decision::DecisionMaker for Pause {
            fn decide_options(
                &mut self,
                _game: &GameState,
                options: &crate::decisions::context::SelectOptionsContext,
            ) -> Vec<usize> {
                self.calls += 1;
                self.pending = self.pause;
                if self.pause {
                    vec![]
                } else {
                    options
                        .options
                        .iter()
                        .filter(|option| option.legal)
                        .take(1)
                        .map(|option| option.index)
                        .collect()
                }
            }
            fn awaiting_choice(&self) -> bool {
                self.pending
            }
        }
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let first = create_creature(&mut game, "First", 3, 50, alice);
        let second = create_creature(&mut game, "Second", 4, 50, bob);
        for (source, modification) in [
            (first, EventModification::Multiply(2)),
            (second, EventModification::Add(1)),
        ] {
            game.effect_store.replacement_effects.add_one_shot_effect(
                ReplacementEffect::with_matcher(
                    source,
                    alice,
                    crate::events::damage::matchers::DamageToObjectMatcher::new(
                        ObjectFilter::specific(second),
                    ),
                    ReplacementAction::Modify(modification),
                ),
            );
        }
        let source = game.new_object_id();
        let effect = FightEffect::new(
            ChooseSpec::SpecificObject(first),
            ChooseSpec::SpecificObject(second),
        );
        let before = game.turn_store.turn_history.event_records.len();
        let mut dm = Pause {
            pause: true,
            ..Default::default()
        };
        let outcome = effect
            .execute(
                &mut game,
                &mut ExecutionContext::new(source, alice, &mut dm),
            )
            .unwrap();
        assert!(dm.pending && dm.calls > 0);
        assert!(outcome.events.is_empty());
        assert_eq!(game.damage_on(first), 0);
        assert_eq!(game.damage_on(second), 0);
        assert_eq!(game.turn_store.turn_history.event_records.len(), before);
        assert!(!game.effect_store.has_pending_trigger_work());
        dm.pause = false;
        dm.pending = false;
        let outcome = effect
            .execute(
                &mut game,
                &mut ExecutionContext::new(source, alice, &mut dm),
            )
            .unwrap();
        assert_eq!(game.damage_on(first), 4);
        assert!(matches!(game.damage_on(second), 7 | 8));
        let damage = outcome
            .events
            .iter()
            .filter(|event| event.downcast::<crate::events::DamageEvent>().is_some())
            .collect::<Vec<_>>();
        assert_eq!(damage.len(), 2);
        assert_eq!(
            damage[0].simultaneous_batch(),
            damage[1].simultaneous_batch()
        );
        let fights = outcome
            .events
            .iter()
            .filter(|event| {
                event
                    .downcast::<KeywordActionEvent>()
                    .is_some_and(|event| event.action == KeywordActionKind::Fight)
            })
            .collect::<Vec<_>>();
        assert_eq!(fights.len(), 2);
        assert!(fights[0].simultaneous_batch().is_some());
        assert_eq!(
            fights[0].simultaneous_batch(),
            fights[1].simultaneous_batch()
        );
        assert_ne!(fights[0].provenance(), fights[1].provenance());
    }

    #[test]
    fn an_unchosen_optional_second_fighter_does_not_reuse_the_required_first() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let first = create_creature(&mut game, "Required fighter", 3, 50, alice);
        let spec1 = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().you_control()));
        let spec2 = ChooseSpec::target(ChooseSpec::Object(ObjectFilter::creature().other()))
            .with_count(crate::effect::ChoiceCount::up_to(1));
        let mut ctx = ExecutionContext::new_default(game.new_object_id(), alice)
            .with_targets(vec![ResolvedTarget::Object(first)])
            .with_target_assignments(vec![
                crate::game_state::TargetAssignment {
                    spec: spec1.clone(),
                    range: 0..1,
                },
                crate::game_state::TargetAssignment {
                    spec: spec2.clone(),
                    range: 1..1,
                },
            ]);
        let outcome = FightEffect::new(spec1, spec2)
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert_eq!(outcome.status, crate::effect::OutcomeStatus::TargetInvalid);
        assert!(outcome.events.is_empty());
        assert_eq!(game.damage_on(first), 0);
    }

    #[test]
    fn direct_and_generic_fight_discovery_failure_is_typed_and_rolls_back_before_characteristic_reads()
     {
        #[derive(Debug, Clone)]
        struct UnboundedFighter(std::sync::Arc<std::sync::atomic::AtomicUsize>);
        impl crate::static_abilities::StaticAbilityKind for UnboundedFighter {
            fn id(&self) -> crate::static_abilities::StaticAbilityId {
                crate::static_abilities::StaticAbilityId::GrantObjectAbilityForFilter
            }
            fn display(&self) -> String {
                "Unbounded fight characteristic fixture".into()
            }
            fn generate_effects(
                &self,
                source: ObjectId,
                controller: PlayerId,
                game: &GameState,
            ) -> Vec<ContinuousEffect> {
                assert!(
                    self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst) < 32_768,
                    "fixture work ceiling"
                );
                let crate::ability::AbilityKind::Static(parent) =
                    &game.object(source).unwrap().abilities[0].kind
                else {
                    panic!("fixture parent");
                };
                vec![
                    ContinuousEffect::new(
                        source,
                        controller,
                        crate::continuous::EffectTarget::Source,
                        crate::continuous::Modification::AddAbility(parent.clone()),
                    ),
                    ContinuousEffect::new(
                        source,
                        controller,
                        crate::continuous::EffectTarget::Source,
                        crate::continuous::Modification::ModifyPower(1),
                    ),
                ]
            }
        }
        std::thread::Builder::new().stack_size(128 * 1024 * 1024).spawn(|| {
            for generic in [false, true] {
                let mut game = setup_game();
                let alice = PlayerId::from_index(0);
                let first = create_creature(&mut game, "Checked first", 3, 50, alice);
                let second = create_creature(&mut game, "Checked second", 4, 50, alice);
                add_static_ability(&mut game, first, StaticAbility::new(UnboundedFighter(Default::default())));
                let source = game.new_object_id();
                let fight = FightEffect::new(ChooseSpec::SpecificObject(first), ChooseSpec::SpecificObject(second));
                let before = game.provenance_graph().node_count();
                let history = game.turn_store.turn_history.event_records.len();
                let mut ctx = ExecutionContext::new_default(source, alice);
                for _ in 0..2 {
                    let result = if generic { execute_effect(&mut game, &Effect::new(fight.clone()), &mut ctx) }
                        else { fight.execute(&mut game, &mut ctx) };
                    assert!(matches!(result, Err(ExecutionError::ContinuousDiscovery(
                        crate::static_ability_processor::StaticEffectDiscoveryError::RoundLimit { .. }))), "{result:?}");
                    assert_eq!(ctx.source, source);
                    assert!(ctx.targets.is_empty() && ctx.target_assignments.is_empty());
                    assert_eq!(game.damage_on(first), 0);
                    assert_eq!(game.damage_on(second), 0);
                    assert_eq!(game.provenance_graph().node_count(), before);
                    assert_eq!(game.turn_store.turn_history.event_records.len(), history);
                    assert!(!game.effect_store.has_pending_trigger_work());
                }
                game.object_mut(first).unwrap().abilities_mut().clear();
                fight.execute(&mut game, &mut ctx).unwrap();
                assert_eq!(game.damage_on(first), 4);
                assert_eq!(game.damage_on(second), 3);
            }
        }).unwrap().join().unwrap();
    }
}
