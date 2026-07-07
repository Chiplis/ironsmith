//! Apply continuous effect implementation.

use crate::continuous::{ContinuousEffect, EffectSourceType, EffectTarget, Modification};
use crate::decision::SelectFirstDecisionMaker;
use crate::effect::{ChoiceCount, EffectOutcome, Until, Value};
use crate::effects::helpers::{
    resolve_objects_for_effect, resolve_objects_from_spec, resolve_player_filter, resolve_value,
    validate_target,
};
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError, ResolvedTarget};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::{ChooseSpec, SourceReferenceSurface};
use crate::types::{CardType, Supertype};
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
        name_override: Option<String>,
        name_override_surface: Option<SourceReferenceSurface>,
        add_supertypes: Vec<Supertype>,
    },
    /// Resolve power/toughness deltas at execution, then apply layer 7c modification.
    ModifyPowerToughness { power: Value, toughness: Value },
    /// Resolve power delta at execution, then apply layer 7c modification.
    ModifyPower { value: Value },
    /// Resolve toughness delta at execution, then apply layer 7c modification.
    ModifyToughness { value: Value },
    /// Remove all abilities from the affected objects.
    RemoveAllAbilities,
    /// Remove the activated ability currently resolving.
    RemoveThisAbility,
    /// Set the Aura attachment restriction while this effect applies.
    SetAuraAttachmentFilter(crate::object::AuraAttachmentFilter),
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

fn next_turn_number_for_player(game: &GameState, player: PlayerId) -> u32 {
    if game.turn_store.turn_order.is_empty() {
        return game.turn.turn_number;
    }

    let mut simulated_active_player = game.turn.active_player;
    let mut simulated_turn_number = game.turn.turn_number;
    let mut simulated_extra_turns = game.turn_store.extra_turns.clone();
    let mut simulated_skip_next_turn = game.turn_store.skip_next_turn.clone();
    let max_iterations = game
        .turn_store
        .turn_order
        .len()
        .saturating_mul(16)
        .saturating_add(simulated_extra_turns.len().saturating_mul(2))
        .saturating_add(16)
        .max(1);

    for _ in 0..max_iterations {
        let next_player = if !simulated_extra_turns.is_empty() {
            simulated_extra_turns.remove(0)
        } else {
            let current_index = game
                .turn_store
                .turn_order
                .iter()
                .position(|&p| p == simulated_active_player)
                .unwrap_or(0);

            let mut next_index = (current_index + 1) % game.turn_store.turn_order.len();
            let start_index = next_index;

            loop {
                let candidate = game.turn_store.turn_order[next_index];
                let is_in_game = game.player(candidate).is_some_and(|p| p.is_in_game());

                if is_in_game {
                    if simulated_skip_next_turn.remove(&candidate) {
                        next_index = (next_index + 1) % game.turn_store.turn_order.len();
                        if next_index == start_index {
                            break;
                        }
                        continue;
                    }
                    break;
                }

                next_index = (next_index + 1) % game.turn_store.turn_order.len();
                if next_index == start_index {
                    break;
                }
            }

            game.turn_store.turn_order[next_index]
        };

        simulated_turn_number = simulated_turn_number.saturating_add(1);
        simulated_active_player = next_player;
        if simulated_active_player == player {
            return simulated_turn_number;
        }
    }

    game.turn.turn_number.saturating_add(1)
}

fn resolve_target(
    effect: &ApplyContinuousEffect,
    game: &mut GameState,
    ctx: &mut ExecutionContext,
) -> Result<(EffectTarget, Option<Vec<ObjectId>>, bool), ExecutionError> {
    let Some(spec) = &effect.target_spec else {
        return Ok((effect.target.clone(), None, false));
    };

    let mut objects = if spec.is_target() {
        resolve_objects_for_effect(game, ctx, spec)?
    } else {
        resolve_non_target_continuous_objects(game, ctx, spec)?
    };
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

fn resolve_non_target_continuous_objects(
    game: &GameState,
    ctx: &ExecutionContext,
    spec: &ChooseSpec,
) -> Result<Vec<ObjectId>, ExecutionError> {
    if let ChooseSpec::Object(filter) = spec.base()
        && !filter.tagged_constraints.is_empty()
    {
        return resolve_continuous_filter_objects(game, ctx, filter);
    }

    if let Ok(objects) = resolve_objects_from_spec(game, spec, ctx)
        && !objects.is_empty()
    {
        return Ok(objects);
    }

    if let ChooseSpec::Object(filter) = spec.base() {
        return resolve_continuous_filter_objects(game, ctx, filter);
    }

    Err(ExecutionError::InvalidTarget)
}

fn resolve_continuous_filter_objects(
    game: &GameState,
    ctx: &ExecutionContext,
    filter: &crate::target::ObjectFilter,
) -> Result<Vec<ObjectId>, ExecutionError> {
    let filter_ctx = ctx.filter_context(game);
    let objects: Vec<ObjectId> = game
        .battlefield
        .iter()
        .filter_map(|&id| game.object(id))
        .filter(|obj| obj.zone == Zone::Battlefield)
        .filter(|obj| filter.matches(obj, &filter_ctx, game))
        .map(|obj| obj.id)
        .collect();
    if objects.is_empty() {
        Err(ExecutionError::InvalidTarget)
    } else {
        Ok(objects)
    }
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
            name_override,
            name_override_surface,
            add_supertypes,
        } => {
            let source = resolve_objects_for_effect(game, ctx, source)?
                .into_iter()
                .next()
                .ok_or(ExecutionError::InvalidTarget)?;
            Ok(Modification::CopyOf {
                target_id: source,
                preserve_source_abilities: *preserve_source_abilities,
                name_override: name_override.clone(),
                name_override_surface: name_override_surface.clone(),
                add_supertypes: add_supertypes.clone(),
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
        RuntimeModification::RemoveThisAbility => {
            let ability_index = ctx.ability_index.ok_or_else(|| {
                ExecutionError::Impossible(
                    "this ability removal requires a resolving activated ability".to_string(),
                )
            })?;
            let ability = game
                .current_ability(ctx.source, ability_index)
                .ok_or_else(|| {
                    ExecutionError::Impossible(
                        "resolving source no longer has the referenced ability".to_string(),
                    )
                })?;
            Ok(Modification::RemoveAbilityGeneric(ability))
        }
        RuntimeModification::SetAuraAttachmentFilter(filter) => {
            Ok(Modification::SetAuraAttachmentFilter(filter.clone()))
        }
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

fn control_change_target_object_ids(
    target: &EffectTarget,
    source_type: &Option<EffectSourceType>,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Vec<ObjectId> {
    let explicit = target_object_ids(target, source_type);
    if !explicit.is_empty() {
        return explicit;
    }
    match target {
        EffectTarget::Filter(filter) => lock_targets_for_filter(filter, game, ctx),
        EffectTarget::AllPermanents => game
            .battlefield
            .iter()
            .copied()
            .filter(|id| game.object(*id).is_some())
            .collect(),
        EffectTarget::AllCreatures => game
            .battlefield
            .iter()
            .copied()
            .filter(|id| game.current_is_creature(*id))
            .collect(),
        _ => Vec::new(),
    }
}

fn is_controller_change_cost(effect: &ApplyContinuousEffect) -> bool {
    let base_is_controller_change = effect
        .modification
        .as_ref()
        .is_none_or(|modification| matches!(modification, Modification::ChangeController(_)));
    let additional_are_controller_changes = effect
        .additional_modifications
        .iter()
        .all(|modification| matches!(modification, Modification::ChangeController(_)));
    let runtime_are_controller_changes = effect.runtime_modifications.iter().all(|modification| {
        matches!(
            modification,
            RuntimeModification::ChangeControllerToEffectController
                | RuntimeModification::ChangeControllerToPlayer(_)
        )
    });
    let has_controller_change = effect
        .modification
        .as_ref()
        .is_some_and(|modification| matches!(modification, Modification::ChangeController(_)))
        || effect
            .additional_modifications
            .iter()
            .any(|modification| matches!(modification, Modification::ChangeController(_)))
        || effect.runtime_modifications.iter().any(|modification| {
            matches!(
                modification,
                RuntimeModification::ChangeControllerToEffectController
                    | RuntimeModification::ChangeControllerToPlayer(_)
            )
        });

    has_controller_change
        && base_is_controller_change
        && additional_are_controller_changes
        && runtime_are_controller_changes
}

fn materialize_runtime_condition(
    condition: &crate::ConditionExpr,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Result<crate::ConditionExpr, ExecutionError> {
    match condition {
        crate::ConditionExpr::TaggedObjectIsTopOfLibrary { tag, player } => {
            let player_id = resolve_player_filter(game, player, ctx)?;
            let Some(snapshot) = ctx
                .get_tagged_all(tag.as_str())
                .and_then(|items| items.first())
            else {
                return Ok(crate::ConditionExpr::Custom(
                    "unresolved tagged top-library condition",
                ));
            };
            Ok(crate::ConditionExpr::StableObjectIsTopOfLibrary {
                stable_id: snapshot.stable_id,
                player: player_id,
                library_top_revision: game.library_top_revision(player_id),
            })
        }
        crate::ConditionExpr::Not(inner) => Ok(crate::ConditionExpr::Not(Box::new(
            materialize_runtime_condition(inner, game, ctx)?,
        ))),
        crate::ConditionExpr::And(left, right) => Ok(crate::ConditionExpr::And(
            Box::new(materialize_runtime_condition(left, game, ctx)?),
            Box::new(materialize_runtime_condition(right, game, ctx)?),
        )),
        crate::ConditionExpr::Or(left, right) => Ok(crate::ConditionExpr::Or(
            Box::new(materialize_runtime_condition(left, game, ctx)?),
            Box::new(materialize_runtime_condition(right, game, ctx)?),
        )),
        _ => Ok(condition.clone()),
    }
}

impl EffectExecutor for ApplyContinuousEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        is_controller_change_cost(self).then_some(self)
    }

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
            if let Modification::ChangeController(new_controller) = &resolved_modification {
                for id in control_change_target_object_ids(&target, &source_type, game, ctx) {
                    if game.current_controller(id) != Some(*new_controller) {
                        game.clear_soulbond_pair(id);
                    }
                }
            }
            let expires_end_of_turn = match self.until {
                Until::EndOfTurn
                | Until::YourNextTurn
                | Until::YourNextUpkeep
                | Until::ControllersNextUntapStep => game.turn.turn_number,
                Until::YourNextTurnEnd => next_turn_number_for_player(game, ctx.controller),
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
                effect =
                    effect.with_condition(materialize_runtime_condition(condition, game, ctx)?);
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
        self.target_spec.as_ref().filter(|spec| spec.is_target())
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        self.get_target_spec().map(ChooseSpec::count)
    }

    fn target_description(&self) -> &'static str {
        "target"
    }
}

impl CostExecutableEffect for ApplyContinuousEffect {
    fn can_execute_as_cost(
        &self,
        game: &GameState,
        source: ObjectId,
        controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        if !is_controller_change_cost(self) {
            return Err(CostValidationError::Other(
                "only controller-change continuous effects can be paid as costs".to_string(),
            ));
        }

        let mut simulated_game = game.clone();
        let mut simulated_decision_maker = SelectFirstDecisionMaker;
        let mut simulated_ctx =
            ExecutionContext::new(source, controller, &mut simulated_decision_maker);
        match self.execute(&mut simulated_game, &mut simulated_ctx) {
            Ok(outcome) if !outcome.status.is_failure() => Ok(()),
            Ok(outcome) => Err(CostValidationError::Other(format!(
                "controller-change cost could not resolve: {:?}",
                outcome.status
            ))),
            Err(err) => Err(CostValidationError::Other(format!(
                "controller-change cost could not resolve: {err:?}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Ability, AbilityKind, ActivatedAbility, ActivationTiming};
    use crate::card::{CardBuilder, PowerToughness};
    use crate::continuous::{ContinuousEffect, Modification};
    use crate::cost::TotalCost;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::snapshot::ObjectSnapshot;
    use crate::tag::TagKey;
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::types::{CardType, Supertype};
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
    fn copy_runtime_exception_overrides_name_and_adds_supertype() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let copy_target = create_creature(&mut game, "Sarkhan", alice);
        let copied_dragon = create_creature(&mut game, "Dragon Stand-In", alice);

        let mut ctx = ExecutionContext::new_default(copy_target, alice);
        let effect = Effect::new(ApplyContinuousEffect::new_runtime(
            EffectTarget::Specific(copy_target),
            RuntimeModification::CopyOf {
                source: ChooseSpec::SpecificObject(copied_dragon),
                preserve_source_abilities: false,
                name_override: Some("Sarkhan, Soul Aflame".to_string()),
                name_override_surface: None,
                add_supertypes: vec![Supertype::Legendary],
            },
            Until::EndOfTurn,
        ));

        execute_effect(&mut game, &effect, &mut ctx).expect("execute copy effect");

        let current = game
            .current_characteristics(copy_target)
            .expect("current characteristics");
        assert_eq!(current.name, "Sarkhan, Soul Aflame");
        assert!(current.supertypes.contains(&Supertype::Legendary));
        assert_eq!(current.power, Some(2));
        assert_eq!(current.toughness, Some(2));
    }

    #[test]
    fn change_controller_continuous_effect_breaks_soulbond_pair() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let paired_a = create_creature(&mut game, "Soulbond A", alice);
        let paired_b = create_creature(&mut game, "Soulbond B", alice);
        game.set_soulbond_pair(paired_a, paired_b);
        assert_eq!(game.soulbond_partner(paired_a), Some(paired_b));

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, bob);
        let effect = Effect::new(ApplyContinuousEffect::new(
            EffectTarget::Specific(paired_a),
            Modification::ChangeController(bob),
            Until::EndOfTurn,
        ));

        execute_effect(&mut game, &effect, &mut ctx).expect("execute control change");

        assert_eq!(game.soulbond_partner(paired_a), None);
        assert_eq!(game.soulbond_partner(paired_b), None);
    }

    #[test]
    fn change_controller_until_next_turn_end_survives_controller_next_turn() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        game.turn.active_player = bob;
        let source = create_creature(&mut game, "Source", alice);
        let target = create_creature(&mut game, "Target", bob);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::new(ApplyContinuousEffect::new(
            EffectTarget::Specific(target),
            Modification::ChangeController(alice),
            Until::YourNextTurnEnd,
        ));

        execute_effect(&mut game, &effect, &mut ctx).expect("execute control change");
        assert_eq!(game.current_controller(target), Some(alice));

        game.next_turn();
        assert_eq!(game.turn.active_player, alice);
        assert_eq!(game.current_controller(target), Some(alice));

        game.next_turn();
        assert_eq!(game.current_controller(target), Some(bob));
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

    #[test]
    fn remove_this_ability_removes_resolving_activated_ability_only() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Licid Stand-In", alice);
        game.object_mut(source)
            .expect("source exists")
            .abilities_mut()
            .push(Ability {
                kind: AbilityKind::Activated(ActivatedAbility {
                    mana_cost: TotalCost::default(),
                    effects: crate::resolution::ResolutionProgram::from_effects(vec![]),
                    choices: vec![],
                    timing: ActivationTiming::AnyTime,
                    additional_restrictions: vec![],
                    activation_restrictions: vec![],
                    mana_output: None,
                    activation_condition: None,
                    mana_usage_restrictions: vec![],
                    is_loyalty_ability: false,
                }),
                functional_zones: vec![Zone::Battlefield],
            });
        let ability_index = game
            .object(source)
            .unwrap()
            .abilities
            .iter()
            .position(|ability| matches!(ability.kind, AbilityKind::Activated(_)))
            .expect("activated ability exists");

        let mut ctx =
            ExecutionContext::new_default(source, alice).with_ability_index(ability_index);
        let effect = Effect::new(ApplyContinuousEffect::new_runtime(
            EffectTarget::Specific(source),
            RuntimeModification::RemoveThisAbility,
            Until::Forever,
        ));

        execute_effect(&mut game, &effect, &mut ctx).expect("remove this ability");

        let current = game.current_abilities(source).expect("source abilities");
        assert!(
            current
                .iter()
                .all(|ability| !matches!(ability.kind, AbilityKind::Activated(_))),
            "the resolving activated ability should be absent from current characteristics"
        );
    }
}
