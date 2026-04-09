//! Choose a card type and store it on the source object.

use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::types::CardType;

#[derive(Debug, Clone, PartialEq)]
pub struct ChooseCardTypeEffect {
    pub chooser: PlayerFilter,
    pub options: Vec<CardType>,
}

impl ChooseCardTypeEffect {
    pub fn new(chooser: PlayerFilter, options: Vec<CardType>) -> Self {
        Self { chooser, options }
    }

    pub fn all_card_types() -> &'static [CardType] {
        &[
            CardType::Artifact,
            CardType::Battle,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Instant,
            CardType::Kindred,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Sorcery,
        ]
    }

    fn card_type_options(&self) -> Vec<CardType> {
        if self.options.is_empty() {
            Self::all_card_types().to_vec()
        } else {
            self.options.clone()
        }
    }
}

impl EffectExecutor for ChooseCardTypeEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;
        let options = self.card_type_options();
        if options.is_empty() {
            return Ok(EffectOutcome::resolved());
        }

        let display_options = options
            .iter()
            .enumerate()
            .map(|(idx, card_type)| SelectableOption::new(idx, card_type.to_string()))
            .collect::<Vec<_>>();
        let permanent_type_options = [
            CardType::Artifact,
            CardType::Creature,
            CardType::Enchantment,
            CardType::Land,
            CardType::Planeswalker,
            CardType::Battle,
        ];
        let prompt = if options == permanent_type_options {
            "Choose a permanent type"
        } else {
            "Choose a card type"
        };
        let choice_ctx =
            SelectOptionsContext::new(chooser, Some(ctx.source), prompt, display_options, 1, 1);
        let selected = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(chosen) = selected
            .into_iter()
            .next()
            .filter(|idx| *idx < options.len())
        else {
            return Ok(EffectOutcome::count(0));
        };

        game.set_chosen_card_type(ctx.source, options[chosen]);
        Ok(EffectOutcome::count(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::ids::{ObjectId, PlayerId};

    struct ChooseLandDm;
    impl DecisionMaker for ChooseLandDm {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("land"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    #[test]
    fn choose_card_type_effect_stores_selected_type_on_source() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = ObjectId::new();
        let mut dm = ChooseLandDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        ChooseCardTypeEffect::new(PlayerFilter::You, vec![CardType::Creature, CardType::Land])
            .execute(&mut game, &mut ctx)
            .expect("choose-card-type should execute");

        assert_eq!(game.chosen_card_type(source), Some(CardType::Land));
    }
}
