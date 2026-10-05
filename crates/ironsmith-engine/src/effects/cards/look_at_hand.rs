//! Look at hand effect implementation.

use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_players_from_spec, view_hidden_candidate_objects};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;
pub type LookAtHandEffect = ironsmith_core::LookAtHandEffect;

impl EffectExecutor for LookAtHandEffect {
    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        self.reveal
            .then_some(crate::effect::PriorEffectAction::Revealed)
    }
    /// "Reveal your hand" can be paid as a cost (Land Grant's alternative
    /// cost); an empty hand can still be revealed.
    fn as_cost_executable(&self) -> Option<&dyn crate::effects::CostExecutableEffect> {
        (self.reveal
            && matches!(
                self.target,
                ChooseSpec::Player(crate::target::PlayerFilter::You)
            ))
        .then_some(self as &dyn crate::effects::CostExecutableEffect)
    }

    fn cost_description(&self) -> Option<String> {
        (self.reveal
            && matches!(
                self.target,
                ChooseSpec::Player(crate::target::PlayerFilter::You)
            ))
        .then(|| "Reveal your hand".to_string())
    }

    fn supports_simultaneous_player_action(&self) -> bool {
        true
    }

    fn prepare_simultaneous_player_action(
        &self,
        _game: &GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<Box<dyn crate::effects::SimultaneousEffectProposal>, ExecutionError> {
        // Revealing/looking at a hand involves no player choices; defer to
        // commit so each opponent's reveal lands in one batch.
        Ok(Box::new(crate::effects::DeferredPlayerActionProposal {
            effect: crate::effect::Effect::new(self.clone()),
            iterated_player: ctx.iteration.iterated_player,
        }))
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let players = resolve_players_from_spec(game, &self.target, ctx)?;

        if players.is_empty() {
            return if self.target.is_target() {
                Ok(EffectOutcome::target_invalid())
            } else {
                Ok(EffectOutcome::count(0))
            };
        }

        let mut total_cards = 0;
        let mut outcome = EffectOutcome::count(0);
        let mut all_snapshots = Vec::new();
        for player_id in players {
            let cards = game
                .player(player_id)
                .map(|p| p.hand.clone())
                .unwrap_or_default();
            total_cards += cards.len() as i32;
            let snapshots = cards
                .iter()
                .filter_map(|id| crate::snapshot::ObjectSnapshot::from_object_id(game, *id))
                .collect::<Vec<_>>();
            all_snapshots.extend(snapshots.iter().cloned());
            if self.reveal {
                let reveal = super::reveal_objects(
                    game,
                    ctx,
                    snapshots.clone(),
                    Some(player_id),
                    "Reveal that player's hand",
                    None,
                )?;
                outcome = EffectOutcome::aggregate([outcome, reveal]);
                for snapshot in snapshots {
                    ctx.tag_object(crate::effects::REVEALED_THIS_WAY_TAG, snapshot);
                }
            } else {
                // Record exactly the looked-at cards so a following "exile
                // those cards" acts on this set. The set lives only in this
                // resolution's context: it is shown to the looking player
                // alone and never marked publicly revealed.
                for card_id in cards.iter().copied() {
                    if let Some(object) = game.object(card_id) {
                        ctx.tag_object(
                            crate::tag::LOOKED_AT_HAND_TAG,
                            crate::snapshot::ObjectSnapshot::from_object(object, game),
                        );
                    }
                }
                let view_ctx =
                    ViewCardsContext::look_at_hand(ctx.controller, player_id, Some(ctx.source));
                ctx.decision_maker
                    .view_cards(game, ctx.controller, &cards, &view_ctx);
            }
        }

        outcome.set_value(crate::effect::OutcomeValue::Count(i64::from(total_cards)));
        if !self.reveal {
            outcome = outcome
                .with_chosen_object_memory(all_snapshots.clone())
                .with_affected_object_memory(all_snapshots);
        }
        Ok(outcome)
    }

    fn get_target_spec(&self) -> Option<&ChooseSpec> {
        if self.target.is_target() {
            Some(&self.target)
        } else {
            None
        }
    }

    fn target_description(&self) -> &'static str {
        if self.reveal {
            "player whose hand is revealed"
        } else {
            "player to look at"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{Card, CardBuilder};
    use crate::decision::DecisionMaker;
    use crate::effects::ResolvedTarget;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[derive(Debug)]
    struct ViewCall {
        viewer: PlayerId,
        subject: PlayerId,
        zone: Zone,
        cards: Vec<ObjectId>,
    }

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<ViewCall>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls.push(ViewCall {
                viewer,
                subject: ctx.subject,
                zone: ctx.zone,
                cards: cards.to_vec(),
            });
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_spell_card(card_id: u32, name: &str) -> Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Instant])
            .build()
    }

    fn add_card_to_hand(game: &mut GameState, name: &str, owner: PlayerId) -> ObjectId {
        let id = game.new_object_id();
        let card = make_spell_card(id.0 as u32, name);
        let obj = Object::from_card(id, &card, owner, Zone::Hand);
        game.add_object(obj);
        id
    }

    #[test]
    fn test_look_at_target_players_hand() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let card1 = add_card_to_hand(&mut game, "Card 1", bob);
        let card2 = add_card_to_hand(&mut game, "Card 2", bob);

        let source = game.new_object_id();
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let effect = LookAtHandEffect::new(ChooseSpec::target_player());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(dm.calls.len(), 1);

        let call = &dm.calls[0];
        assert_eq!(call.viewer, alice);
        assert_eq!(call.subject, bob);
        assert_eq!(call.zone, Zone::Hand);
        assert_eq!(call.cards, vec![card1, card2]);
    }

    #[test]
    fn test_reveal_target_players_hand_to_all_players() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let card1 = add_card_to_hand(&mut game, "Card 1", bob);
        let card2 = add_card_to_hand(&mut game, "Card 2", bob);

        let source = game.new_object_id();
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm)
            .with_targets(vec![ResolvedTarget::Player(bob)]);

        let effect = LookAtHandEffect::reveal(ChooseSpec::target_player());
        let result = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(dm.calls.len(), 2, "both players should see revealed hand");
        assert!(dm.calls.iter().all(|call| call.subject == bob));
        assert!(dm.calls.iter().all(|call| call.zone == Zone::Hand));
        assert!(dm.calls.iter().all(|call| call.cards == vec![card1, card2]));
    }
}

impl crate::effects::CostExecutableEffect for LookAtHandEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: crate::ids::ObjectId,
        _controller: crate::ids::PlayerId,
    ) -> Result<(), crate::effects::executor_trait::CostValidationError> {
        Ok(())
    }
}
