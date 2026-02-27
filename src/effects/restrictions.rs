//! Effects that apply rule restrictions ("can't" effects).

use crate::effect::{EffectOutcome, Restriction, Until};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;

/// Effect that applies a restriction for a duration.
#[derive(Debug, Clone, PartialEq)]
pub struct CantEffect {
    pub restriction: Restriction,
    pub duration: Until,
}

impl CantEffect {
    pub fn new(restriction: Restriction, duration: Until) -> Self {
        Self {
            restriction,
            duration,
        }
    }

    pub fn until_end_of_turn(restriction: Restriction) -> Self {
        Self::new(restriction, Until::EndOfTurn)
    }
}

impl EffectExecutor for CantEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        if matches!(self.duration, Until::ControllersNextUntapStep)
            && let Restriction::Untap(filter) = &self.restriction
        {
            let filter_ctx = ctx.filter_context(game);
            let targets: Vec<_> = game
                .battlefield
                .iter()
                .filter_map(|object_id| {
                    let obj = game.object(*object_id)?;
                    if filter.matches(obj, &filter_ctx, game) {
                        Some((*object_id, obj.controller))
                    } else {
                        None
                    }
                })
                .collect();

            if !targets.is_empty() {
                for (object_id, controller) in targets {
                    game.add_restriction_effect(
                        Restriction::untap(crate::target::ObjectFilter::specific(object_id)),
                        self.duration.clone(),
                        ctx.source,
                        controller,
                    );
                }
            } else {
                game.add_restriction_effect(
                    self.restriction.clone(),
                    self.duration.clone(),
                    ctx.source,
                    ctx.controller,
                );
            }
        } else {
            game.add_restriction_effect(
                self.restriction.clone(),
                self.duration.clone(),
                ctx.source,
                ctx.controller,
            );
        }
        game.update_cant_effects();
        Ok(EffectOutcome::resolved())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effects::RegenerateEffect;
    use crate::executor::ExecutionContext;
    use crate::game_state::GameState;
    use crate::ids::CardId;
    use crate::ids::PlayerId;
    use crate::target::{ObjectFilter, PlayerFilter};
    use crate::types::CardType;
    use crate::zone::Zone;

    #[test]
    fn cant_effect_blocks_life_gain() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = CantEffect::until_end_of_turn(Restriction::gain_life(PlayerFilter::Any));
        effect.execute(&mut game, &mut ctx).expect("execute cant");

        game.update_cant_effects();

        assert!(!game.can_gain_life(PlayerId::from_index(0)));
        assert!(!game.can_gain_life(PlayerId::from_index(1)));
    }

    #[test]
    fn cant_be_regenerated_clears_existing_regeneration_shields() {
        let mut game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let alice = PlayerId::from_index(0);

        let creature_card = CardBuilder::new(CardId::from_raw(1), "Shielded Bear")
            .card_types(vec![CardType::Creature])
            .build();
        let creature_id = game.create_object_from_card(&creature_card, alice, Zone::Battlefield);

        let mut regen_ctx = ExecutionContext::new_default(creature_id, alice);
        RegenerateEffect::source(Until::EndOfTurn)
            .execute(&mut game, &mut regen_ctx)
            .expect("apply regeneration shield");
        assert!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id)
                > 0
        );

        let source = game.new_object_id();
        let mut cant_ctx = ExecutionContext::new_default(source, alice);
        CantEffect::until_end_of_turn(Restriction::be_regenerated(ObjectFilter::specific(
            creature_id,
        )))
        .execute(&mut game, &mut cant_ctx)
        .expect("apply cant be regenerated");

        assert!(!game.can_be_regenerated(creature_id));
        assert_eq!(
            game.replacement_effects
                .count_one_shot_effects_from_source(creature_id),
            0
        );
    }
}
