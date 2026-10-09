//! Prevent all damage effect implementation.

use super::prevention_helpers::{
    SourceChoiceSelection, choose_source_of_your_choice,
    choose_source_sharing_activation_payment_color, register_prevention_shield,
};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_objects_from_spec, resolve_objects_for_effect};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
pub type PreventAllDamageEffect = ironsmith_core::PreventAllDamageEffect<crate::effect::Effect>;

/// Effect that prevents all damage until end of turn.
///
/// Can optionally filter to only prevent damage to certain permanents.
///
/// # Fields
///
/// * `filter` - Optional filter for which permanents to protect
///
/// # Example
///
/// ```ignore
/// // Prevent all damage this turn (Fog)
/// let effect = PreventAllDamageEffect::all();
///
/// // Prevent all damage to creatures you control this turn
/// let effect = PreventAllDamageEffect::matching(
///     ObjectFilter::creature().you_control()
/// );
/// ```
trait ExecuteBoundPrevention {
    fn execute_bound(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError>;
}
impl ExecuteBoundPrevention for PreventAllDamageEffect {
    fn execute_bound(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        game.establish_control_transition_boundary()
            .map_err(ExecutionError::ContinuousDiscovery)?;
        if self.protect_source_target && self.source_target.is_none() {
            return Err(ExecutionError::UnresolvableValue(
                "bidirectional prevention requires one bound source selector".into(),
            ));
        }
        let duration = match &self.until {
            crate::effect::Until::ForAsLongAs(predicate) => {
                let predicate = crate::effects::continuous::materialize_duration_predicate(
                    predicate,
                    &crate::continuous::EffectTarget::Source,
                    &None,
                    game,
                    ctx,
                )
                .ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "prevention duration requires exact bound objects".into(),
                    )
                })?;
                if !crate::continuous::continuous_duration_predicate_matches(&predicate, game) {
                    return Ok(EffectOutcome::resolved());
                }
                crate::effect::Until::ForAsLongAs(predicate)
            }
            other => other.clone(),
        };
        // CR 615.12 / 614.17a: while damage can't be prevented the shield
        // still exists and simply prevents nothing (the damage pipeline checks
        // preventability per event), so it keeps working once the
        // restriction ends later in its duration.
        let mut damage_filter = self.damage_filter.clone();
        let selected_sources = if let Some(source_target) = &self.source_target {
            let mut sources = resolve_objects_for_effect(game, ctx, source_target)?;
            sources.sort_unstable();
            sources.dedup();
            if sources.is_empty() {
                return if source_target.count().min == 0 {
                    Ok(EffectOutcome::count(0))
                } else { Err(ExecutionError::InvalidTarget) };
            }
            Some(sources)
        } else { None };
        if let Some(excluded_source_target) = &self.excluded_source_target {
            damage_filter.excluded_specific_source =
                resolve_objects_from_spec(game, excluded_source_target, ctx)?
                    .first()
                    .copied();
            if damage_filter.excluded_specific_source.is_none() {
                return Err(ExecutionError::InvalidTarget);
            }
        }
        if self.source_of_your_choice {
            let selection = if self.source_choice_shares_activation_mana_color {
                choose_source_sharing_activation_payment_color(game, ctx)
            } else if let Some(source_filter) = self.damage_filter.from_source.as_ref() {
                // "a creature of your choice": the choice is limited to the
                // sources the shield describes.
                super::prevention_helpers::choose_source_of_your_choice_matching_filter(
                    game,
                    ctx,
                    source_filter,
                )
            } else {
                choose_source_of_your_choice(game, ctx)
            };
            match selection {
                SourceChoiceSelection::Chosen(source) => {
                    damage_filter.from_specific_source = Some(source);
                    // CR 609.7a: the descriptor ("a red source of your
                    // choice") only limits the choice made on resolution; the
                    // shield then follows that object even if its
                    // characteristics change afterwards.
                    damage_filter.from_source = None;
                }
                SourceChoiceSelection::NoAvailableSource => return Ok(EffectOutcome::resolved()),
                SourceChoiceSelection::NoChoiceMade => return Ok(EffectOutcome::count(0)),
            }
        }

        let protected = if self.protect_source {
            crate::prevention::PreventionTarget::Permanent(ctx.source)
        } else {
            self.target.clone()
        };
        // Each selected source gets its own unlimited identity-bound shield;
        // selecting several sources must not silently keep only the first.
        let sources = selected_sources.map(|sources| sources.into_iter().map(Some).collect::<Vec<_>>())
            .unwrap_or_else(|| vec![damage_filter.from_specific_source]);
        for source in sources {
            let mut filter = damage_filter.clone();
            filter.from_specific_source = source;
            // A following "whenever damage is prevented this way" delayed
            // trigger links to this shield (CR 603.7, 615.5).
            ctx.last_prevention_shield = Some(register_prevention_shield(
                game,
                ctx,
                protected.clone(),
                None,
                duration.clone(),
                filter,
                self.follow_up_effects.clone(),
                ctx.targets.clone(),
                ctx.target_assignments.clone(),
            ));
            if self.protect_source_target {
                let source = source.ok_or_else(|| {
                    ExecutionError::UnresolvableValue(
                        "bidirectional prevention lost its selected source".into(),
                    )
                })?;
                // The incoming direction shares the exact selected incarnation,
                // but has no source restriction. It applies to every damager.
                register_prevention_shield(
                    game,
                    ctx,
                    crate::prevention::PreventionTarget::Permanent(source),
                    None,
                    duration.clone(),
                    crate::prevention::DamageFilter {
                        combat_only: self.damage_filter.combat_only,
                        noncombat_only: self.damage_filter.noncombat_only,
                        ..Default::default()
                    },
                    self.follow_up_effects.clone(),
                    ctx.targets.clone(),
                    ctx.target_assignments.clone(),
                );
            }
        }

        Ok(EffectOutcome::resolved())
    }
}

impl EffectExecutor for PreventAllDamageEffect {
    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&crate::effect::Effect)) {
        for effect in &self.follow_up_effects {
            visitor(effect);
        }
    }
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let result = crate::effects::tokens::execute_resource_transaction_atomically(
            game,
            ctx,
            |game, ctx| self.execute_bound(game, ctx),
        );
        if ctx.decision_maker.awaiting_choice() && result.is_ok() {
            return Ok(EffectOutcome::count(0));
        }
        result
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        self.get_target_spec().map(|spec| spec.count())
    }

    fn get_target_spec(&self) -> Option<&crate::target::ChooseSpec> {
        self.source_target
            .as_ref()
            .or(self.excluded_source_target.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Until;
    use crate::ids::PlayerId;
    use crate::target::ObjectFilter;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    #[test]
    fn test_prevent_all_damage() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::all(Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);

        // Shield should have unlimited prevention
        let shield = &game.effect_store.prevention_effects.shields()[0];
        assert!(shield.amount_remaining.is_none());
    }

    #[test]
    fn test_prevent_all_damage_to_your_creatures() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::your_creatures(Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 1);
    }

    #[test]
    fn test_prevent_all_damage_with_filter() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = PreventAllDamageEffect::matching(ObjectFilter::creature(), Until::EndOfTurn);
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.status, crate::effect::OutcomeStatus::Succeeded);
    }

    #[test]
    fn test_prevent_all_damage_clone_box() {
        let effect = PreventAllDamageEffect::all(Until::EndOfTurn);
        let cloned = effect.clone_box();
        assert!(format!("{:?}", cloned).contains("PreventAllDamageEffect"));
    }
    #[test]
    fn malformed_bidirectional_selector_fails_without_registering_partial_shields() {
        let mut game = setup_game();
        let source = game.new_object_id();
        let mut context = ExecutionContext::new_default(source, PlayerId::from_index(0));
        let effect = PreventAllDamageEffect::all(Until::YourNextTurn).protecting_target_source();
        assert!(matches!(
            effect.execute(&mut game, &mut context),
            Err(ExecutionError::UnresolvableValue(_))
        ));
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }

    #[test]
    fn a_source_presence_shield_does_not_start_after_its_source_is_gone() {
        let mut game = setup_game();
        let source = game.new_object_id();
        let mut context = ExecutionContext::new_default(source, PlayerId::from_index(0));
        let effect = PreventAllDamageEffect::all(Until::while_source_remains_on_battlefield());
        effect.execute(&mut game, &mut context).unwrap();
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }
}
