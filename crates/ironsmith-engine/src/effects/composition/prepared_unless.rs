//! A selected unless branch exposes its child's payment claims and preparation
//! phases. The enclosing clause owns the result and the commit boundary.

use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionError, PaymentResourceClaim,
    SimultaneousEffectCommit, SimultaneousEffectProposal,
};
use crate::game_state::GameState;
use crate::ids::PlayerId;

#[derive(Debug, Clone, Copy)]
pub(super) enum UnlessBranch {
    Action,
    UnpaidConsequence,
    PaidCost,
}

impl UnlessBranch {
    pub(super) fn prepared(
        self,
        inner: Box<dyn SimultaneousEffectProposal>,
    ) -> Box<dyn SimultaneousEffectProposal> {
        Box::new(PreparedUnless {
            branch: self,
            inner,
        })
    }
}

#[derive(Debug)]
struct PreparedUnless {
    branch: UnlessBranch,
    inner: Box<dyn SimultaneousEffectProposal>,
}

impl SimultaneousEffectProposal for PreparedUnless {
    fn has_simultaneous_originals(&self) -> bool {
        self.inner.has_simultaneous_originals()
    }

    fn nominal_payment_quantity(&self) -> Option<u64> {
        self.inner.nominal_payment_quantity()
    }

    fn declared_payment_resources(&self) -> Vec<PaymentResourceClaim> {
        self.inner.declared_payment_resources()
    }

    fn declared_life_payments(&self) -> Vec<(PlayerId, u32)> {
        self.inner.declared_life_payments()
    }

    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.inner.prepare_selection(game, ctx)
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.inner.prepare_original(game, ctx)
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.inner.seal_original(game, ctx)
    }

    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }

    fn commit_original_with_outputs(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        let receipt = self.inner.commit_original_with_outputs(game, ctx)?;
        match self.branch {
            UnlessBranch::Action | UnlessBranch::UnpaidConsequence => Ok(receipt),
            UnlessBranch::PaidCost => Ok(super::compose_original_commits_with_projection_outputs(
                vec![receipt],
                Box::new(|outcomes| {
                    EffectOutcome::aggregate_with_primary_result(
                        EffectOutcome::declined(),
                        outcomes,
                    )
                }),
            )),
        }
    }

    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        match self.branch {
            UnlessBranch::Action => self.inner.commit(game, ctx),
            UnlessBranch::UnpaidConsequence | UnlessBranch::PaidCost => {
                super::complete_prepared_original(self, game, ctx)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cost::TotalCost;
    use crate::costs::Cost;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::BooleanContext;
    use crate::effect::{Effect, OutcomeStatus};
    use crate::target::PlayerFilter;

    struct Offer(bool);
    impl DecisionMaker for Offer {
        fn decide_boolean(&mut self, _: &GameState, ctx: &BooleanContext) -> bool {
            assert_eq!(ctx.player, PlayerId(1));
            self.0
        }
    }

    #[test]
    fn selected_unless_branches_preserve_the_payers_claim_and_the_clause_result() {
        let controller = PlayerId(0);
        let payer = PlayerId(1);
        for payment_clause in [false, true] {
            for accepted in [false, true] {
                let mut game = crate::tests::test_helpers::setup_two_player_game();
                let source = game.new_object_id();
                let mut offer = Offer(accepted);
                let mut ctx = ExecutionContext::new_default(source, controller)
                    .with_decision_maker(&mut offer);
                ctx.iteration.iterated_player = Some(payer);
                let consequence = vec![Effect::gain_life(3)];
                let effect = if payment_clause {
                    Effect::unless_pays_total_cost(
                        consequence,
                        PlayerFilter::IteratedPlayer,
                        TotalCost::from_cost(Cost::life(2)),
                    )
                } else {
                    Effect::unless_action(
                        consequence,
                        vec![Effect::pay_life(2)],
                        PlayerFilter::IteratedPlayer,
                    )
                };
                let mut proposal = effect
                    .prepare_simultaneous_player_action(&game, &mut ctx)
                    .unwrap();
                let claims = if accepted {
                    vec![(payer, 2)]
                } else {
                    Vec::new()
                };
                assert_eq!(proposal.declared_life_payments(), claims);
                assert_eq!(game.player(payer).unwrap().life, 20);
                proposal.prepare_selection(&mut game, &mut ctx).unwrap();
                proposal.prepare_original(&mut game, &mut ctx).unwrap();
                proposal.seal_original(&mut game, &mut ctx).unwrap();
                let receipt = proposal
                    .commit_original_with_outputs(&mut game, &mut ctx)
                    .unwrap();
                assert!(receipt.completion.is_none());
                let outcome = receipt.outcome.outcome;
                let expected = match (accepted, payment_clause) {
                    (false, _) => (OutcomeStatus::Succeeded, Some(3), 23, 20),
                    (true, false) => (OutcomeStatus::Succeeded, Some(2), 20, 18),
                    (true, true) => (OutcomeStatus::Declined, None, 20, 18),
                };
                assert_eq!(
                    (
                        outcome.status,
                        outcome.as_count(),
                        game.player(controller).unwrap().life,
                        game.player(payer).unwrap().life
                    ),
                    expected
                );
            }
        }
    }
}
