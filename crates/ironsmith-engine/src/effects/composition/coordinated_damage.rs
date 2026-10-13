//! One authored damage clause can assign different amounts to its recipients.
//! Keep its logical children and target bindings, but use one physical owner.

use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, DamageActionBinding, ExecutionContext, ExecutionError,
    ProgramActionScope, SimultaneousEffectCommit, SimultaneousEffectProposal,
};
use crate::game_state::GameState;

pub(super) fn is_shared_clause(sequence: &super::sequence::SequenceEffect) -> bool {
    sequence.surface.is_coordinated()
        && sequence.effects.len() > 1
        && sequence.effects.iter().all(|effect| {
            effect.0.supports_damage_action_cohort()
                && effect.0.supports_simultaneous_player_action()
        })
}

#[derive(Debug)]
struct CoordinatedDamageProposal {
    children: Vec<(ProgramActionScope, Box<dyn SimultaneousEffectProposal>)>,
}

pub(super) fn prepare(
    sequence: &super::sequence::SequenceEffect,
    game: &GameState,
    ctx: &mut ExecutionContext,
) -> Result<Box<dyn SimultaneousEffectProposal>, ExecutionError> {
    if !is_shared_clause(sequence) {
        return Err(ExecutionError::InternalError(
            "shared damage requires one coordinated damage clause".into(),
        ));
    }
    let scopes = super::sequence::coordinated_child_scopes(sequence, ctx);
    let mut children = Vec::with_capacity(sequence.effects.len());
    for (effect, scope) in sequence.effects.iter().zip(scopes) {
        let proposal = if let Some((targets, assignments)) = &scope.targets {
            ctx.with_temp_targets(targets.clone(), |ctx| {
                ctx.with_temp_target_assignments(assignments.clone(), |ctx| {
                    effect.prepare_simultaneous_player_action(game, ctx)
                })
            })?
        } else {
            effect.prepare_simultaneous_player_action(game, ctx)?
        };
        children.push((scope, proposal));
    }
    Ok(Box::new(CoordinatedDamageProposal { children }))
}

impl SimultaneousEffectProposal for CoordinatedDamageProposal {
    fn has_simultaneous_originals(&self) -> bool {
        true
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for (scope, proposal) in &mut self.children {
            scope.run(game, ctx, |game, ctx| proposal.prepare_selection(game, ctx))?;
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
        }
        Ok(())
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        for (scope, proposal) in &mut self.children {
            scope.run(game, ctx, |game, ctx| proposal.prepare_original(game, ctx))?;
            if ctx.decision_maker.awaiting_choice() {
                break;
            }
        }
        Ok(())
    }

    fn damage_action_inputs(&self) -> Option<crate::effects::damage::DamageActionInputs> {
        crate::effects::damage::DamageActionInputs::collect(
            self.children
                .iter()
                .map(|(_, proposal)| proposal.damage_action_inputs()),
        )
    }

    fn bind_damage_action(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        owner: &CompletedEffectOutputs,
    ) -> Result<DamageActionBinding, ExecutionError> {
        let mut bindings = Vec::with_capacity(self.children.len());
        for (scope, proposal) in self.children {
            bindings.push(scope.run(game, ctx, |game, ctx| {
                proposal.bind_damage_action(game, ctx, owner)
            })?);
        }
        Ok(DamageActionBinding::from_bindings(
            bindings,
            EffectOutcome::aggregate_summing_counts,
        ))
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        crate::effects::damage::complete_prepared_damage_action(game, ctx, self)
            .map(SimultaneousEffectCommit::finished)
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::damage::complete_prepared_damage_action(game, ctx, self)
            .map(CompletedEffectOutputs::into_outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::EffectExecutor;

    #[test]
    fn coordinated_damage_binds_the_selected_objects_controller_before_damage() {
        for amount in [0, 1] {
            let mut game = crate::tests::test_helpers::setup_two_player_game();
            let alice = crate::PlayerId::from_index(0);
            let bob = crate::PlayerId::from_index(1);
            let card = crate::card::CardBuilder::new(crate::CardId::new(), "Recipient")
                .card_types(vec![crate::CardType::Creature])
                .power_toughness(crate::card::PowerToughness::fixed(2, 3))
                .build();
            let source = game.create_object_from_card(&card, alice, crate::Zone::Battlefield);
            let target = game.create_object_from_card(&card, bob, crate::Zone::Battlefield);
            let sequence = super::super::sequence::SequenceEffect::coordinated(vec![
                crate::Effect::deal_damage(amount, crate::ChooseSpec::target(crate::ChooseSpec::creature()))
                    .tag("recipient"),
                crate::Effect::deal_damage(1, crate::ChooseSpec::Player(
                    crate::target::PlayerFilter::ControllerOf(crate::target::ObjectRef::tagged("recipient")),
                )),
            ]);
            let mut ctx = ExecutionContext::new_default(source, alice)
                .with_targets(vec![crate::effects::ResolvedTarget::Object(target)]);
            sequence.execute(&mut game, &mut ctx).unwrap();
            assert_eq!(game.damage_on(target), amount as u32);
            assert_eq!(game.player(bob).unwrap().life, 19);
            assert_eq!(game.player(alice).unwrap().life, 20);
        }
    }

    #[test]
    fn coordinated_damage_uses_one_batch_but_successive_damage_does_not() {
        for coordinated in [false, true] {
            for dispatched in [false, true] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let alice = crate::PlayerId::from_index(0);
                let bob = crate::PlayerId::from_index(1);
                let card = crate::card::CardBuilder::new(crate::CardId::new(), "Damage source")
                    .card_types(vec![crate::CardType::Instant])
                    .build();
                let source = game.create_object_from_card(&card, alice, crate::Zone::Stack);
                let effects = vec![
                    crate::Effect::deal_damage(4, crate::ChooseSpec::SpecificPlayer(bob)),
                    crate::Effect::deal_damage(2, crate::ChooseSpec::SpecificPlayer(alice)),
                ];
                let sequence = if coordinated {
                    super::super::sequence::SequenceEffect::coordinated(effects)
                } else {
                    super::super::sequence::SequenceEffect::new(effects)
                };
                let mut ctx = ExecutionContext::new_default(source, alice);
                let outcome = if dispatched {
                    crate::effects::execute_effect(
                        &mut game,
                        &crate::Effect::new(sequence),
                        &mut ctx,
                    )
                } else {
                    sequence.execute(&mut game, &mut ctx)
                }
                .unwrap();
                assert_eq!(game.player(alice).unwrap().life, 18);
                assert_eq!(game.player(bob).unwrap().life, 16);
                let batches = outcome
                    .events
                    .iter()
                    .filter(|event| event.downcast::<crate::events::DamageEvent>().is_some())
                    .map(|event| event.simultaneous_batch())
                    .collect::<Vec<_>>();
                assert_eq!(
                    batches.len(),
                    2,
                    "each physical assignment is published once"
                );
                if coordinated {
                    assert!(batches[0].is_some());
                    assert_eq!(batches[0], batches[1]);
                } else {
                    assert!(batches[0].is_none() || batches[0] != batches[1]);
                }
            }
        }
    }
}
