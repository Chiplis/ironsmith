//! Reveal tagged cards effect implementation.
//!
//! Reveals currently update player-facing visibility and carry that visibility
//! through tagged contexts when later stack objects still need it.

use crate::effect::EffectOutcome;
use crate::effects::{CostExecutableEffect, CostValidationError, EffectExecutor};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};

#[cfg(test)]
use crate::tag::TagKey;
pub type RevealTaggedEffect = ironsmith_core::RevealTaggedEffect;

impl EffectExecutor for RevealTaggedEffect {
    fn cost_choice_bindings(&self) -> crate::effects::CostChoiceBindings {
        crate::effects::CostChoiceBindings::requiring(self.tag.clone())
    }

    fn result_action(&self) -> Option<crate::effect::PriorEffectAction> {
        Some(crate::effect::PriorEffectAction::Revealed)
    }
    fn is_read_only_simultaneous_player_action(&self) -> bool {
        true
    }

    fn as_cost_executable(&self) -> Option<&dyn CostExecutableEffect> {
        Some(self)
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let tagged = ctx
            .get_tagged_all(self.tag.clone())
            .cloned()
            .unwrap_or_default();
        super::reveal_objects(game, ctx, tagged, None, "Reveal cards", None)
    }
}

impl CostExecutableEffect for RevealTaggedEffect {
    fn can_execute_as_cost(
        &self,
        _game: &GameState,
        _source: ObjectId,
        _controller: PlayerId,
    ) -> Result<(), CostValidationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effects::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    #[derive(Debug, Default)]
    struct CaptureViewDm {
        calls: Vec<(PlayerId, PlayerId, Zone, bool, Vec<crate::ids::ObjectId>)>,
    }

    impl DecisionMaker for CaptureViewDm {
        fn view_cards(
            &mut self,
            _game: &GameState,
            viewer: PlayerId,
            cards: &[crate::ids::ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls
                .push((viewer, ctx.subject, ctx.zone, ctx.public, cards.to_vec()));
        }
    }

    #[test]
    fn reveal_tagged_emits_public_view_for_tagged_cards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        let card = CardBuilder::new(CardId::from_raw(201), "Tagged Card")
            .card_types(vec![CardType::Instant])
            .build();
        let object_id = game.create_object_from_card(&card, bob, Zone::Library);

        let snapshot = {
            let obj = game.object(object_id).expect("tagged object");
            ObjectSnapshot::from_object(obj, &game)
        };
        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        ctx.set_tagged_objects(TagKey::from("revealed"), vec![snapshot]);

        RevealTaggedEffect::new("revealed")
            .execute(&mut game, &mut ctx)
            .expect("reveal tagged");

        assert_eq!(dm.calls.len(), 2);
        assert!(dm.calls.iter().all(|(_, subject, zone, public, cards)| {
            *subject == bob && *zone == Zone::Library && *public && cards == &vec![object_id]
        }));
    }
}
