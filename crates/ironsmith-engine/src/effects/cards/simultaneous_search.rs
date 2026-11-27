//! CR 701.23i. The searchers inspect their libraries together, choose in turn
//! order, and only then disclose and act on what they found.
use super::search_overrides::{
    LibrarySearchLook, LibrarySearchRequest, OppositionAgentSearch,
    begin_opposition_agent_search_control, finish_opposition_agent_search_control,
};
use crate::effect::EffectOutcome;
use crate::effects::{
    CompletedEffectOutputs, ExecutionContext, ExecutionError, SimultaneousEffectCommit,
    SimultaneousEffectProposal,
};
use crate::game_state::GameState;
use crate::ids::PlayerId;

pub(crate) fn controller_searches_each_library(effect: &crate::effect::Effect) -> bool {
    let effect = effect
        .downcast_ref::<crate::effects::TaggedEffect>()
        .map_or(effect, |tagged| tagged.effect.as_ref());
    effect
        .downcast_ref::<crate::effects::SearchLibraryEffect>()
        .is_some_and(|search| {
            search.chooser == crate::target::PlayerFilter::You
                && search.player == crate::target::PlayerFilter::IteratedPlayer
        })
}

/// After inspecting all the libraries, one searcher may finish any remaining
/// library next. Keep the original indices for simultaneous moves and shuffles.
pub(crate) fn choose_next_library(
    game: &GameState,
    ctx: &mut ExecutionContext,
    remaining: &mut Vec<(usize, PlayerId)>,
) -> Option<usize> {
    let position = if let Some(index) = remaining.iter().position(|(_, owner)| {
        !game.can_search_library_from_effect(ctx.controller, *owner, ctx.controller)
    }) {
        index
    } else if remaining.len() == 1 {
        0
    } else {
        let choices: Vec<_> = remaining
            .iter()
            .enumerate()
            .map(|(index, (_, owner))| {
                let name = &game.player(*owner).expect("library owner exists").name;
                (format!("Finish searching {name}'s library"), index)
            })
            .collect();
        let next = crate::decisions::ask_choose_one(
            game,
            &mut ctx.decision_maker,
            ctx.controller,
            ctx.source,
            &choices,
        );
        if ctx.decision_maker.awaiting_choice() {
            return None;
        }
        next.unwrap_or(0)
    };
    Some(remaining.remove(position).0)
}

pub(crate) trait LibrarySearchAction: Send + std::fmt::Debug + 'static {
    type Selection: Send + std::fmt::Debug;
    fn select(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
        look: LibrarySearchLook,
    ) -> Result<Self::Selection, ExecutionError>;
    fn disclose(
        &self,
        _selection: &mut Self::Selection,
        _game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        Ok(())
    }
    fn finish(
        &self,
        selection: Self::Selection,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError>;
}

#[derive(Debug)]
struct SimultaneousSearch<A: LibrarySearchAction> {
    action: A,
    request: LibrarySearchRequest,
    look: Option<LibrarySearchLook>,
    selection: Option<A::Selection>,
    policy: Option<OppositionAgentSearch>,
    disclosed: bool,
}

pub(crate) fn prepare_search<A: LibrarySearchAction>(
    action: A,
    request: LibrarySearchRequest,
) -> Box<dyn SimultaneousEffectProposal> {
    Box::new(SimultaneousSearch {
        action,
        request,
        look: None,
        selection: None,
        policy: None,
        disclosed: false,
    })
}

impl<A: LibrarySearchAction> SimultaneousEffectProposal for SimultaneousSearch<A> {
    fn prepare_selection(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        if self.look.is_none() && self.selection.is_none() {
            let look = self.request.look(game, ctx);
            self.policy = look.policy;
            self.look = Some(look);
        }
        Ok(())
    }

    fn prepare_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_selection(game, ctx)?;
        if !ctx.decision_maker.awaiting_choice()
            && let Some(look) = self.look.take()
        {
            self.selection = Some(self.action.select(game, ctx, look)?);
        }
        Ok(())
    }

    fn seal_original(
        &mut self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<(), ExecutionError> {
        self.prepare_original(game, ctx)?;
        if !self.disclosed
            && !ctx.decision_maker.awaiting_choice()
            && let Some(selection) = &mut self.selection
        {
            let token =
                begin_opposition_agent_search_control(game, self.request.chooser, self.policy);
            let result = self.action.disclose(selection, game, ctx);
            if ctx.decision_maker.awaiting_choice() {
                game.capture_pending_decision_controllers();
            }
            finish_opposition_agent_search_control(game, token);
            result?;
            self.disclosed = true;
        }
        Ok(())
    }

    fn commit_original_with_outputs(
        mut self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit<CompletedEffectOutputs>, ExecutionError> {
        self.seal_original(game, ctx)?;
        if ctx.decision_maker.awaiting_choice() {
            return Ok(SimultaneousEffectCommit::finished(
                CompletedEffectOutputs::aggregate_only(EffectOutcome::count(0)),
            ));
        }
        let selection = self.selection.take().ok_or_else(|| {
            ExecutionError::InternalError("missing prepared library search".into())
        })?;
        let token = begin_opposition_agent_search_control(game, self.request.chooser, self.policy);
        let result = self.action.finish(selection, game, ctx);
        if ctx.decision_maker.awaiting_choice() {
            game.capture_pending_decision_controllers();
        }
        finish_opposition_agent_search_control(game, token);
        result
    }
    fn commit_original(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<SimultaneousEffectCommit, ExecutionError> {
        self.commit_original_with_outputs(game, ctx)
            .map(SimultaneousEffectCommit::into_aggregate)
    }
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let receipt = self.commit_original_with_outputs(game, ctx)?;
        crate::effects::composition::complete_committed_original_with_outputs(game, ctx, receipt)
            .map(CompletedEffectOutputs::into_outcome)
    }
}
