//! Look at top cards effect implementation.

use crate::decisions::context::ViewCardsContext;
use crate::effect::{EffectOutcome, Value};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::executor::{ExecutionContext, ExecutionError};
use crate::filter::PlayerFilter;
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that looks at the top N cards of a player's library and tags them.
#[derive(Debug, Clone, PartialEq)]
pub struct LookAtTopCardsEffect {
    pub player: PlayerFilter,
    pub count: Value,
    pub tag: TagKey,
}

impl LookAtTopCardsEffect {
    pub fn new(player: PlayerFilter, count: impl Into<Value>, tag: impl Into<TagKey>) -> Self {
        Self {
            player,
            count: count.into(),
            tag: tag.into(),
        }
    }
}

impl EffectExecutor for LookAtTopCardsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let player_id = resolve_player_filter(game, &self.player, ctx)?;
        let Some(player) = game.player(player_id) else {
            return Ok(EffectOutcome::count(0));
        };
        let count = resolve_value(game, &self.count, ctx)?.max(0) as usize;
        if count == 0 {
            return Ok(EffectOutcome::count(0));
        }

        let top_cards: Vec<_> = player.library.iter().rev().take(count).copied().collect();
        if top_cards.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        let snapshots: Vec<ObjectSnapshot> = top_cards
            .iter()
            .filter_map(|&id| {
                game.object(id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect();
        if snapshots.is_empty() {
            return Ok(EffectOutcome::count(0));
        }

        ctx.set_tagged_objects(self.tag.clone(), snapshots.clone());
        let view_ctx = ViewCardsContext::new(
            ctx.controller,
            player_id,
            Some(ctx.source),
            crate::zone::Zone::Library,
            "Look at cards from the top of a library",
        );
        ctx.decision_maker
            .view_cards(game, ctx.controller, &top_cards, &view_ctx);
        ctx.remember_face_down_exile_viewers(&top_cards, ctx.controller);
        Ok(EffectOutcome::count(snapshots.len() as i32))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::ids::{CardId, PlayerId};
    use crate::zone::Zone;

    #[derive(Debug)]
    struct ViewCall {
        viewer: PlayerId,
        subject: PlayerId,
        zone: Zone,
        public: bool,
        cards: Vec<crate::ids::ObjectId>,
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
            cards: &[crate::ids::ObjectId],
            ctx: &crate::decisions::context::ViewCardsContext,
        ) {
            self.calls.push(ViewCall {
                viewer,
                subject: ctx.subject,
                zone: ctx.zone,
                public: ctx.public,
                cards: cards.to_vec(),
            });
        }
    }

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_cards_to_library(game: &mut GameState, owner: PlayerId, count: usize) {
        for idx in 0..count {
            let card = CardBuilder::new(
                CardId::from_raw(10_000 + idx as u32),
                &format!("Library Card {idx}"),
            )
            .build();
            game.create_object_from_card(&card, owner, Zone::Library);
        }
    }

    #[test]
    fn look_at_top_fixed_count_tags_cards() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_cards_to_library(&mut game, alice, 5);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = LookAtTopCardsEffect::new(PlayerFilter::You, 2, "looked");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("execute look-at-top");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));
        assert_eq!(
            ctx.tagged_objects
                .get(&TagKey::from("looked"))
                .map(|snapshots| snapshots.len()),
            Some(2)
        );
    }

    #[test]
    fn look_at_top_x_count_uses_context_x_value() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_cards_to_library(&mut game, alice, 6);

        let mut ctx = ExecutionContext::new_default(source, alice).with_x(3);
        let effect = LookAtTopCardsEffect::new(PlayerFilter::You, Value::X, "looked_x");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("execute look-at-top");

        assert_eq!(result.value, crate::effect::OutcomeValue::Count(3));
        assert_eq!(
            ctx.tagged_objects
                .get(&TagKey::from("looked_x"))
                .map(|snapshots| snapshots.len()),
            Some(3)
        );
    }

    #[test]
    fn look_at_top_emits_private_view_cards_event() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let source = game.new_object_id();
        add_cards_to_library(&mut game, alice, 4);

        let mut dm = CaptureViewDm::default();
        let mut ctx = ExecutionContext::new(source, alice, &mut dm);
        let effect = LookAtTopCardsEffect::new(PlayerFilter::You, 2, "looked");
        effect
            .execute(&mut game, &mut ctx)
            .expect("execute look-at-top");

        assert_eq!(dm.calls.len(), 1);
        let call = &dm.calls[0];
        assert_eq!(call.viewer, alice);
        assert_eq!(call.subject, alice);
        assert_eq!(call.zone, Zone::Library);
        assert!(!call.public);
        assert_eq!(call.cards.len(), 2);
    }
}
