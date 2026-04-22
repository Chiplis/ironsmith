//! Apply continuous effect implementation.

use crate::continuous::{ContinuousEffect, EffectSourceType, EffectTarget, Modification};
use crate::effect::{ChoiceCount, EffectOutcome, Until, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_player_filter, resolve_value, validate_target,
};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::ObjectId;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::zone::Zone;

/// Runtime-resolved continuous modification templates.
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeModification {
    /// Change controller to the executing effect's controller.
    ChangeControllerToEffectController,
    /// Change controller to a resolved player filter.
    ChangeControllerToPlayer(crate::target::PlayerFilter),
    /// Resolve a copy source at execution, then apply a layer-1 copy effect.
    CopyOf {
        source: ChooseSpec,
        preserve_source_abilities: bool,
    },
    /// Resolve power/toughness deltas at execution, then apply layer 7c modification.
    ModifyPowerToughness { power: Value, toughness: Value },
    /// Resolve power delta at execution, then apply layer 7c modification.
    ModifyPower { value: Value },
    /// Resolve toughness delta at execution, then apply layer 7c modification.
    ModifyToughness { value: Value },
    /// Remove all abilities from the affected objects.
    RemoveAllAbilities,
}

/// Effect that registers a continuous effect with the game state.
///
/// This is a low-level primitive used by other effects to compose
/// continuous effects without duplicating registration logic.
pub type ApplyContinuousEffect = ironsmith_core::ApplyContinuousEffect<
    EffectTarget,
    Modification,
    RuntimeModification,
    crate::ConditionExpr,
    EffectSourceType,
>;

fn resolve_target(
    effect: &ApplyContinuousEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<(EffectTarget, Option<Vec<ObjectId>>, bool), ExecutionError> {
    let Some(spec) = &effect.target_spec else {
        return Ok((effect.target.clone(), None, false));
    };

    let mut objects = resolve_objects_for_effect(game, ctx, spec)?;
    if spec.is_target() {
        objects.retain(|id| validate_target(game, &ResolvedTarget::Object(*id), spec, ctx));
    }
    if objects.is_empty() {
        if spec.is_target() {
            return Ok((EffectTarget::AllPermanents, Some(Vec::new()), true));
        }
        if !matches!(spec.base(), ChooseSpec::All(_)) {
            return Err(ExecutionError::InvalidTarget);
        }
        return Ok((EffectTarget::AllPermanents, Some(Vec::new()), false));
    }

    if objects.len() == 1 {
        return Ok((EffectTarget::Specific(objects[0]), None, false));
    }

    Ok((EffectTarget::AllPermanents, Some(objects), false))
}

fn lock_targets_for_filter(
    filter: &crate::target::ObjectFilter,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<ObjectId> {
    let filter_ctx = ctx.filter_context(game);
    game.battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| obj.zone == Zone::Battlefield)
        .filter(|obj| filter.matches(obj, &filter_ctx, game))
        .map(|obj| obj.id)
        .collect()
}

fn resolve_set_pt_modification(
    effect: &ApplyContinuousEffect,
    game: &GameState,
    ctx: &ExecutionContext,
    modification: &Modification,
) -> Result<Modification, ExecutionError> {
    if !effect.resolve_set_pt_values_at_resolution {
        return Ok(modification.clone());
    }

    match modification {
        Modification::SetPower { value, sublayer } => Ok(Modification::SetPower {
            value: Value::Fixed(resolve_value(game, value, ctx)?),
            sublayer: *sublayer,
        }),
        Modification::SetToughness { value, sublayer } => Ok(Modification::SetToughness {
            value: Value::Fixed(resolve_value(game, value, ctx)?),
            sublayer: *sublayer,
        }),
        Modification::SetPowerToughness {
            power,
            toughness,
            sublayer,
        } => Ok(Modification::SetPowerToughness {
            power: Value::Fixed(resolve_value(game, power, ctx)?),
            toughness: Value::Fixed(resolve_value(game, toughness, ctx)?),
            sublayer: *sublayer,
        }),
        _ => Ok(modification.clone()),
    }
}

fn resolve_runtime_modification(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    modification: &RuntimeModification,
) -> Result<Modification, ExecutionError> {
    match modification {
        RuntimeModification::ChangeControllerToEffectController => {
            Ok(Modification::ChangeController(ctx.controller))
        }
        RuntimeModification::ChangeControllerToPlayer(player) => Ok(
            Modification::ChangeController(resolve_player_filter(game, player, ctx)?),
        ),
        RuntimeModification::CopyOf {
            source,
            preserve_source_abilities,
        } => {
            let source = resolve_objects_for_effect(game, ctx, source)?
                .into_iter()
                .next()
                .ok_or(ExecutionError::InvalidTarget)?;
            Ok(Modification::CopyOf {
                target_id: source,
                preserve_source_abilities: *preserve_source_abilities,
            })
        }
        RuntimeModification::ModifyPowerToughness { power, toughness } => {
            Ok(Modification::ModifyPowerToughness {
                power: resolve_value(game, power, ctx)?,
                toughness: resolve_value(game, toughness, ctx)?,
            })
        }
        RuntimeModification::ModifyPower { value } => {
            Ok(Modification::ModifyPower(resolve_value(game, value, ctx)?))
        }
        RuntimeModification::ModifyToughness { value } => Ok(Modification::ModifyToughness(
            resolve_value(game, value, ctx)?,
        )),
        RuntimeModification::RemoveAllAbilities => Ok(Modification::RemoveAllAbilities),
    }
}

fn target_object_ids(
    target: &EffectTarget,
    source_type: &Option<EffectSourceType>,
) -> Vec<ObjectId> {
    if let Some(EffectSourceType::Resolution { locked_targets }) = source_type {
        return locked_targets.clone();
    }
    match target {
        EffectTarget::Specific(id) => vec![*id],
        _ => Vec::new(),
    }
}

impl EffectExecutor for ApplyContinuousEffect {
    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        if let Some(spec) = &self.target_spec {
            return vec![spec.clone()];
        }
        match &self.target {
            EffectTarget::Filter(filter) => vec![ChooseSpec::All(filter.clone())],
            EffectTarget::Specific(id) => vec![ChooseSpec::SpecificObject(*id)],
            _ => Vec::new(),
        }
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let (target, spec_locked_targets, target_invalid) = resolve_target(self, game, ctx)?;
        if target_invalid {
            return Ok(EffectOutcome::target_invalid());
        }
        let mut source_type = self.source_type.clone();

        let filter_locked_targets = if let EffectTarget::Filter(filter) = &target {
            // Tagged filters depend on spell-resolution context and cannot be evaluated
            // dynamically once the one-shot effect has finished.
            let must_lock_tagged_filter = !filter.tagged_constraints.is_empty();
            if self.lock_filter_at_resolution || must_lock_tagged_filter {
                Some(lock_targets_for_filter(filter, game, ctx))
            } else {
                None
            }
        } else {
            None
        };

        let locked_targets = filter_locked_targets.or(spec_locked_targets);
        if let Some(locked_targets) = locked_targets {
            source_type = Some(EffectSourceType::Resolution { locked_targets });
        }

        let mut mods = Vec::with_capacity(
            self.additional_modifications.len() + self.runtime_modifications.len() + 1,
        );
        if let Some(modification) = &self.modification {
            mods.push(modification.clone());
        }
        mods.extend(self.additional_modifications.iter().cloned());
        for runtime_modification in &self.runtime_modifications {
            mods.push(resolve_runtime_modification(
                game,
                ctx,
                runtime_modification,
            )?);
        }
        let effect_group =
            (mods.len() > 1).then(|| game.effect_store.continuous_effects.next_effect_group_id());

        if self.require_creature_target {
            for id in target_object_ids(&target, &source_type) {
                let Some(obj) = game.object(id) else {
                    return Err(ExecutionError::ObjectNotFound(id));
                };
                if !obj.has_card_type(CardType::Creature) {
                    return Ok(EffectOutcome::target_invalid());
                }
            }
        }

        for modification in mods {
            let resolved_modification =
                resolve_set_pt_modification(self, game, ctx, &modification)?;
            let expires_end_of_turn = match self.until {
                Until::EndOfTurn | Until::YourNextTurn | Until::ControllersNextUntapStep => {
                    game.turn.turn_number
                }
                _ => u32::MAX,
            };
            let mut effect = ContinuousEffect::new(
                ctx.source,
                ctx.controller,
                target.clone(),
                resolved_modification,
            )
            .until(self.until.clone())
            .with_expires_end_of_turn(expires_end_of_turn);

            if let Some(source_type) = &source_type {
                effect = effect.with_source_type(source_type.clone());
            }
            if let Some(condition) = &self.condition {
                effect = effect.with_condition(condition.clone());
            }
            if let Some(group) = effect_group {
                effect = effect.with_group(group);
            }

            game.effect_store.continuous_effects.add_effect(effect);
        }

        game.refresh_continuous_state();

        Ok(EffectOutcome::resolved())
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        self.target_spec.as_ref()
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.target_spec.as_ref().map(ChooseSpec::count)
    }

    fn target_description(&self) -> &'static str {
        "target"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::{ContinuousEffect, Modification};
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn create_creature(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    #[test]
    fn conditional_continuous_effect_only_applies_while_condition_true() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", alice);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::new(
            ApplyContinuousEffect::new_runtime(
                EffectTarget::Specific(target),
                RuntimeModification::ModifyPowerToughness {
                    power: Value::Fixed(2),
                    toughness: Value::Fixed(-2),
                },
                Until::ThisLeavesTheBattlefield,
            )
            .with_condition(crate::ConditionExpr::SourceIsTapped),
        );

        execute_effect(&mut game, &effect, &mut ctx).expect("execute conditional apply");

        // Condition false: source is untapped
        assert_eq!(game.calculated_power(target), Some(2));
        assert_eq!(game.calculated_toughness(target), Some(2));

        game.tap(source);
        assert_eq!(game.calculated_power(target), Some(4));
        assert_eq!(game.calculated_toughness(target), Some(0));

        game.untap(source);
        assert_eq!(game.calculated_power(target), Some(2));
        assert_eq!(game.calculated_toughness(target), Some(2));
    }

    #[test]
    fn tagged_same_name_filter_locks_targets_using_execution_context_tags() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let target = create_creature(&mut game, "Bear", alice);
        let same_name_other = create_creature(&mut game, "Bear", alice);
        let different_name = create_creature(&mut game, "Wolf", alice);

        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        let target_snapshot = ObjectSnapshot::from_object(game.object(target).unwrap(), &game);
        ctx.set_tagged_objects(TagKey::from("marked"), vec![target_snapshot]);

        let mut filter = ObjectFilter::creature();
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("marked"),
            relation: TaggedOpbjectRelation::SameNameAsTagged,
        });
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from("marked"),
            relation: TaggedOpbjectRelation::IsNotTaggedObject,
        });

        let apply = ApplyContinuousEffect::new_runtime(
            EffectTarget::Filter(filter),
            RuntimeModification::ModifyPowerToughness {
                power: Value::Fixed(-2),
                toughness: Value::Fixed(-2),
            },
            Until::EndOfTurn,
        );

        execute_effect(&mut game, &Effect::new(apply), &mut ctx).unwrap();

        let effects = game.effect_store.continuous_effects.effects_sorted();
        assert_eq!(effects.len(), 1);
        match &effects[0].source_type {
            EffectSourceType::Resolution { locked_targets } => {
                assert!(locked_targets.contains(&same_name_other));
                assert!(!locked_targets.contains(&target));
                assert!(!locked_targets.contains(&different_name));
            }
            _ => panic!("expected resolution-locked effect for tagged filter"),
        }
    }

    #[test]
    fn double_power_uses_negative_power_at_resolution() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", alice);
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                source,
                alice,
                EffectTarget::Specific(target),
                Modification::ModifyPower(-5),
            )
            .until(Until::EndOfTurn),
        );
        assert_eq!(game.calculated_power(target), Some(-3));

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::new(
            ApplyContinuousEffect::new_runtime(
                EffectTarget::Specific(target),
                RuntimeModification::ModifyPowerToughness {
                    power: Value::PowerOf(Box::new(ChooseSpec::SpecificObject(target))),
                    toughness: Value::Fixed(0),
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        );

        execute_effect(&mut game, &effect, &mut ctx).expect("execute double power");

        assert_eq!(
            game.calculated_power(target),
            Some(-6),
            "doubling negative power should add the current negative value again"
        );
    }

    #[test]
    fn triple_power_uses_negative_power_at_resolution() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", alice);
        game.effect_store.continuous_effects.add_effect(
            ContinuousEffect::new(
                source,
                alice,
                EffectTarget::Specific(target),
                Modification::ModifyPower(-5),
            )
            .until(Until::EndOfTurn),
        );
        assert_eq!(game.calculated_power(target), Some(-3));

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::new(
            ApplyContinuousEffect::new_runtime(
                EffectTarget::Specific(target),
                RuntimeModification::ModifyPowerToughness {
                    power: Value::Scaled(
                        Box::new(Value::PowerOf(Box::new(ChooseSpec::SpecificObject(target)))),
                        2,
                    ),
                    toughness: Value::Fixed(0),
                },
                Until::EndOfTurn,
            )
            .require_creature_target(),
        );

        execute_effect(&mut game, &effect, &mut ctx).expect("execute triple power");

        assert_eq!(
            game.calculated_power(target),
            Some(-9),
            "tripling negative power should add twice the current negative value"
        );
    }
}
