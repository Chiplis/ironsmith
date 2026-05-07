//! Learn keyword action implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{
    ChooseObjectsEffect, EffectExecutor, ExecutionContext, ExecutionError, MayEffect,
    SequenceEffect, execute_effect,
};
use crate::game_state::GameState;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::types::Subtype;
use crate::zone::Zone;

pub type LearnEffect = ironsmith_core::LearnEffect;

const LEARN_LESSON_TAG: &str = "__learn_lesson";

impl EffectExecutor for LearnEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let lesson_filter = ObjectFilter::default()
            .with_subtype(Subtype::Lesson)
            .owned_by(PlayerFilter::You)
            .in_zone(Zone::OutsideGame);

        let choose_lesson = Effect::new(
            ChooseObjectsEffect::new(lesson_filter, 1, PlayerFilter::You, LEARN_LESSON_TAG)
                .as_optional_search()
                .in_zone(Zone::OutsideGame),
        );
        let choose_outcome = execute_effect(game, &choose_lesson, ctx)?;
        if choose_outcome
            .objects()
            .is_some_and(|objects| !objects.is_empty())
        {
            let reveal_and_put = SequenceEffect::new(vec![
                Effect::new(crate::effects::RevealTaggedEffect::new(LEARN_LESSON_TAG)),
                Effect::move_to_zone(ChooseSpec::tagged(LEARN_LESSON_TAG), Zone::Hand, false),
            ]);
            return execute_effect(game, &Effect::new(reveal_and_put), ctx);
        }

        let rummage = MayEffect::new(vec![Effect::discard(1), Effect::draw(1)]);
        execute_effect(game, &Effect::new(rummage), ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::types::{CardType, Subtype};

    #[test]
    fn learn_can_reveal_lesson_from_outside_game_and_put_it_into_hand() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let lesson = CardBuilder::new(CardId::from_raw(99), "Intro to Annihilation")
            .card_types(vec![CardType::Sorcery])
            .subtypes(vec![Subtype::Lesson])
            .build();
        game.create_object_from_card(&lesson, alice, Zone::OutsideGame);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        LearnEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("learn should execute");

        let hand = game.player(alice).expect("alice exists").hand.clone();
        assert!(hand.iter().any(|object_id| {
            game.object(*object_id).is_some_and(|object| {
                object.zone == Zone::Hand && object.name == "Intro to Annihilation"
            })
        }));
    }
}
