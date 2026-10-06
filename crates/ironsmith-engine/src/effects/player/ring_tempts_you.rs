//! The Ring tempts you effect implementation.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{normalize_object_selection, resolve_player_filter};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::game_state::GameState;
use crate::target::PlayerFilter;
use crate::types::CardType;

#[derive(Debug, Clone, PartialEq)]
pub struct RingTemptsYouEffect {
    pub player: PlayerFilter,
}

impl RingTemptsYouEffect {
    pub fn new(player: PlayerFilter) -> Self {
        Self { player }
    }

    pub fn you() -> Self {
        Self::new(PlayerFilter::You)
    }
}

impl EffectExecutor for RingTemptsYouEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        crate::effects::composition::execute_compound(game, ctx, |game, ctx| {
            let player_id = resolve_player_filter(game, &self.player, ctx)?;
            game.reconcile_ring_bearer(player_id);
            game.increment_ring_temptations(player_id);

            let mut candidates = game
                .battlefield
                .iter()
                .copied()
                .filter(|&id| {
                    // CR 702.26b: a phased-out creature can't become Ring-bearer.
                    !game.is_phased_out(id)
                        && game.current_controller(id) == Some(player_id)
                        && game.object_has_card_type(id, CardType::Creature)
                })
                .collect::<Vec<_>>();
            candidates.sort_unstable();

            let mut chosen_bearer = None;
            if !candidates.is_empty() {
                let chosen = if candidates.len() == 1 {
                    candidates[0]
                } else {
                    let selection = make_decision(
                        game,
                        ctx.decision_maker,
                        player_id,
                        Some(ctx.source),
                        ChooseObjectsSpec::new(
                            ctx.source,
                            "Choose your Ring-bearer",
                            candidates.clone(),
                            1,
                            Some(1),
                        ),
                    );
                    if ctx.decision_maker.awaiting_choice() {
                        return Ok(EffectOutcome::default());
                    }
                    let normalized = normalize_object_selection(selection, &candidates, 1);
                    *normalized.first().ok_or_else(|| {
                        ExecutionError::Impossible("missing Ring-bearer choice".to_string())
                    })?
                };
                game.set_ring_bearer(player_id, chosen);
                chosen_bearer = game.object(chosen).map(|object| {
                    crate::snapshot::ObjectSnapshot::from_object_with_calculated_characteristics(
                        object, game,
                    )
                });
            }

            let mut event =
                KeywordActionEvent::new(KeywordActionKind::RingTemptsYou, player_id, ctx.source, 1);
            if let Some(snapshot) = chosen_bearer {
                event.object_tags.insert(
                    ironsmith_core::tag::RING_BEARER_CHOSEN_TAG.into(),
                    vec![snapshot],
                );
            }
            crate::effects::composition::complete_keyword_action(game, ctx, event)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::card::PowerToughness;
    use crate::decision::DecisionMaker;
    use crate::decisions::context::SelectObjectsContext;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::types::Supertype;
    use crate::zone::Zone;

    struct ChooseSecondDecisionMaker;

    impl DecisionMaker for ChooseSecondDecisionMaker {
        fn decide_objects(
            &mut self,
            _game: &GameState,
            ctx: &SelectObjectsContext,
        ) -> Vec<ObjectId> {
            ctx.candidates
                .get(1)
                .map(|candidate| vec![candidate.id])
                .unwrap_or_default()
        }
    }

    fn make_creature(game: &mut GameState, owner: PlayerId, name: &str) -> ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, owner, Zone::Battlefield)
    }

    #[test]
    fn ring_tempts_you_tracks_count_and_chooses_ring_bearer() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source = ObjectId::from_raw(777);
        let first = make_creature(&mut game, alice, "First Bearer");
        let second = make_creature(&mut game, alice, "Second Bearer");
        let mut dm = ChooseSecondDecisionMaker;

        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let outcome = RingTemptsYouEffect::you()
            .execute(&mut game, &mut ctx)
            .expect("ring tempts should resolve");

        assert_eq!(outcome.status, crate::effect::OutcomeStatus::Succeeded);
        assert_eq!(game.ring_temptations(alice), 1);
        assert_eq!(game.current_ring_bearer(alice), Some(second));
        // CR 701.54c: the Ring-bearer is legendary through a layer-4 effect,
        // not a copiable supertype (CR 701.54b).
        game.refresh_continuous_state();
        let legendary = |id| {
            game.current_characteristics(id)
                .is_some_and(|chars| chars.supertypes.contains(&Supertype::Legendary))
        };
        assert!(legendary(second));
        assert!(!legendary(first));
        assert!(
            !game
                .object(second)
                .is_some_and(|object| object.supertypes.contains(&Supertype::Legendary)),
            "Legendary isn't written into the bearer's copiable supertypes"
        );
        assert_eq!(outcome.events.len(), 1);
        let keyword = outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::RingTemptsYou);
        assert_eq!(keyword.player, alice);
    }
}

#[cfg(test)]
mod event_evidence_tests {
    use super::*;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::triggers::{TriggerContext, TriggerMatcher};
    const A: PlayerId = PlayerId(0);
    const B: PlayerId = PlayerId(1);
    fn creature(game: &mut GameState) -> ObjectId {
        let card = crate::card::CardBuilder::new(CardId::new(), "Bearer evidence")
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, A, crate::zone::Zone::Battlefield)
    }
    fn tempt(game: &mut GameState) -> crate::triggers::TriggerEvent {
        let mut ctx = ExecutionContext::new_default(ObjectId::from_raw(777), A);
        RingTemptsYouEffect::you()
            .execute(game, &mut ctx)
            .unwrap()
            .events
            .remove(0)
    }
    #[test]
    fn no_creature_is_not_a_choice_but_reselecting_the_same_bearer_is_a_new_choice() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let none = tempt(&mut game);
        let watcher = crate::triggers::RingBearerChosenTrigger {
            player: PlayerFilter::You,
        };
        let ctx = TriggerContext::for_source(ObjectId::from_raw(777), A, &game);
        assert!(!watcher.matches(&none, &ctx));
        assert_eq!(game.ring_temptations(A), 1);
        let bearer = creature(&mut game);
        for _ in 0..2 {
            let chosen = tempt(&mut game);
            let ctx = TriggerContext::for_source(ObjectId::from_raw(777), A, &game);
            assert!(watcher.matches(&chosen, &ctx));
            let keyword = chosen.downcast::<KeywordActionEvent>().unwrap();
            assert_eq!(
                keyword.object_tags[ironsmith_core::tag::RING_BEARER_CHOSEN_TAG][0].object_id,
                bearer
            );
            let opponent = TriggerContext::for_source(ObjectId::from_raw(777), B, &game);
            assert!(!watcher.matches(&chosen, &opponent));
        }
        assert_eq!(game.ring_temptations(A), 3);
    }
    #[test]
    fn an_intervening_choice_predicate_does_not_drift_when_the_bearer_changes_or_leaves() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let first = creature(&mut game);
        let event = tempt(&mut game);
        let other = creature(&mut game);
        let condition = crate::ConditionExpr::YouChoseAnotherRingBearer;
        let own = ExecutionContext::new_default(first, A).with_triggering_event(event.clone());
        let observer = ExecutionContext::new_default(other, A).with_triggering_event(event.clone());
        assert!(
            !crate::condition_eval::evaluate_condition_resolution(&game, &condition, &own).unwrap()
        );
        assert!(
            crate::condition_eval::evaluate_condition_resolution(&game, &condition, &observer)
                .unwrap()
        );
        game.set_ring_bearer(A, other);
        game.move_object(
            first,
            crate::zone::Zone::Graveyard,
            crate::events::EventCause::effect(),
        )
        .unwrap();
        assert!(
            !crate::condition_eval::evaluate_condition_resolution(&game, &condition, &own).unwrap()
        );
        assert!(
            crate::condition_eval::evaluate_condition_resolution(&game, &condition, &observer)
                .unwrap()
        );
        assert_eq!(game.current_ring_bearer(A), Some(other));
    }
    #[test]
    fn a_pending_bearer_choice_rolls_back_temptation_count_and_designation() {
        struct Pause(bool);
        impl DecisionMaker for Pause {
            fn decide_objects(
                &mut self,
                _: &GameState,
                _: &crate::decisions::context::SelectObjectsContext,
            ) -> Vec<ObjectId> {
                self.0 = true;
                vec![]
            }
            fn awaiting_choice(&self) -> bool {
                self.0
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let first = creature(&mut game);
        creature(&mut game);
        game.set_ring_bearer(A, first);
        let mut dm = Pause(false);
        let mut ctx = ExecutionContext::new(first, A, &mut dm);
        let result = RingTemptsYouEffect::you()
            .execute(&mut game, &mut ctx)
            .unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert!(result.events.is_empty());
        assert_eq!(game.ring_temptations(A), 0);
        assert_eq!(game.current_ring_bearer(A), Some(first));
        let _event = tempt(&mut game);
        assert_eq!(game.ring_temptations(A), 1);
    }
}
