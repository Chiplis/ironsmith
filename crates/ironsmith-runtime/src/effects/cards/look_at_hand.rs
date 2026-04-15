//! Look at hand effect implementation.

use crate::decisions::context::ViewCardsContext;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_players_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

/// Effect that lets a player look at another player's hand.
#[derive(Debug, Clone, PartialEq)]
pub struct LookAtHandEffect {
    /// Which player(s) to look at.
    pub target: ChooseSpec,
    /// Whether this is a public reveal instead of a private look.
    pub reveal: bool,
}

impl LookAtHandEffect {
    /// Create a new look-at-hand effect.
    pub fn new(target: ChooseSpec) -> Self {
        Self {
            target,
            reveal: false,
        }
    }

    /// Create a reveal-hand effect (all players see the hand).
    pub fn reveal(target: ChooseSpec) -> Self {
        Self {
            target,
            reveal: true,
        }
    }
}

impl EffectExecutor for LookAtHandEffect {
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
        for player_id in players {
            let cards = game
                .player(player_id)
                .map(|p| p.hand.clone())
                .unwrap_or_default();
            total_cards += cards.len() as i32;

            if self.reveal {
                for viewer_idx in 0..game.players.len() {
                    let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
                    let mut view_ctx =
                        ViewCardsContext::look_at_hand(viewer, player_id, Some(ctx.source));
                    view_ctx.description = "Reveal that player's hand".to_string();
                    view_ctx.public = true;
                    ctx.decision_maker
                        .view_cards(game, viewer, &cards, &view_ctx);
                }
            } else {
                let view_ctx =
                    ViewCardsContext::look_at_hand(ctx.controller, player_id, Some(ctx.source));
                ctx.decision_maker
                    .view_cards(game, ctx.controller, &cards, &view_ctx);
            }
        }

        Ok(EffectOutcome::count(total_cards))
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
    use crate::executor::ResolvedTarget;
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
