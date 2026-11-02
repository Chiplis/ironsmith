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

fn creature_type_options(game: &GameState, effect: &BecomeCreatureTypeChoiceEffect) -> Vec<Subtype> {
    game.subtypes_in_family(SubtypeFamily::Creature)
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
        if self.use_source_chosen_type && self.use_source_chosen_card_type {
            let Some(kind) = game.chosen_card_type(ctx.source).or_else(||
                ctx.source_snapshot.as_ref().and_then(|snapshot| snapshot.chosen_card_type)) else {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved()));
            };
            let modification = if self.in_addition_to_other_types {
                Modification::AddCardTypes(vec![kind])
            } else {
                Modification::SetCardTypes(vec![kind])
            };
            return ApplyContinuousEffect::with_spec(self.target.clone(), modification, self.duration.clone())
                .execute_with_outputs(game, ctx);
        }
        if self.use_source_chosen_type {
            let Some(kind)=game.chosen_creature_type(ctx.source).or_else(|| ctx.source_snapshot.as_ref().and_then(|snapshot|snapshot.chosen_subtype)) else {
                return Ok(crate::effects::CompletedEffectOutputs::aggregate_only(EffectOutcome::resolved()));
            };
            let modification=if self.in_addition_to_other_types {
                ApplyContinuousEffect::with_spec(self.target.clone(),Modification::AddSubtypes(vec![kind]),self.duration.clone())
            } else {
                ApplyContinuousEffect::with_spec(self.target.clone(),Modification::RemoveAllSubtypesOfFamily(SubtypeFamily::Creature),self.duration.clone())
                    .with_additional_modification(Modification::AddSubtypes(vec![kind]))
            }.require_creature_target();
            return modification.execute_with_outputs(game,ctx);
        }
        let chooser =
            crate::effects::helpers::resolve_player_filter_as_chooser(game, &self.chooser, ctx)?;
        let options = creature_type_options(game, self)
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

#[cfg(test)]
mod prior_chosen_type_tests {
    use super::*;
    #[test]
    fn prior_choice_is_snapshotted_and_additive_instead_of_rechosen() {
        let mut game=GameState::new(vec!["A".into(),"B".into()],20);
        let player=crate::ids::PlayerId(0);
        let id=game.create_object_from_definition(&crate::cards::definitions::grizzly_bears(),player,crate::Zone::Battlefield);
        game.set_chosen_creature_type(id,Subtype::Zombie);
        let mut effect=BecomeCreatureTypeChoiceEffect::new(crate::target::ChooseSpec::SpecificObject(id),crate::Until::EndOfTurn,Vec::new()).in_addition_to_other_types(true);
        effect.use_source_chosen_type=true;
        effect.execute(&mut game,&mut ExecutionContext::new_default(id,player)).unwrap();
        game.set_chosen_creature_type(id,Subtype::Elf);
        let types=game.calculated_subtypes(id);
        assert!(types.contains(&Subtype::Bear)&&types.contains(&Subtype::Zombie));
        assert!(!types.contains(&Subtype::Elf),"later choices cannot rewrite completed modifications");
    }
}

#[cfg(test)]
mod prior_card_type_tests {
    use super::*;
    #[test]
    fn chosen_card_type_applies_to_noncreatures_and_does_not_read_stale_creature_choice() {
        let mut game = GameState::new(vec!["A".into(), "B".into()], 20);
        let player = crate::ids::PlayerId(0);
        let source = game.create_object_from_definition(&crate::cards::definitions::grizzly_bears(), player, crate::Zone::Battlefield);
        let target = game.create_object_from_definition(&crate::cards::definitions::basic_forest(), player, crate::Zone::Battlefield);
        game.set_chosen_creature_type(source, Subtype::Zombie);
        game.set_chosen_card_type(source, crate::types::CardType::Artifact);
        let mut effect = BecomeCreatureTypeChoiceEffect::new(crate::target::ChooseSpec::SpecificObject(target), crate::Until::EndOfTurn, Vec::new()).in_addition_to_other_types(true);
        effect.use_source_chosen_type = true;
        effect.use_source_chosen_card_type = true;
        effect.execute(&mut game, &mut ExecutionContext::new_default(source, player)).unwrap();
        game.set_chosen_card_type(source, crate::types::CardType::Enchantment);
        let types = game.current_card_types(target).unwrap();
        assert!(types.contains(&crate::types::CardType::Land));
        assert!(types.contains(&crate::types::CardType::Artifact));
        assert!(!types.contains(&crate::types::CardType::Enchantment));
        assert!(!game.calculated_subtypes(target).contains(&Subtype::Zombie));
    }
}
