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
        /// Original parsed exception text, retained only for faithful compiled-text rendering.
        copy_exception_surface: Option<String>,
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
    game.next_turn_number_if_player_stayed(player)
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
    ctx: &mut ExecutionContext,
    modification: &Modification,
) -> Result<Modification, ExecutionError> {
    if let Modification::AddAbility(ability) = modification
        && let Some(materialized) = ability.materialize_resolution_values(game, ctx)?
    {
        return Ok(Modification::AddAbility(materialized));
    }

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
            copy_exception_surface: _,
        } => {
            let sacrificed_snapshot =
                source
                    .sacrificed_object_kind()
                    .and_then(|_| match source.base() {
                        ChooseSpec::Tagged(tag) => ctx.get_tagged(tag.as_str()).cloned(),
                        _ => None,
                    });
            let source_id = if let Some(snapshot) = sacrificed_snapshot.as_ref() {
                snapshot.object_id
            } else {
                resolve_objects_for_effect(game, ctx, source)?
                    .into_iter()
                    .next()
                    .ok_or(ExecutionError::InvalidTarget)?
            };
            let copiable_values = if let Some(snapshot) = sacrificed_snapshot.as_ref() {
                snapshot.copiable_values.clone()
            } else {
                let effects = game.all_continuous_effects();
                crate::continuous::copiable_values_with_effects(
                    source_id,
                    game.objects_map(),
                    &effects,
                    &game.battlefield,
                    game.commander_objects(),
                    game,
                )
                .ok_or(ExecutionError::InvalidTarget)?
            };
            Ok(Modification::CopyOf {
                target_id: source_id,
                copiable_values: Box::new(copiable_values),
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
            Ok(Modification::RemoveAbilityGeneric {
                ability,
                mode: ironsmith_core::AbilityLossMode::Lose,
            })
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

fn materialize_duration_object(
    reference: &ironsmith_core::ContinuousDurationObject,
    target: &EffectTarget,
    source_type: &Option<EffectSourceType>,
    ctx: &ExecutionContext,
) -> Option<ironsmith_core::ContinuousDurationObject> {
    use ironsmith_core::ContinuousDurationObject as ObjectRef;

    let id = match reference {
        ObjectRef::Source => ctx.source,
        ObjectRef::AffectedObject => {
            let affected = target_object_ids(target, source_type);
            if affected.len() != 1 {
                return None;
            }
            affected[0]
        }
        ObjectRef::Tagged(tag) => ctx
            .get_tagged_all(tag.as_str())
            .and_then(|snapshots| snapshots.first())?
            .object_id,
        ObjectRef::Specific(id) => *id,
    };
    Some(ObjectRef::Specific(id))
}

fn materialize_duration_player(
    reference: &ironsmith_core::ContinuousDurationPlayer,
    target: &EffectTarget,
    source_type: &Option<EffectSourceType>,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Option<ironsmith_core::ContinuousDurationPlayer> {
    use ironsmith_core::ContinuousDurationPlayer as PlayerRef;

    let id = match reference {
        PlayerRef::EffectController => ctx.controller,
        PlayerRef::ControllerOf(object) => {
            let object = materialize_duration_object(object, target, source_type, ctx)?;
            let ironsmith_core::ContinuousDurationObject::Specific(object) = object else {
                return None;
            };
            game.current_controller(object)?
        }
        PlayerRef::Tagged(tag) => *ctx.get_tagged_players(tag.as_str())?.first()?,
        PlayerRef::Specific(id) => *id,
    };
    Some(PlayerRef::Specific(id))
}

fn materialize_duration_predicate(
    predicate: &ironsmith_core::ContinuousDurationPredicate,
    target: &EffectTarget,
    source_type: &Option<EffectSourceType>,
    game: &GameState,
    ctx: &ExecutionContext,
) -> Option<ironsmith_core::ContinuousDurationPredicate> {
    use ironsmith_core::ContinuousDurationPredicate as Predicate;

    Some(match predicate {
        Predicate::All(predicates) => Predicate::All(
            predicates
                .iter()
                .map(|predicate| {
                    materialize_duration_predicate(predicate, target, source_type, game, ctx)
                })
                .collect::<Option<Vec<_>>>()?,
        ),
        Predicate::ObjectOnBattlefield(object) => Predicate::ObjectOnBattlefield(
            materialize_duration_object(object, target, source_type, ctx)?,
        ),
        Predicate::ObjectTapped(object) => Predicate::ObjectTapped(
            materialize_duration_object(object, target, source_type, ctx)?,
        ),
        Predicate::ObjectHasCounter {
            object,
            counter_type,
            minimum,
        } => Predicate::ObjectHasCounter {
            object: materialize_duration_object(object, target, source_type, ctx)?,
            counter_type: *counter_type,
            minimum: *minimum,
        },
        Predicate::ObjectAttachedTo {
            attachment,
            attached_to,
        } => Predicate::ObjectAttachedTo {
            attachment: materialize_duration_object(attachment, target, source_type, ctx)?,
            attached_to: materialize_duration_object(attached_to, target, source_type, ctx)?,
        },
        Predicate::ObjectIsEnchanted(object) => Predicate::ObjectIsEnchanted(
            materialize_duration_object(object, target, source_type, ctx)?,
        ),
        Predicate::PlayerIsMonarch(player) => Predicate::PlayerIsMonarch(
            materialize_duration_player(player, target, source_type, game, ctx)?,
        ),
        Predicate::ObjectPowerAtMostObject { lesser, greater } => {
            Predicate::ObjectPowerAtMostObject {
                lesser: materialize_duration_object(lesser, target, source_type, ctx)?,
                greater: materialize_duration_object(greater, target, source_type, ctx)?,
            }
        }
    })
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

        let materialized_until = match &self.until {
            Until::ForAsLongAs(predicate) => {
                let Some(predicate) = materialize_duration_predicate(
                    predicate,
                    &target,
                    &source_type,
                    game,
                    ctx,
                ) else {
                    return Ok(EffectOutcome::resolved());
                };
                if !crate::continuous::continuous_duration_predicate_matches(&predicate, game) {
                    return Ok(EffectOutcome::resolved());
                }
                Until::ForAsLongAs(predicate)
            }
            until => until.clone(),
        };

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
                // CR 800.4b: an effect cannot give control of an object to a
                // player who has left the game.
                if !game
                    .player(*new_controller)
                    .is_some_and(|player| player.is_in_game())
                {
                    continue;
                }
                for id in control_change_target_object_ids(&target, &source_type, game, ctx) {
                    if game.current_controller(id) != Some(*new_controller) {
                        game.clear_soulbond_pair(id);
                        game.set_summoning_sick(id);
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
            .until(materialized_until.clone())
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
    use crate::costs::Cost;
    use crate::effect::Effect;
    use crate::effects::{ExecutionContext, ResolvedTarget, execute_effect};
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::{CounterType, Object};
    use crate::snapshot::ObjectSnapshot;
    use crate::static_abilities::StaticAbility;
    use crate::tag::TagKey;
    use crate::target::{
        ChooseSpecSurfaceHint, ObjectFilter, SacrificedObjectKind, TaggedObjectConstraint,
        TaggedOpbjectRelation,
    };
    use crate::types::{CardType, Subtype, Supertype};
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

    fn create_land(game: &mut GameState, name: &str, controller: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .card_types(vec![CardType::Land])
            .build();
        let obj = Object::from_card(id, &card, controller, Zone::Battlefield);
        game.add_object(obj);
        id
    }

    fn fixed_block_cost_from_current_abilities(
        game: &GameState,
        blocker: ObjectId,
        attacker: ObjectId,
    ) -> ManaCost {
        game.current_abilities(blocker)
            .expect("blocker has current abilities")
            .iter()
            .filter_map(|ability| match &ability.kind {
                AbilityKind::Static(ability) => ability.block_cost_for_declaration(
                    game,
                    blocker,
                    game.current_controller(blocker)
                        .expect("blocker controller"),
                    blocker,
                    attacker,
                ),
                _ => None,
            })
            .find_map(|cost| cost.mana_cost().cloned())
            .expect("granted fixed block cost")
    }

    #[test]
    fn dynamic_block_tax_captures_x_and_applies_to_later_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Tax Source", alice);
        let existing_blocker = create_creature(&mut game, "Existing Blocker", bob);
        let attacker = create_creature(&mut game, "Attacker", alice);
        let dynamic_x = Cost::dynamic_mana(ironsmith_core::DynamicManaCost::new(
            ManaCost::from_pips(vec![vec![ManaSymbol::X]]),
            None,
            None,
            None,
            ironsmith_core::DynamicManaDisplayHint::Default,
        ));
        let block_cost = StaticAbility::block_cost(
            ObjectFilter::source(),
            ObjectFilter::creature(),
            TotalCost::from_cost(dynamic_x),
            "This creature can't block unless its controller pays {X}",
        );
        let effect = Effect::new(ApplyContinuousEffect::new(
            EffectTarget::Filter(ObjectFilter::creature()),
            Modification::AddAbility(block_cost),
            Until::EndOfTurn,
        ));
        let mut ctx = ExecutionContext::new_default(source, alice).with_x(2);

        execute_effect(&mut game, &effect, &mut ctx).expect("register dynamic block tax");
        let later_blocker = create_creature(&mut game, "Later Blocker", bob);

        for blocker in [existing_blocker, later_blocker] {
            assert_eq!(
                fixed_block_cost_from_current_abilities(&game, blocker, attacker).pips(),
                &[vec![ManaSymbol::Generic(2)]],
                "X should be captured as two and the live filter should include later entrants"
            );
        }
    }

    #[test]
    fn temporary_creature_type_addition_keeps_and_then_restores_land_type() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = create_creature(&mut game, "Animator", alice);
        let land = create_land(&mut game, "Animated Land", alice);
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = Effect::new(
            ApplyContinuousEffect::new(
                EffectTarget::Specific(land),
                Modification::AddCardTypes(vec![CardType::Creature]),
                Until::EndOfTurn,
            )
            .with_additional_modification(Modification::SetPowerToughness {
                power: Value::Fixed(3),
                toughness: Value::Fixed(3),
                sublayer: crate::continuous::PtSublayer::Setting,
            })
            .with_type_retention_surface(Some(ironsmith_core::TypeRetentionSurface::StillALand)),
        );

        execute_effect(&mut game, &effect, &mut ctx).expect("animate land");
        assert!(game.current_card_types(land).is_some_and(|types| {
            types.contains(&CardType::Land) && types.contains(&CardType::Creature)
        }));
        assert_eq!(game.current_power(land), Some(3));
        assert_eq!(game.current_toughness(land), Some(3));

        game.next_turn();
        assert!(game.current_card_types(land).is_some_and(|types| {
            types.contains(&CardType::Land) && !types.contains(&CardType::Creature)
        }));
        assert_eq!(game.current_power(land), None);
        assert_eq!(game.current_toughness(land), None);
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
                copy_exception_surface: None,
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
    fn sacrificed_continuous_copy_source_persists_snapshot_backed_values() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let copy_target = create_creature(&mut game, "Dane", alice);
        let sacrificed = create_creature(&mut game, "Battlefield Form", alice);
        let snapshot = ObjectSnapshot::from_object_with_calculated_characteristics(
            game.object(sacrificed).expect("sacrifice source exists"),
            &game,
        );
        let graveyard_id = game
            .move_object(
                sacrificed,
                Zone::Graveyard,
                crate::events::EventCause::effect(),
            )
            .expect("sacrifice source moves");
        game.object_mut(graveyard_id)
            .expect("moved card exists")
            .name = "Graveyard Form".into();

        let mut ctx = ExecutionContext::new_default(copy_target, alice);
        ctx.set_tagged_objects("sacrifice_cost_0", vec![snapshot]);
        let source = ChooseSpec::Tagged(TagKey::from("sacrifice_cost_0")).with_surface_hint(
            ChooseSpecSurfaceHint::SacrificedObject(SacrificedObjectKind::Creature),
        );
        let effect = Effect::new(ApplyContinuousEffect::new_runtime(
            EffectTarget::Specific(copy_target),
            RuntimeModification::CopyOf {
                source,
                preserve_source_abilities: true,
                name_override: None,
                name_override_surface: None,
                add_supertypes: Vec::new(),
                copy_exception_surface: None,
            },
            Until::EndOfTurn,
        ));

        execute_effect(&mut game, &effect, &mut ctx).expect("apply snapshot-backed copy");
        assert_eq!(
            game.current_characteristics(copy_target)
                .expect("copied characteristics")
                .name,
            "Battlefield Form"
        );
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
        assert!(
            game.is_summoning_sick(paired_a),
            "a creature becomes summoning sick when its controller changes"
        );
    }

    #[test]
    fn multiplayer_800_4b_cannot_give_control_to_player_who_left() {
        let mut game = GameState::new(vec!["Alice".into(), "Bob".into(), "Charlie".into()], 20);
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let target = create_creature(&mut game, "Control Target", bob);
        game.player_mut(alice).expect("Alice").has_left_game = true;
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, bob);
        let effect = Effect::new(ApplyContinuousEffect::new(
            EffectTarget::Specific(target),
            Modification::ChangeController(alice),
            Until::Forever,
        ));

        execute_effect(&mut game, &effect, &mut ctx)
            .expect("control instruction should be skipped");

        assert_eq!(game.current_controller(target), Some(bob));
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
        game.remove_summoning_sickness(target);

        game.next_turn();
        assert_eq!(game.current_controller(target), Some(bob));
        assert!(
            game.is_summoning_sick(target),
            "expiration of a control effect is also a controller change"
        );
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

    fn execute_latched_control(
        game: &mut GameState,
        source: ObjectId,
        controller: PlayerId,
        target: ObjectId,
        predicate: ironsmith_core::ContinuousDurationPredicate,
    ) {
        let effect = Effect::new(ApplyContinuousEffect::with_spec(
            ChooseSpec::target(ChooseSpec::creature()),
            Modification::ChangeController(controller),
            Until::ForAsLongAs(predicate),
        ));
        let mut ctx = ExecutionContext::new_default(source, controller)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        execute_effect(game, &effect, &mut ctx).expect("latched control effect resolves");
    }

    #[test]
    fn u079_latched_duration_that_is_false_at_creation_starts_no_effect() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Duration Source", alice);
        let target = create_creature(&mut game, "Duration Target", bob);

        execute_latched_control(
            &mut game,
            source,
            alice,
            target,
            ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                CounterType::Shield,
            ),
        );

        assert_eq!(game.current_controller(target), Some(bob));
        assert!(
            game.effect_store.continuous_effects.effects().is_empty(),
            "CR 611.2b: a false initial duration must create no effect"
        );
    }

    #[test]
    fn u079_counter_and_monarch_durations_expire_once_and_never_restart() {
        use ironsmith_core::{
            ContinuousDurationObject as ObjectRef, ContinuousDurationPlayer as PlayerRef,
            ContinuousDurationPredicate as Predicate,
        };

        let mut counter_game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut counter_game, "Counter Source", alice);
        let target = create_creature(&mut counter_game, "Counter Target", bob);
        counter_game
            .object_mut(target)
            .unwrap()
            .counters
            .insert(CounterType::Shield, 1);
        execute_latched_control(
            &mut counter_game,
            source,
            alice,
            target,
            Predicate::affected_object_has_counter(CounterType::Shield),
        );
        let (effect_id, timestamp) = {
            let effect = &counter_game.effect_store.continuous_effects.effects()[0];
            (effect.id, effect.timestamp)
        };
        assert_eq!(counter_game.current_controller(target), Some(alice));
        counter_game
            .object_mut(target)
            .unwrap()
            .counters
            .remove(&CounterType::Shield);
        assert_eq!(counter_game.current_controller(target), Some(bob));
        counter_game
            .object_mut(target)
            .unwrap()
            .counters
            .insert(CounterType::Shield, 1);
        assert_eq!(counter_game.current_controller(target), Some(bob));
        let effect = &counter_game.effect_store.continuous_effects.effects()[0];
        assert_eq!((effect.id, effect.timestamp), (effect_id, timestamp));

        let mut monarch_game = setup_game();
        let source = create_creature(&mut monarch_game, "Monarch Source", alice);
        let target = create_creature(&mut monarch_game, "Monarch Target", bob);
        monarch_game.set_monarch(Some(bob));
        execute_latched_control(
            &mut monarch_game,
            source,
            alice,
            target,
            Predicate::PlayerIsMonarch(PlayerRef::ControllerOf(ObjectRef::AffectedObject)),
        );
        assert_eq!(monarch_game.current_controller(target), Some(alice));
        monarch_game.set_monarch(Some(alice));
        assert_eq!(monarch_game.current_controller(target), Some(bob));
        monarch_game.set_monarch(Some(bob));
        assert_eq!(monarch_game.current_controller(target), Some(bob));
    }

    #[test]
    fn u079_tagged_attachment_duration_latches_and_preserves_separate_identity() {
        use ironsmith_core::{
            ContinuousDurationObject as ObjectRef, ContinuousDurationPredicate as Predicate,
        };

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Attachment Trigger Source", alice);
        let target = create_creature(&mut game, "Attachment Host", bob);
        let aura_card = CardBuilder::new(CardId::from_raw(97_901), "Duration Aura")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let aura = game.create_object_from_card(&aura_card, alice, Zone::Battlefield);
        assert!(game.attach_object_to_target(
            aura,
            crate::object::AttachmentTarget::Object(target)
        ));
        let aura_snapshot = ObjectSnapshot::from_object(game.object(aura).unwrap(), &game);

        let effect = Effect::new(ApplyContinuousEffect::with_spec(
            ChooseSpec::target(ChooseSpec::creature()),
            Modification::ChangeController(alice),
            Until::ForAsLongAs(Predicate::ObjectAttachedTo {
                attachment: ObjectRef::Tagged(TagKey::from("triggering")),
                attached_to: ObjectRef::AffectedObject,
            }),
        ));
        let mut ctx = ExecutionContext::new_default(source, alice)
            .with_targets(vec![ResolvedTarget::Object(target)]);
        ctx.set_tagged_objects("triggering", vec![aura_snapshot]);
        execute_effect(&mut game, &effect, &mut ctx).expect("attachment duration resolves");
        assert_eq!(game.current_controller(target), Some(alice));

        assert!(game.detach_object_from_current_target(aura));
        assert_eq!(game.current_controller(target), Some(bob));
        assert!(game.attach_object_to_target(
            aura,
            crate::object::AttachmentTarget::Object(target)
        ));
        assert_eq!(game.current_controller(target), Some(bob));
    }

    #[test]
    fn u079_composite_power_duration_uses_current_characteristics_and_latches() {
        use ironsmith_core::{
            ContinuousDurationObject as ObjectRef, ContinuousDurationPredicate as Predicate,
        };

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Power Source", alice);
        let target = create_creature(&mut game, "Power Target", bob);
        game.tap(source);
        execute_latched_control(
            &mut game,
            source,
            alice,
            target,
            Predicate::all([
                Predicate::ObjectTapped(ObjectRef::Source),
                Predicate::ObjectPowerAtMostObject {
                    lesser: ObjectRef::AffectedObject,
                    greater: ObjectRef::Source,
                },
            ]),
        );
        assert_eq!(game.current_controller(target), Some(alice));

        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                source,
                bob,
                EffectTarget::Specific(target),
                Modification::ModifyPowerToughness {
                    power: 2,
                    toughness: 0,
                },
            ));
        assert_eq!(game.calculated_power(target), Some(4));
        assert_eq!(game.current_controller(target), Some(bob));

        game.effect_store
            .continuous_effects
            .add_effect(ContinuousEffect::new(
                source,
                bob,
                EffectTarget::Specific(source),
                Modification::ModifyPowerToughness {
                    power: 3,
                    toughness: 0,
                },
            ));
        assert!(game.calculated_power(source).unwrap() > game.calculated_power(target).unwrap());
        assert_eq!(game.current_controller(target), Some(bob));
    }

    #[test]
    fn u079_phased_out_tracked_object_expires_duration_before_it_phases_in() {
        use ironsmith_core::{
            ContinuousDurationObject as ObjectRef, ContinuousDurationPredicate as Predicate,
        };

        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Phasing Source", alice);
        let target = create_creature(&mut game, "Phasing Target", bob);
        execute_latched_control(
            &mut game,
            source,
            alice,
            target,
            Predicate::ObjectOnBattlefield(ObjectRef::AffectedObject),
        );
        assert_eq!(game.current_controller(target), Some(alice));
        game.phase_out(target);
        assert_eq!(game.current_controller(target), Some(bob));
        game.phase_in(target);
        assert_eq!(game.current_controller(target), Some(bob));
    }

    #[test]
    fn u079_cloned_duration_payload_materializes_independent_affected_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = create_creature(&mut game, "Copied Ability Source", alice);
        let first = create_creature(&mut game, "First Target", bob);
        let second = create_creature(&mut game, "Second Target", bob);
        for target in [first, second] {
            game.object_mut(target)
                .unwrap()
                .counters
                .insert(CounterType::Shield, 1);
        }
        let copied_effect = Effect::new(ApplyContinuousEffect::with_spec(
            ChooseSpec::target(ChooseSpec::creature()),
            Modification::ChangeController(alice),
            Until::ForAsLongAs(
                ironsmith_core::ContinuousDurationPredicate::affected_object_has_counter(
                    CounterType::Shield,
                ),
            ),
        ));
        for target in [first, second] {
            let mut ctx = ExecutionContext::new_default(source, alice)
                .with_targets(vec![ResolvedTarget::Object(target)]);
            execute_effect(&mut game, &copied_effect.clone(), &mut ctx)
                .expect("each copied duration resolves independently");
        }
        game.object_mut(first)
            .unwrap()
            .counters
            .remove(&CounterType::Shield);
        assert_eq!(game.current_controller(first), Some(bob));
        assert_eq!(game.current_controller(second), Some(alice));
    }
}
