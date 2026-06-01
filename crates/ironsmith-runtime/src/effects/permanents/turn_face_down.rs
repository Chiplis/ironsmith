//! Turn permanents face down.

use crate::effect::EffectOutcome;
use crate::effects::helpers::{ObjectApplyResultPolicy, apply_to_selected_objects};
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
use crate::types::CardType;
use crate::{PtValue, zone::Zone};

pub use ironsmith_core::TurnFaceDownEffect;

impl EffectExecutor for TurnFaceDownEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if self.target.is_target() && self.target.count().min == 0 && ctx.targets.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let apply_result = apply_to_selected_objects(
            game,
            ctx,
            &self.target,
            ObjectApplyResultPolicy::CountApplied,
            |game, _ctx, object_id| {
                {
                    let Some(object) = game.object_mut(object_id) else {
                        return Ok(false);
                    };
                    if object.zone != Zone::Battlefield {
                        return Ok(false);
                    }

                    object.apply_face_down_cast_overlay();
                    object.card_types = vec![CardType::Creature];
                    object.subtypes = self.subtypes.clone();
                    object.base_power = Some(PtValue::Fixed(self.power));
                    object.base_toughness = Some(PtValue::Fixed(self.toughness));
                }
                game.set_face_down(object_id);
                Ok(true)
            },
        )?;

        Ok(apply_result.outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn get_target_count(&self) -> Option<crate::effect::ChoiceCount> {
        if self.target.is_target() {
            Some(self.target.count())
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        "permanent to turn face down"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::target::ObjectFilter;
    use crate::types::Subtype;

    fn add_creature(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        tapped: bool,
        token: bool,
    ) -> crate::ids::ObjectId {
        let id = game.new_object_id();
        let mut builder = CardBuilder::new(CardId::from_raw(id.0 as u32), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(3, 3));
        if token {
            builder = builder.token();
        }
        let object = Object::from_card(id, &builder.build(), controller, Zone::Battlefield);
        game.add_object(object);
        if tapped {
            game.tap(id);
        }
        id
    }

    #[test]
    fn turns_selected_permanent_face_down_with_characteristics() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let target = add_creature(&mut game, "Illithid Target", alice, true, false);
        let source = game.new_object_id();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Object(target)]);

        TurnFaceDownEffect::target(ChooseSpec::creature())
            .with_characteristics(2, 2, vec![Subtype::Horror])
            .execute(&mut game, &mut ctx)
            .expect("Illithid-style turn-face-down effect should resolve");

        assert!(game.is_face_down(target));
        assert_eq!(game.calculated_power(target), Some(2));
        assert_eq!(game.calculated_toughness(target), Some(2));
        assert!(game.calculated_subtypes(target).contains(&Subtype::Horror));
    }

    #[test]
    fn any_number_zero_targets_resolves_without_touching_creatures() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let untouched = add_creature(&mut game, "Illithid Nonchoice", alice, true, false);
        let source = game.new_object_id();
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        TurnFaceDownEffect::targets(
            ChooseSpec::Object(ObjectFilter::creature()),
            crate::effect::ChoiceCount::any_number(),
        )
        .with_characteristics(2, 2, vec![Subtype::Horror])
        .execute(&mut game, &mut ctx)
        .expect("Illithid-style any-number zero-target branch should resolve");

        assert!(!game.is_face_down(untouched));
    }
}
