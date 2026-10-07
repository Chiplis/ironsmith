//! Copy a stack object once for each matching legal target.

use std::collections::HashSet;

use crate::effect::EffectOutcome;
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter};
use crate::effects::stack::copy_spell::{create_stack_copy, stack_entry_for_copy_target};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::spells::{BecomesTargetedEvent, SpellCopiedEvent};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::effects::stack::retarget_stack_object::stack_entry_retarget_requirements;
use crate::game_state::{GameState, Target};
use crate::target::ChooseSpec;
use crate::triggers::TriggerEvent;

pub type CopySpellForEachTargetEffect = ironsmith_core::CopySpellForEachTargetEffect;

fn candidate_matches(
    effect: &CopySpellForEachTargetEffect,
    target: Target,
    game: &GameState,
    ctx: &ExecutionContext,
) -> bool {
    let filter_ctx = ctx.filter_context(game);
    match target {
        Target::Object(object_id) => {
            let Some(filter) = &effect.object_filter else {
                return effect.player_filter.is_none();
            };
            game.object(object_id)
                .is_some_and(|object| filter.matches(object, &filter_ctx, game))
        }
        Target::Player(player_id) => {
            let Some(filter) = &effect.player_filter else {
                return effect.object_filter.is_none();
            };
            filter.matches_player(player_id, &filter_ctx)
        }
    }
}

impl crate::effects::EffectExecutor for CopySpellForEachTargetEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let target_id = *resolve_objects_for_effect(game, ctx, &self.target)?
            .first()
            .ok_or(ExecutionError::InvalidTarget)?;
        let Some(original_entry) = stack_entry_for_copy_target(
            game,
            target_id,
            ctx,
            super::counter::counter_target_stack_kind(&self.target),
        )?
        else {
            return Ok(EffectOutcome::target_invalid());
        };
        // An ability named by its own stack id is copied from its source.
        let target_id = if game.object(target_id).is_none() && original_entry.is_ability {
            original_entry.object_id
        } else {
            target_id
        };
        let copier = resolve_player_filter(game, &self.copier, ctx)?;

        // CR 707.10d: the target slots come from the entry's announced
        // assignments (the chosen modes' targets for a modal spell, CR 700.2g),
        // and legality is judged for the copies' controller.
        let mut probe_entry = original_entry.clone();
        probe_entry.controller = copier;
        let Some(slots) = stack_entry_retarget_requirements(game, &probe_entry, false)? else {
            return Ok(EffectOutcome::resolved());
        };
        let slots: Vec<_> = slots
            .into_iter()
            .filter(|slot| !slot.range.is_empty())
            .collect();
        if slots.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        // One object can't be chosen twice for a single "target" (CR 115.3),
        // nor for both a target and "another target", so a copy whose targets
        // must all be one object can't exist then.
        if slots.iter().any(|slot| slot.range.len() > 1)
            || slots.iter().any(|slot| slot.excludes_prior_object_targets)
        {
            return Ok(EffectOutcome::resolved());
        }

        let mut created_ids = Vec::new();
        let mut events = Vec::new();
        let mut seen = HashSet::new();

        for candidate in slots
            .iter()
            .flat_map(|slot| slot.requirement.legal_targets.iter())
        {
            if !seen.insert(*candidate) {
                continue;
            }
            if self.exclude_current_targets && original_entry.targets.contains(candidate) {
                continue;
            }
            if !candidate_matches(self, *candidate, game, ctx) {
                continue;
            }
            // CR 707.10d: every target of the copy is that player or object,
            // so it must be legal for each instance of "target".
            if !slots
                .iter()
                .all(|slot| slot.requirement.legal_targets.contains(candidate))
            {
                continue;
            }

            let mut targets = original_entry.targets.clone();
            for slot in &slots {
                for index in slot.range.clone() {
                    if let Some(target) = targets.get_mut(index) {
                        *target = *candidate;
                    }
                }
            }
            let Some(copy_id) = create_stack_copy(
                game,
                target_id,
                &original_entry,
                copier,
                &self.removed_supertypes,
                Some(targets),
            )? else { continue; };
            created_ids.push(copy_id);

            if !original_entry.is_ability {
                events.push(TriggerEvent::new_with_provenance(SpellCopiedEvent::new(copy_id, copier), ctx.provenance));
            }
            if let Some(entry) = game.stack.iter().find(|entry| entry.object_id == copy_id) {
                let mut seen = Vec::new();
                for target in &entry.targets {
                    if seen.contains(target) { continue; }
                    seen.push(*target);
                    events.push(TriggerEvent::new_with_provenance(BecomesTargetedEvent::from_stack_entry(*target, entry).with_participant_snapshots(game), ctx.provenance));
                }
            }
        }

        Ok(EffectOutcome::with_objects(created_ids).with_events(events))
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        Some(&self.target)
    }

    fn target_description(&self) -> &'static str {
        "stack object to copy"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::Effect;
    use crate::effects::EffectExecutor;
    use crate::game_state::StackEntry;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        artifact: bool,
    ) -> crate::ids::ObjectId {
        let mut types = vec![CardType::Creature];
        if artifact {
            types.push(CardType::Artifact);
        }
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(types)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn stack_nonartifact_creature_spell(
        game: &mut GameState,
        controller: PlayerId,
        target: crate::ids::ObjectId,
    ) -> crate::ids::ObjectId {
        let target_spec = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature()
                .without_type(CardType::Artifact)
                .controlled_by(PlayerFilter::You),
        ));
        let card = CardBuilder::new(CardId::new(), "Friendly Calibration")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&card, controller, Zone::Stack);
        game.object_mut(spell_id)
            .expect("spell object should exist")
            .spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::TargetOnlyEffect::new(target_spec.clone()),
            )])
            .into(),
        );
        game.push_to_stack(
            StackEntry::new(spell_id, controller).with_targets(vec![Target::Object(target)]),
        );
        spell_id
    }

    fn stack_player_spell(
        game: &mut GameState,
        controller: PlayerId,
        target: PlayerId,
    ) -> crate::ids::ObjectId {
        let target_spec = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
        let card = CardBuilder::new(CardId::new(), "Friendly Ping")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Red]]))
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&card, controller, Zone::Stack);
        game.object_mut(spell_id)
            .expect("spell object should exist")
            .spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::TargetOnlyEffect::new(target_spec),
            )])
            .into(),
        );
        game.push_to_stack(
            StackEntry::new(spell_id, controller).with_targets(vec![Target::Player(target)]),
        );
        spell_id
    }

    fn stack_creature_and_player_spell(
        game: &mut GameState,
        controller: PlayerId,
        creature_target: crate::ids::ObjectId,
        player_target: PlayerId,
    ) -> crate::ids::ObjectId {
        let creature_spec = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
        ));
        let player_spec = ChooseSpec::target(ChooseSpec::Player(PlayerFilter::Any));
        let card = CardBuilder::new(CardId::new(), "Friendly Coordination")
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Blue]]))
            .card_types(vec![CardType::Instant])
            .build();
        let spell_id = game.create_object_from_card(&card, controller, Zone::Stack);
        game.object_mut(spell_id)
            .expect("spell object should exist")
            .spell_effect = Some(
            crate::resolution::ResolutionProgram::from_effects(vec![
                Effect::new(crate::effects::TargetOnlyEffect::new(creature_spec)),
                Effect::new(crate::effects::TargetOnlyEffect::new(player_spec)),
            ])
            .into(),
        );
        game.push_to_stack(StackEntry::new(spell_id, controller).with_targets(vec![
            Target::Object(creature_target),
            Target::Player(player_target),
        ]));
        spell_id
    }

    #[test]
    fn copies_stack_object_once_for_each_matching_other_legal_object_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let original = create_creature(&mut game, "Original", alice, false);
        let ally = create_creature(&mut game, "Ally", alice, false);
        let second_ally = create_creature(&mut game, "Second Ally", alice, false);
        let artifact_ally = create_creature(&mut game, "Artifact Ally", alice, true);
        let bob_creature = create_creature(&mut game, "Bob Creature", bob, false);
        let spell_id = stack_nonartifact_creature_spell(&mut game, alice, original);

        let effect = CopySpellForEachTargetEffect::new(ChooseSpec::SpecificObject(spell_id))
            .with_object_filter(ObjectFilter::creature().controlled_by(PlayerFilter::You))
            .exclude_current_targets(true);
        let mut ctx = ExecutionContext::new_default(spell_id, alice);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        let created = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids,
            other => panic!("expected copied object ids, got {other:?}"),
        };
        assert_eq!(
            created.len(),
            2,
            "only legal same-controller nonartifact allies should copy"
        );

        let original_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == spell_id)
            .expect("original spell should remain on stack");
        assert_eq!(original_entry.targets, vec![Target::Object(original)]);

        let copy_targets: HashSet<Target> = created
            .iter()
            .map(|copy_id| {
                let entry = game
                    .stack
                    .iter()
                    .find(|entry| entry.object_id == *copy_id)
                    .expect("copy should have a stack entry");
                assert_eq!(entry.x_value, original_entry.x_value);
                assert_eq!(
                    entry.optional_costs_paid,
                    original_entry.optional_costs_paid
                );
                entry.targets[0]
            })
            .collect();
        assert_eq!(
            copy_targets,
            HashSet::from([Target::Object(ally), Target::Object(second_ally)])
        );
        assert!(!copy_targets.contains(&Target::Object(original)));
        assert!(!copy_targets.contains(&Target::Object(artifact_ally)));
        assert!(!copy_targets.contains(&Target::Object(bob_creature)));
    }

    #[test]
    fn copies_ability_stack_entry_for_each_matching_other_legal_object_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let ability_source = create_creature(&mut game, "Ability Source", alice, false);
        let ally = create_creature(&mut game, "Ally", alice, false);
        let second_ally = create_creature(&mut game, "Second Ally", alice, false);

        let target_spec = ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature().controlled_by(PlayerFilter::You),
        ));
        let ability_effects =
            crate::resolution::ResolutionProgram::from_effects(vec![Effect::new(
                crate::effects::TargetOnlyEffect::new(target_spec),
            )]);
        game.push_to_stack(
            StackEntry::ability(ability_source, alice, ability_effects)
                .with_targets(vec![Target::Object(ability_source)]),
        );

        let effect = CopySpellForEachTargetEffect::new(ChooseSpec::SpecificObject(ability_source))
            .with_object_filter(ObjectFilter::creature().controlled_by(PlayerFilter::You))
            .exclude_current_targets(true);
        let mut ctx = ExecutionContext::new_default(ability_source, alice);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        let created = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids,
            other => panic!("expected copied ability object ids, got {other:?}"),
        };
        assert_eq!(created.len(), 2);

        let copy_targets: HashSet<Target> = created
            .iter()
            .map(|copy_id| {
                let entry = game
                    .stack
                    .iter()
                    .find(|entry| entry.object_id == *copy_id)
                    .expect("copy should have a stack entry");
                assert!(
                    entry.is_ability,
                    "ability copies should remain ability entries"
                );
                entry.targets[0]
            })
            .collect();
        assert_eq!(
            copy_targets,
            HashSet::from([Target::Object(ally), Target::Object(second_ally)])
        );
        assert!(!copy_targets.contains(&Target::Object(ability_source)));
    }

    #[test]
    fn copies_stack_object_once_for_each_matching_other_legal_player_target() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let spell_id = stack_player_spell(&mut game, alice, bob);

        let effect = CopySpellForEachTargetEffect::new(ChooseSpec::SpecificObject(spell_id))
            .with_player_filter(PlayerFilter::Any)
            .exclude_current_targets(true);
        let mut ctx = ExecutionContext::new_default(spell_id, alice);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        let created = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids,
            other => panic!("expected copied object ids, got {other:?}"),
        };
        assert_eq!(created.len(), 1);

        let copy_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == created[0])
            .expect("copy should have a stack entry");
        assert_eq!(copy_entry.targets, vec![Target::Player(alice)]);

        let original_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == spell_id)
            .expect("original spell should remain on stack");
        assert_eq!(original_entry.targets, vec![Target::Player(bob)]);
    }

    #[test]
    fn creates_no_copy_when_no_matching_other_legal_target_exists() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let original = create_creature(&mut game, "Original", alice, false);
        let artifact_ally = create_creature(&mut game, "Artifact Ally", alice, true);
        let spell_id = stack_nonartifact_creature_spell(&mut game, alice, original);

        let effect = CopySpellForEachTargetEffect::new(ChooseSpec::SpecificObject(spell_id))
            .with_object_filter(ObjectFilter::creature().controlled_by(PlayerFilter::You))
            .exclude_current_targets(true);
        let mut ctx = ExecutionContext::new_default(spell_id, alice);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        let created = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids,
            other => panic!("expected copied object ids, got {other:?}"),
        };
        assert!(created.is_empty());
        assert_eq!(game.stack.len(), 1);
        assert!(game.object(artifact_ally).is_some());
    }

    #[test]
    fn creates_no_copy_when_candidate_is_illegal_for_another_target_slot() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let original = create_creature(&mut game, "Original", alice, false);
        let _ally = create_creature(&mut game, "Ally", alice, false);
        let spell_id = stack_creature_and_player_spell(&mut game, alice, original, bob);

        let effect = CopySpellForEachTargetEffect::new(ChooseSpec::SpecificObject(spell_id))
            .with_object_filter(ObjectFilter::creature().controlled_by(PlayerFilter::You))
            .exclude_current_targets(true);
        let mut ctx = ExecutionContext::new_default(spell_id, alice);
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        let created = match outcome.value {
            crate::effect::OutcomeValue::Objects(ids) => ids,
            other => panic!("expected copied object ids, got {other:?}"),
        };
        // CR 707.10d: every target of the copy must be the candidate, and a
        // creature isn't a legal target for the "target player" slot.
        assert!(created.is_empty());
    }
}
