//! Choose a creature type and store it on the source object.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::target::PlayerFilter;
use crate::types::{Subtype, SubtypeFamily};

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCreatureTypeEffect {
    pub chooser: PlayerFilter,
    pub excluded_subtypes: Vec<Subtype>,
    pub family: SubtypeFamily,
}

impl ChooseCreatureTypeEffect {
    pub fn new(chooser: PlayerFilter, excluded_subtypes: Vec<Subtype>) -> Self {
        Self {
            chooser,
            excluded_subtypes,
            family: SubtypeFamily::Creature,
        }
    }

    pub fn for_family(chooser: PlayerFilter, family: SubtypeFamily) -> Self {
        Self {
            chooser,
            excluded_subtypes: Vec::new(),
            family,
        }
    }

    fn subtype_options(&self) -> Vec<Subtype> {
        self.family
            .all_subtypes()
            .iter()
            .copied()
            .filter(|subtype| !self.excluded_subtypes.contains(subtype))
            .collect()
    }

    fn choose_subtype(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Option<Subtype>, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let subtype_options = self.subtype_options();
        if subtype_options.is_empty() {
            return Ok(None);
        }

        let options: Vec<SelectableOption> = subtype_options
            .iter()
            .enumerate()
            .map(|(idx, subtype)| SelectableOption::new(idx, subtype.to_string()))
            .collect();
        let choice_ctx = SelectOptionsContext::new(
            chooser,
            Some(ctx.source),
            format!("Choose a {}", self.family.type_phrase()),
            options,
            1,
            1,
        );
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(None);
        }
        Ok(selected
            .into_iter()
            .next()
            .and_then(|idx| subtype_options.get(idx).copied()))
    }
}

#[derive(Debug)]
struct ChooseSubtypeProposal {
    source: ObjectId,
    subtype: Option<Subtype>,
}

impl crate::effects::SimultaneousEffectProposal for ChooseSubtypeProposal {
    fn commit(
        self: Box<Self>,
        game: &mut GameState,
        _ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(subtype) = self.subtype else {
            return Ok(EffectOutcome::count(0));
        };
        game.set_chosen_subtype(self.source, subtype);
        Ok(EffectOutcome::count(1))
    }
}

impl EffectExecutor for ChooseCreatureTypeEffect {
    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let Some(chosen) = self.choose_subtype(game, ctx)? else {
            return Ok(EffectOutcome::count(0));
        };
        game.set_chosen_subtype(ctx.source, chosen);
        Ok(EffectOutcome::count(1))
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Collect every player's type choice against the pre-action state in
        // APNAP order, then accumulate the selected types atomically on the
        // shared source during the ForPlayers commit phase.
        Ok(Box::new(ChooseSubtypeProposal {
            source: ctx.source,
            subtype: self.choose_subtype(game, ctx)?,
        }))
    }
}

impl CostExecutableEffect for ChooseCreatureTypeEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::ids::{ObjectId, PlayerId};

    struct ChooseZombieDm;
    impl DecisionMaker for ChooseZombieDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("zombie"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    struct ChooseJaceDm;
    impl DecisionMaker for ChooseJaceDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("jace"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    #[test]
    fn choose_creature_type_effect_stores_selected_type_on_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::new();
        let mut dm = ChooseZombieDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        ChooseCreatureTypeEffect::new(PlayerFilter::You, vec![])
            .execute(&mut game, &mut ctx)
            .expect("choose-creature-type should execute");

        assert_eq!(game.chosen_creature_type(source), Some(Subtype::Zombie));
    }

    #[test]
    fn choose_planeswalker_type_effect_uses_the_family_options_and_generic_store() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::new();
        let mut dm = ChooseJaceDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        ChooseCreatureTypeEffect::for_family(PlayerFilter::You, SubtypeFamily::Planeswalker)
            .execute(&mut game, &mut ctx)
            .expect("choose-planeswalker-type should execute");

        assert_eq!(game.chosen_subtype(source), Some(Subtype::Jace));
    }
}
