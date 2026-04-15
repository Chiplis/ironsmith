//! Choose a creature type and store it on the source object.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_player_filter;
use crate::effects::{BecomeCreatureTypeChoiceEffect, EffectExecutor};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::types::Subtype;

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCreatureTypeEffect {
    pub chooser: PlayerFilter,
    pub excluded_subtypes: Vec<Subtype>,
}

impl ChooseCreatureTypeEffect {
    pub fn new(chooser: PlayerFilter, excluded_subtypes: Vec<Subtype>) -> Self {
        Self {
            chooser,
            excluded_subtypes,
        }
    }

    fn creature_type_options(&self) -> Vec<Subtype> {
        BecomeCreatureTypeChoiceEffect::all_creature_types()
            .iter()
            .copied()
            .filter(|subtype| !self.excluded_subtypes.contains(subtype))
            .collect()
    }
}

impl EffectExecutor for ChooseCreatureTypeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let subtype_options = self.creature_type_options();
        if subtype_options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let options: Vec<SelectableOption> = subtype_options
            .iter()
            .enumerate()
            .map(|(idx, subtype)| SelectableOption::new(idx, subtype.to_string()))
            .collect();
        let choice_ctx = SelectOptionsContext::new(
            chooser,
            Some(ctx.source),
            "Choose a creature type",
            options,
            1,
            1,
        );
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(chosen) = selected
            .into_iter()
            .next()
            .filter(|idx| *idx < subtype_options.len())
        else {
            return Ok(EffectOutcome::count(0));
        };

        game.set_chosen_creature_type(ctx.source, subtype_options[chosen]);
        Ok(EffectOutcome::count(1))
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
}
