//! "Becomes the color of your choice" effect.
//!
//! Used for cards like Swirling Spriggan:
//! "{1}: Target creature becomes the color of your choice until end of turn."

use crate::color::Color;
use crate::continuous::Modification;
#[cfg(test)]
use crate::decisions::context::SelectOptionsContext;
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
        self.execute_with_outputs(game, ctx)
            .map(crate::effects::CompletedEffectOutputs::into_outcome)
    }
    fn execute_with_outputs(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<crate::effects::CompletedEffectOutputs, ExecutionError> {
        if let Some(reference) = self.color_from {
            // "becomes that color": read the exact earlier local color
            // decision (a chosen color, or the single color of mana just
            // added). No new choice is made here.
            let ironsmith_core::ColorChoiceReference::Effect(color_id) = reference else {
                return Err(ExecutionError::UnresolvableValue(
                    "unbound local color choice".into(),
                ));
            };
            let colors = ctx
                .get_outcome(color_id)
                .map(|outcome| {
                    outcome
                        .instruction_result()
                        .execution_facts
                        .iter()
                        .filter_map(|fact| match fact {
                            crate::effect::ExecutionFact::ChosenColor(color) => Some(*color),
                            _ => None,
                        })
                        .collect::<crate::color::ColorSet>()
                })
                .unwrap_or_default();
            if colors.is_empty() {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                    EffectOutcome::count(0),
                ));
            }
            return crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                self.target.clone(),
                Modification::SetColors(colors),
                self.duration.clone(),
            ))
            .0
            .execute_child_with_outputs(game, ctx);
        }
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let options = color_options()
            .into_iter()
            .map(|(color, label)| (color, label.to_string()))
            .collect::<Vec<_>>();
        crate::effects::composition::scoped_choice::execute_choice_program(
            game,
            ctx,
            chooser,
            if self.allow_multiple {
                "Choose one or more colors"
            } else {
                "Choose a color"
            },
            &options,
            if self.allow_multiple {
                options.len()
            } else {
                1
            },
            false,
            |colors| {
                let colors = colors.into_iter().collect::<crate::color::ColorSet>();
                Ok((!colors.is_empty()).then(|| {
                    crate::effect::Effect::new(crate::effects::ApplyContinuousEffect::with_spec(
                        self.target.clone(),
                        Modification::SetColors(colors),
                        self.duration.clone(),
                    ))
                }))
            },
        )
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
