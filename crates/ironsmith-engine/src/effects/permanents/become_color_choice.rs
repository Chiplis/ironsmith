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

/// Effect: target permanent becomes one or more colors of the chooser's choice.
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
        let maximum = if self.allow_multiple {
            options.len()
        } else {
            1
        };
        let choice_ctx = SelectOptionsContext::new(
            chooser,
            Some(ctx.source),
            if self.allow_multiple {
                "Choose one or more colors"
            } else {
                "Choose a color"
            },
            options,
            1,
            maximum,
        );
        let chosen = ctx.decision_maker.decide_options(game, &choice_ctx);
        if ctx.decision_maker.awaiting_choice() {
            return Ok(EffectOutcome::count(0));
        }
        let colors = chosen
            .into_iter()
            .filter_map(|index| color_options().get(index).map(|(color, _)| *color))
            .collect::<crate::color::ColorSet>();
        if colors.is_empty() {
            return Ok(EffectOutcome::count(0));
        }
        let apply = crate::effects::ApplyContinuousEffect::with_spec(
            self.target.clone(),
            Modification::SetColors(colors),
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

    struct ChooseWhiteAndBlueDm;
    impl DecisionMaker for ChooseWhiteAndBlueDm {
        fn decide_options(&mut self, _game: &GameState, _ctx: &SelectOptionsContext) -> Vec<usize> {
            vec![0, 1]
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

    #[test]
    fn become_color_choice_can_set_multiple_colors() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let creature_id =
            game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);

        let mut dm = ChooseWhiteAndBlueDm;
        let mut ctx = ExecutionContext::new(ObjectId::from_raw(9999), alice, &mut dm);
        let effect =
            BecomeColorChoiceEffect::new(ChooseSpec::SpecificObject(creature_id), Until::EndOfTurn)
                .with_multiple_colors(true);

        effect
            .execute(&mut game, &mut ctx)
            .expect("multi-color choice should execute");

        assert_eq!(
            game.calculated_characteristics(creature_id)
                .expect("calculated characteristics")
                .colors,
            crate::color::ColorSet::WHITE.union(crate::color::ColorSet::BLUE),
        );
    }
}
