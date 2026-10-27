//! "Choose a creature type ... becomes that type" effect.

use crate::continuous::Modification;
#[cfg(test)]
use crate::decisions::context::SelectOptionsContext;
use crate::effect::EffectOutcome;
use crate::effects::{ApplyContinuousEffect, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::types::{Subtype, SubtypeFamily};

/// Effect: target object(s) become a chosen creature type.
pub type BecomeCreatureTypeChoiceEffect = ironsmith_core::BecomeCreatureTypeChoiceEffect;

fn creature_type_options(effect: &BecomeCreatureTypeChoiceEffect) -> Vec<Subtype> {
    all_creature_types()
        .iter()
        .copied()
        .filter(|subtype| !effect.excluded_subtypes.contains(subtype))
        .collect()
}

pub(crate) fn all_creature_types() -> &'static [Subtype] {
    Subtype::all_creature_types()
}

impl EffectExecutor for BecomeCreatureTypeChoiceEffect {
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
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let options = creature_type_options(self)
            .into_iter()
            .map(|kind| (kind, kind.to_string()))
            .collect::<Vec<_>>();
        if options.is_empty() {
            return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(
                EffectOutcome::resolved(),
            ));
        }
        crate::effects::composition::scoped_choice::execute_choice_program(
            game,
            ctx,
            chooser,
            "Choose a creature type",
            &options,
            1,
            false,
            |selected| {
                Ok(selected.first().map(|kind| {
                    if self.in_addition_to_other_types {
                        return crate::effect::Effect::new(
                            ApplyContinuousEffect::with_spec(
                                self.target.clone(),
                                Modification::AddSubtypes(vec![*kind]),
                                self.duration.clone(),
                            )
                            .require_creature_target(),
                        );
                    }
                    crate::effect::Effect::new(
                        ApplyContinuousEffect::with_spec(
                            self.target.clone(),
                            Modification::RemoveAllSubtypesOfFamily(SubtypeFamily::Creature),
                            self.duration.clone(),
                        )
                        .with_additional_modification(Modification::AddSubtypes(vec![*kind]))
                        .require_creature_target(),
                    )
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
    use crate::ids::PlayerId;
    use crate::target::ObjectFilter;
    use crate::test_prelude::*;
    use crate::zone::Zone;

    struct ChooseZombieDm;
    impl DecisionMaker for ChooseZombieDm {
        fn decide_options(&mut self, _game: &GameState, ctx: &SelectOptionsContext) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.description.eq_ignore_ascii_case("zombie"))
                .map(|option| vec![option.index])
                .unwrap_or_else(|| vec![0])
        }
    }

    #[test]
    fn become_creature_type_choice_replaces_creature_subtype() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_def = grizzly_bears();
        let creature_id =
            game.create_object_from_definition(&creature_def, alice, Zone::Battlefield);

        let source = game.new_object_id();
        let mut dm = ChooseZombieDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = BecomeCreatureTypeChoiceEffect::new(
            ChooseSpec::SpecificObject(creature_id),
            Until::EndOfTurn,
            vec![],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("become-creature-type-choice should execute");

        let subtypes = game.calculated_subtypes(creature_id);
        assert!(
            subtypes.contains(&Subtype::Zombie),
            "expected target creature to have Zombie subtype, got {subtypes:?}"
        );
        assert!(
            !subtypes.contains(&Subtype::Bear),
            "expected original Bear subtype to be replaced, got {subtypes:?}"
        );
    }

    #[test]
    fn become_creature_type_choice_applies_to_all_creatures() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let first = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);
        let second = game.create_object_from_definition(&grizzly_bears(), alice, Zone::Battlefield);

        let source = game.new_object_id();
        let mut dm = ChooseZombieDm;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = BecomeCreatureTypeChoiceEffect::new(
            ChooseSpec::All(ObjectFilter::creature()),
            Until::EndOfTurn,
            vec![],
        );

        effect
            .execute(&mut game, &mut ctx)
            .expect("become-creature-type-choice should apply to all creatures");

        for creature in [first, second] {
            let subtypes = game.calculated_subtypes(creature);
            assert!(
                subtypes.contains(&Subtype::Zombie),
                "expected target creature to have Zombie subtype, got {subtypes:?}"
            );
            assert!(
                !subtypes.contains(&Subtype::Bear),
                "expected original Bear subtype to be replaced, got {subtypes:?}"
            );
        }
    }
}
