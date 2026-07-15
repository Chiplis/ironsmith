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
        // CR 701.48a offers the discard first. Only a player who did not
        // actually discard a card gets the later Lesson choice.
        let discard_outcome = execute_effect(
            game,
            &Effect::new(MayEffect::single(Effect::discard(1))),
            ctx,
        )?;
        if discard_outcome.count_or_zero() > 0 {
            let draw_outcome = execute_effect(game, &Effect::draw(1), ctx)?;
            return Ok(EffectOutcome::aggregate([discard_outcome, draw_outcome]));
        }

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
            let lesson_outcome = execute_effect(game, &Effect::new(reveal_and_put), ctx)?;
            return Ok(EffectOutcome::aggregate([
                discard_outcome,
                choose_outcome,
                lesson_outcome,
            ]));
        }

        Ok(EffectOutcome::aggregate([discard_outcome, choose_outcome]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::{CardType, Subtype};

    fn card(name: &str, card_type: CardType) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![card_type])
            .build()
    }

    fn lesson(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Sorcery])
            .subtypes(vec![Subtype::Lesson])
            .build()
    }

    fn has_named_object_in_zone(game: &GameState, zone: Zone, name: &str) -> bool {
        game.objects_in_zone(zone)
            .into_iter()
            .any(|id| game.object(id).is_some_and(|object| object.name == name))
    }

    #[derive(Default)]
    struct DeclineDiscardThenChooseLesson {
        prompts: Vec<&'static str>,
    }

    impl DecisionMaker for DeclineDiscardThenChooseLesson {
        fn decide_boolean(
            &mut self,
            _game: &GameState,
            _ctx: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.prompts.push("discard");
            false
        }

        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectObjectsContext,
        ) -> Vec<ObjectId> {
            self.prompts.push("lesson");
            ctx.candidates
                .iter()
                .find(|candidate| candidate.legal)
                .map(|candidate| vec![candidate.id])
                .unwrap_or_default()
        }
    }

    #[test]
    fn learn_discards_then_draws_before_offering_a_lesson() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.create_object_from_card(&card("Discard Me", CardType::Instant), alice, Zone::Hand);
        game.create_object_from_card(&card("Draw Me", CardType::Instant), alice, Zone::Library);
        game.create_object_from_card(&lesson("Environmental Sciences"), alice, Zone::OutsideGame);
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);

        LearnEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("learn should execute");

        assert!(has_named_object_in_zone(
            &game,
            Zone::Graveyard,
            "Discard Me"
        ));
        assert!(has_named_object_in_zone(&game, Zone::Hand, "Draw Me"));
        assert!(has_named_object_in_zone(
            &game,
            Zone::OutsideGame,
            "Environmental Sciences"
        ));
    }

    #[test]
    fn learn_offers_a_lesson_only_after_discard_is_declined() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        game.create_object_from_card(&card("Keep Me", CardType::Instant), alice, Zone::Hand);
        game.create_object_from_card(&lesson("Expanded Anatomy"), alice, Zone::OutsideGame);
        let source = game.new_object_id();
        let mut dm = DeclineDiscardThenChooseLesson::default();
        let mut ctx = ExecutionContext::new_default(source, alice).with_decision_maker(&mut dm);

        LearnEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("learn should execute");

        assert_eq!(dm.prompts, vec!["discard", "lesson"]);
        assert!(has_named_object_in_zone(&game, Zone::Hand, "Keep Me"));
        assert!(has_named_object_in_zone(
            &game,
            Zone::Hand,
            "Expanded Anatomy"
        ));
    }

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
