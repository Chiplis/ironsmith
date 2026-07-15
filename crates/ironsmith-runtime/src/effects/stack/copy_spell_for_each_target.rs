//! Copy a stack object once for each matching legal target.

use std::collections::HashSet;

use crate::decisions::context::TargetRequirementContext;
use crate::effect::{ChoiceCount, EffectOutcome};
use crate::effects::helpers::{resolve_objects_for_effect, resolve_player_filter};
use crate::effects::stack::copy_spell::{create_stack_copy, stack_entry_for_copy_target};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::spells::{BecomesTargetedEvent, SpellCopiedEvent};
use crate::filter::{ObjectFilterExt as _, PlayerFilterExt as _};
use crate::game_state::{GameState, StackEntry, Target};
use crate::target::ChooseSpec;
use crate::targeting::{assigned_target_ranges, compute_legal_targets_with_tagged_objects};
use crate::triggers::TriggerEvent;

pub type CopySpellForEachTargetEffect = ironsmith_core::CopySpellForEachTargetEffect;

fn requires_target_selection(spec: &ChooseSpec) -> bool {
    match spec {
        ChooseSpec::Target(_) => true,
        ChooseSpec::AnyTarget
        | ChooseSpec::AnyOtherTarget
        | ChooseSpec::Player(_)
        | ChooseSpec::Object(_)
        | ChooseSpec::PlayerOrPlaneswalker(_) => true,
        ChooseSpec::WithCount(inner, _) | ChooseSpec::WithCountValue(inner, _, _) => {
            requires_target_selection(inner)
        }
        _ => false,
    }
}

fn effects_for_stack_entry(game: &GameState, entry: &StackEntry) -> Vec<crate::effect::Effect> {
    if let Some(ref effects) = entry.ability_effects {
        return effects.to_vec();
    }

    game.object(entry.object_id)
        .and_then(|object| object.spell_effect_owned())
        .map(|effects| effects.to_vec())
        .unwrap_or_default()
}

fn extract_requirements(
    game: &GameState,
    entry: &StackEntry,
) -> Option<Vec<TargetRequirementContext>> {
    let effects = effects_for_stack_entry(game, entry);
    let mut requirements = Vec::new();

    for effect in &effects {
        let Some(spec) = effect.0.get_target_spec() else {
            continue;
        };
        if !requires_target_selection(spec) {
            continue;
        }

        let count: ChoiceCount = effect.0.get_target_count().unwrap_or_default();
        let legal_targets = compute_legal_targets_with_tagged_objects(
            game,
            spec,
            entry.controller,
            Some(entry.object_id),
            if entry.tagged_objects.is_empty() {
                None
            } else {
                Some(&entry.tagged_objects)
            },
        );
        let legal_target_sets =
            crate::targeting::legal_target_sets_for_spec(game, spec, &legal_targets);
        let has_enough = crate::targeting::has_enough_legal_targets_for_spec(
            game,
            spec,
            &legal_targets,
            count.min,
        );
        if !has_enough {
            return None;
        }

        requirements.push(TargetRequirementContext {
            description: effect.0.target_description().to_string(),
            legal_targets,
            legal_target_sets,
            min_targets: count.min,
            max_targets: count.max,
            distinct_player_group: None,
        });
    }

    Some(requirements)
}

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

fn copy_targets_for_candidate(
    original_targets: &[Target],
    replace_idx: usize,
    candidate: Target,
) -> Vec<Target> {
    let mut targets = original_targets.to_vec();
    if let Some(target) = targets.get_mut(replace_idx) {
        *target = candidate;
    }
    targets
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
        let Some(original_entry) = stack_entry_for_copy_target(game, target_id, ctx)? else {
            return Ok(EffectOutcome::target_invalid());
        };
        let Some(requirements) = extract_requirements(game, &original_entry) else {
            return Ok(EffectOutcome::resolved());
        };
        if requirements.is_empty() {
            return Ok(EffectOutcome::resolved());
        }
        let Some(ranges) = assigned_target_ranges(&requirements, &original_entry.targets) else {
            return Ok(EffectOutcome::resolved());
        };

        let copier = resolve_player_filter(game, &self.copier, ctx)?;
        let mut created_ids = Vec::new();
        let mut events = Vec::new();
        let mut seen = HashSet::new();

        for (requirement, range) in requirements.iter().zip(ranges.iter()) {
            let Some(replace_idx) = range.clone().next() else {
                continue;
            };

            for candidate in &requirement.legal_targets {
                if self.exclude_current_targets && original_entry.targets.contains(candidate) {
                    continue;
                }
                if !candidate_matches(self, *candidate, game, ctx) {
                    continue;
                }
                if !seen.insert(*candidate) {
                    continue;
                }

                let targets =
                    copy_targets_for_candidate(&original_entry.targets, replace_idx, *candidate);
                let copy_id = create_stack_copy(
                    game,
                    target_id,
                    &original_entry,
                    copier,
                    &self.removed_supertypes,
                    Some(targets),
                )?;
                created_ids.push(copy_id);

                events.push(TriggerEvent::new_with_provenance(
                    SpellCopiedEvent::new(copy_id, copier),
                    ctx.provenance,
                ));
                if let Target::Object(target_id) = candidate {
                    events.push(TriggerEvent::new_with_provenance(
                        BecomesTargetedEvent::new(
                            *target_id,
                            copy_id,
                            original_entry.controller,
                            original_entry.is_ability,
                        ),
                        ctx.provenance,
                    ));
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
    fn retargets_matching_slot_and_preserves_other_target_slots() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let original = create_creature(&mut game, "Original", alice, false);
        let ally = create_creature(&mut game, "Ally", alice, false);
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
        assert_eq!(created.len(), 1);

        let copy_entry = game
            .stack
            .iter()
            .find(|entry| entry.object_id == created[0])
            .expect("copy should have a stack entry");
        assert_eq!(
            copy_entry.targets,
            vec![Target::Object(ally), Target::Player(bob)]
        );
    }
}
