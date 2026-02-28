//! "Becomes the color of your choice" effect.
//!
//! Used for cards like Swirling Spriggan:
//! "{1}: Target creature becomes the color of your choice until end of turn."

use crate::color::Color;
use crate::continuous::Modification;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::{EffectOutcome, Until};
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_player_filter;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, PlayerFilter};

/// Effect: target permanent becomes one color of the chooser's choice.
#[derive(Debug, Clone, PartialEq)]
pub struct BecomeColorChoiceEffect {
    pub target: ChooseSpec,
    pub until: Until,
    pub chooser: PlayerFilter,
}

impl BecomeColorChoiceEffect {
    pub fn new(target: ChooseSpec, until: Until) -> Self {
        Self {
            target,
            until,
            chooser: PlayerFilter::You,
        }
    }

    pub fn with_chooser(mut self, chooser: PlayerFilter) -> Self {
        self.chooser = chooser;
        self
    }

    fn color_options() -> [(Color, &'static str); 5] {
        [
            (Color::White, "White"),
            (Color::Blue, "Blue"),
            (Color::Black, "Black"),
            (Color::Red, "Red"),
            (Color::Green, "Green"),
        ]
    }
}

impl EffectExecutor for BecomeColorChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser = resolve_player_filter(game, &self.chooser, ctx)?;

        let options: Vec<SelectableOption> = Self::color_options()
            .iter()
            .enumerate()
            .map(|(idx, (_, label))| SelectableOption::new(idx, *label))
            .collect();
        let choice_ctx =
            SelectOptionsContext::new(chooser, Some(ctx.source), "Choose a color", options, 1, 1);
        let chosen = ctx
            .decision_maker
            .decide_options(game, &choice_ctx)
            .into_iter()
            .next()
            .unwrap_or(0);

        let (color, _) = Self::color_options()[chosen.min(4)];
        let apply = crate::effects::ApplyContinuousEffect::with_spec(
            self.target.clone(),
            Modification::SetColors(crate::color::ColorSet::from_color(color)),
            self.until.clone(),
        );

        apply.execute(game, ctx)
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::definitions::grizzly_bears;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectOptionsContext;
    use crate::ids::{ObjectId, PlayerId};
    use crate::zone::Zone;

    struct ChooseRedDm;
    impl DecisionMaker for ChooseRedDm {
        fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
            // Red option index in color_options().
            vec![3]
        }
    }

    #[test]
    fn become_color_choice_sets_target_color_until_eot() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_def = grizzly_bears();
        let creature_id =
            game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);

        let source = ObjectId::from_raw(9999);
        let mut dm = ChooseRedDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect =
            BecomeColorChoiceEffect::new(ChooseSpec::SpecificObject(creature_id), Until::EndOfTurn);

        effect
            .execute(&mut game, &mut ctx)
            .expect("become-color-choice should execute");

        let colors = game
            .calculated_characteristics(creature_id)
            .expect("calculated characteristics")
            .colors;
        assert_eq!(
            colors,
            crate::color::ColorSet::RED,
            "expected target creature to become red"
        );
    }
}
