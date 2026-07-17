//! "Becomes the color of your choice" effect.
//!
//! Used for cards like Swirling Spriggan:
//! "{1}: Target creature becomes the color of your choice until end of turn."

use crate::color::Color;
use crate::continuous::Modification;
use crate::decisions::context::{SelectOptionsContext, SelectableOption};
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Effect: target permanent becomes one color of the chooser's choice.
pub type BecomeColorChoiceEffect = ironsmith_core::BecomeColorChoiceEffect;

fn color_options() -> [(Color, &'static str); 5] {
    [
        (Color::White, "White"),
        (Color::Blue, "Blue"),
        (Color::Black, "Black"),
        (Color::Red, "Red"),
        (Color::Green, "Green"),
    ]
}

impl EffectExecutor for BecomeColorChoiceEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;

        let options: Vec<SelectableOption> = color_options()
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
            .next();
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let Some(chosen) = chosen.filter(|idx| *idx < color_options().len()) else {
            return Ok(EffectOutcome::count(0));
        };

        let (color, _) = color_options()[chosen];
        let apply = crate::effects::ApplyContinuousEffect::with_spec(
            self.target.clone(),
            Modification::SetColors(crate::color::ColorSet::from_color(color)),
            self.duration.clone(),
        );

        apply.execute(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cards::definitions::grizzly_bears;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectOptionsContext;
    use crate::ids::{ObjectId, PlayerId};
    use crate::test_prelude::*;
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
