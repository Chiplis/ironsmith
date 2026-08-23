//! Learn keyword action implementation.

use crate::effect::{Effect, EffectOutcome};
use crate::effects::{
    ChooseObjectsEffect, EffectExecutor, ExecutionContext, ExecutionError, MayEffect,
    SequenceEffect, execute_effect,
};
use crate::events::processing::{
    TraitEventResult, process_trait_event_with_dm_and_applied_effects,
};
use crate::events::{Event, KeywordActionEvent, KeywordActionKind};
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
        let would_event = Event::new_with_provenance(
            KeywordActionEvent::new(KeywordActionKind::Learn, ctx.controller, ctx.source, 1),
            ctx.provenance,
        );
        let applied_effects = ctx.replacement.suppressed_replacement_effects.clone();
        let applied_effect_keys = ctx.replacement.suppressed_replacement_effect_keys.clone();
        if applied_effects.is_empty() && applied_effect_keys.is_empty() {
            game.update_replacement_effects();
        }
        match process_trait_event_with_dm_and_applied_effects(
            game,
            would_event,
            ctx.decision_maker,
            &applied_effects,
            &applied_effect_keys,
        ) {
            TraitEventResult::Replaced {
                effects, effect_id, ..
            } => {
                return crate::effects::composition::mechanic_actions::execute_keyword_action_replacement_effects(
                    game, ctx, effects, effect_id, None,
                );
            }
            TraitEventResult::Prevented => return Ok(EffectOutcome::count(0)),
            TraitEventResult::NeedsChoice { .. } | TraitEventResult::NeedsInteraction { .. } => {
                return Ok(EffectOutcome::count(0));
            }
            TraitEventResult::Proceed(_) | TraitEventResult::Modified(_) => {}
        }

        // CR 701.48a offers the discard first. Only a player who did not
        // actually discard a card gets the later Lesson choice.
        let discard_outcome = execute_effect(
            game,
            &Effect::new(MayEffect::single(Effect::discard(1))),
            ctx,
        )?;
        if discard_outcome.count_or_zero() > 0 {
            let draw_outcome = execute_effect(game, &Effect::draw(1), ctx)?;
            return Ok(
                EffectOutcome::aggregate([discard_outcome, draw_outcome]).with_event(
                    crate::triggers::TriggerEvent::new_with_provenance(
                        KeywordActionEvent::new(
                            KeywordActionKind::Learn,
                            ctx.controller,
                            ctx.source,
                            1,
                        ),
                        ctx.provenance,
                    ),
                ),
            );
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
            return Ok(
                EffectOutcome::aggregate([discard_outcome, choose_outcome, lesson_outcome])
                    .with_event(crate::triggers::TriggerEvent::new_with_provenance(
                        KeywordActionEvent::new(
                            KeywordActionKind::Learn,
                            ctx.controller,
                            ctx.source,
                            1,
                        ),
                        ctx.provenance,
                    )),
            );
        }

        Ok(
            EffectOutcome::aggregate([discard_outcome, choose_outcome]).with_event(
                crate::triggers::TriggerEvent::new_with_provenance(
                    KeywordActionEvent::new(
                        KeywordActionKind::Learn,
                        ctx.controller,
                        ctx.source,
                        1,
                    ),
                    ctx.provenance,
                ),
            ),
        )
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

    struct ChooseReplacementNamed(&'static str);

    impl DecisionMaker for ChooseReplacementNamed {
        fn decide_options(
            &mut self,
            _game: &GameState,
            ctx: &crate::decisions::context::SelectOptionsContext,
        ) -> Vec<usize> {
            ctx.options
                .iter()
                .find(|option| option.legal && option.description.contains(self.0))
                .map(|option| vec![option.index])
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

    #[test]
    fn learn_replacement_returns_its_graveyard_source_before_learning() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let phoenix = game.create_object_from_card(
            &card("Retriever", CardType::Creature),
            alice,
            Zone::Graveyard,
        );
        game.object_mut(phoenix)
            .expect("phoenix exists")
            .abilities_mut()
            .push(
                crate::ability::Ability::static_ability(
                    crate::static_abilities::StaticAbility::keyword_action_replacement_with_performer(
                        KeywordActionKind::Learn,
                        ObjectFilter::default(),
                        Some(PlayerFilter::You),
                        vec![Effect::move_to_zone(ChooseSpec::Source, Zone::Battlefield, false)],
                        true,
                        "Return Retriever instead of learning",
                    ),
                )
                .in_zones(vec![Zone::Graveyard]),
            );
        let source = game.new_object_id();
        let mut dm = ChooseReplacementNamed("Return Retriever");
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);

        LearnEffect::new()
            .execute(&mut game, &mut ctx)
            .expect("learn replacement should resolve");

        assert!(has_named_object_in_zone(
            &game,
            Zone::Battlefield,
            "Retriever"
        ));
        assert!(game.player(alice).unwrap().hand.is_empty());
    }
}
