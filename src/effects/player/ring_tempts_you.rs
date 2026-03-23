//! The Ring tempts you effect implementation.

use crate::decisions::make_decision;
use crate::decisions::specs::ChooseObjectsSpec;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{normalize_object_selection, resolve_player_filter};
use crate::events::{KeywordActionEvent, KeywordActionKind};
use crate::executor::{ExecutionContext, ExecutionError};
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
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        game.reconcile_ring_bearer(player_id);
        game.increment_ring_temptations(player_id);

        let mut candidates = game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| {
                game.current_controller(id) == Some(player_id)
                    && game.object_has_card_type(id, CardType::Creature)
            })
            .collect::<Vec<_>>();
        candidates.sort_unstable();

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
        }

        let event =
            KeywordActionEvent::new(KeywordActionKind::RingTemptsYou, player_id, ctx.source, 1);
        Ok(EffectOutcome::resolved().with_event(
            crate::triggers::TriggerEvent::new_with_provenance(event, ctx.provenance),
        ))
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
        assert!(
            game.object(second)
                .is_some_and(|object| object.supertypes.contains(&Supertype::Legendary))
        );
        assert!(
            !game
                .object(first)
                .is_some_and(|object| object.supertypes.contains(&Supertype::Legendary))
        );
        assert_eq!(outcome.events.len(), 1);
        let keyword = outcome.events[0]
            .downcast::<KeywordActionEvent>()
            .expect("expected keyword action event");
        assert_eq!(keyword.action, KeywordActionKind::RingTemptsYou);
        assert_eq!(keyword.player, alice);
    }
}
