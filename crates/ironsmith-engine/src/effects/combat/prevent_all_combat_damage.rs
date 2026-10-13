//! Shared prevent-all-combat-damage wrapper.

use crate::effect::{ChoiceCount, Effect, EffectOutcome};
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::prevention::{DamageFilter, PreventionTarget};
use crate::target::ChooseSpec;
pub use ironsmith_core::{CombatDamagePreventionTarget, PreventAllCombatDamageEffect};

impl EffectExecutor for PreventAllCombatDamageEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }

    fn visit_child_effects(&self, visitor: &mut dyn FnMut(&Effect)) {
        visitor(&combat_prevention_body(&self.target, &self.until));
    }

    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        crate::effects::composition::execute_transaction(
            game,
            ctx,
            || crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            |game, ctx| {
                let body = if let CombatDamagePreventionTarget::ToAndFrom(spec) = &self.target {
                    // Resolve once: both shields protect the same exact incarnations.
                    let objects =
                        crate::effects::helpers::resolve_objects_for_effect(game, ctx, spec)?
                            .into_iter()
                            .filter(|id| game.object(*id).is_some())
                            .collect::<Vec<_>>();
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                            EffectOutcome::count(0),
                        ));
                    }
                    if objects.is_empty() && spec.count().min != 0 {
                        return Err(ExecutionError::InvalidTarget);
                    }
                    Effect::new(crate::effects::SequenceEffect::new(
                        objects
                            .into_iter()
                            .map(|id| {
                                combat_prevention_body(
                                    &CombatDamagePreventionTarget::ToAndFrom(
                                        ChooseSpec::SpecificObject(id),
                                    ),
                                    &self.until,
                                )
                            })
                            .collect(),
                    ))
                } else {
                    combat_prevention_body(&self.target, &self.until)
                };
                body.0.execute_child_with_outputs(game, ctx)
            },
        )
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        match &self.target {
            CombatDamagePreventionTarget::From(source)
            | CombatDamagePreventionTarget::ToAndFrom(source)
                if source.is_target() =>
            {
                Some(source)
            }
            _ => None,
        }
    }

    fn get_target_count(&self) -> Option<ChoiceCount> {
        match &self.target {
            CombatDamagePreventionTarget::From(source)
            | CombatDamagePreventionTarget::ToAndFrom(source)
                if source.is_target() =>
            {
                Some(source.count())
            }
            _ => None,
        }
    }

    fn target_description(&self) -> &'static str {
        "source to prevent combat damage from"
    }
}

/// One body definition for execution and structural inspection.
fn combat_prevention_body(
    target: &CombatDamagePreventionTarget,
    until: &crate::effect::Until,
) -> Effect {
    match target {
        CombatDamagePreventionTarget::All => Effect::new(
            super::PreventAllDamageEffect::all_with_filter(DamageFilter::combat(), until.clone()),
        ),
        CombatDamagePreventionTarget::Players => Effect::new(super::PreventAllDamageEffect::new(
            PreventionTarget::Players,
            DamageFilter::combat(),
            until.clone(),
        )),
        CombatDamagePreventionTarget::You => Effect::new(super::PreventAllDamageEffect::new(
            PreventionTarget::You,
            DamageFilter::combat(),
            until.clone(),
        )),
        CombatDamagePreventionTarget::From(source) => Effect::new(
            super::PreventAllCombatDamageFromEffect::new(source.clone(), until.clone()),
        ),
        CombatDamagePreventionTarget::ToAndFrom(object) => {
            Effect::new(crate::effects::SequenceEffect::new(vec![
                Effect::new(super::PreventAllCombatDamageFromEffect::new(
                    object.clone(),
                    until.clone(),
                )),
                Effect::new(
                    super::PreventAllDamageToTargetEffect::new(object.clone(), until.clone())
                        .with_filter(DamageFilter::combat()),
                ),
            ]))
        }
    }
}

#[cfg(test)]
mod composition_tests {
    use super::*;
    use crate::{card::CardBuilder, ids::CardId, types::CardType, zone::Zone};

    #[test]
    fn paired_shields_prevent_only_combat_to_and_from_captured_object() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let player = crate::ids::PlayerId::from_index(0);
        let card = CardBuilder::new(CardId::new(), "Protected")
            .card_types(vec![CardType::Creature])
            .build();
        let object = game.create_object_from_card(&card, player, Zone::Battlefield);
        let other = game.create_object_from_card(&card, player, Zone::Battlefield);
        let effect = PreventAllCombatDamageEffect {
            target: CombatDamagePreventionTarget::ToAndFrom(ChooseSpec::SpecificObject(object)),
            until: crate::effect::Until::EndOfTurn,
            source_would_deal_surface: false,
        };
        let mut ctx = ExecutionContext::new_default(object, player);
        effect.execute(&mut game, &mut ctx).unwrap();
        assert_eq!(game.effect_store.prevention_effects.shields().len(), 2);
        let colors = crate::color::ColorSet::COLORLESS;
        for (recipient, source, combat, expected) in [
            (object, other, true, 0),
            (other, object, true, 0),
            (object, other, false, 3),
            (other, object, false, 3),
            (other, other, true, 3),
        ] {
            assert_eq!(
                game.effect_store
                    .prevention_effects
                    .apply_prevention_to_permanent(
                        recipient,
                        player,
                        3,
                        combat,
                        source,
                        &colors,
                        &[CardType::Creature],
                        true
                    ),
                expected
            );
        }
    }

    #[test]
    fn missing_paired_target_does_not_install_partial_shield() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, crate::ids::PlayerId::from_index(0));
        let effect = PreventAllCombatDamageEffect {
            target: CombatDamagePreventionTarget::ToAndFrom(ChooseSpec::SpecificObject(source)),
            until: crate::effect::Until::EndOfTurn,
            source_would_deal_surface: false,
        };
        let _ = effect.execute(&mut game, &mut ctx);
        assert!(game.effect_store.prevention_effects.shields().is_empty());
    }
}
