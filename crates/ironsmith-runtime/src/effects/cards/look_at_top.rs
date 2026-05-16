//! Look at top cards effect implementation.

use crate::decisions::context::ViewCardsContext;
use crate::effect::{EffectOutcome, OutcomeObjectMemory};
use crate::effects::EffectExecutor;
use crate::effects::helpers::{resolve_player_filter, resolve_value};
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
pub use ironsmith_core::LookAtTopCardsEffect;

/// Effect that looks at the top N cards of a player's library and tags them.
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

        if self.reveal {
            ctx.tag_objects_unique(self.tag.clone(), snapshots.clone());
            ctx.tag_objects(crate::effects::PUBLIC_REVEALED_TAG, snapshots.clone());
            for viewer_idx in 0..game.players.len() {
                let viewer = crate::ids::PlayerId::from_index(viewer_idx as u8);
                let view_ctx = ViewCardsContext::new(
                    viewer,
                    player_id,
                    Some(ctx.source),
                    crate::zone::Zone::Library,
                    "Reveal cards from the top of a library",
                )
                .with_public(true);
                ctx.decision_maker
                    .view_cards(game, viewer, &top_cards, &view_ctx);
            }
        } else {
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
            ctx.set_tagged_objects(self.tag.clone(), snapshots.clone());
        }

        let memory: Vec<_> = snapshots
            .iter()
            .map(OutcomeObjectMemory::from_snapshot)
            .collect();
        let mut outcome = EffectOutcome::count(snapshots.len() as i32)
            .with_chosen_object_memory(memory.clone())
            .with_affected_object_memory(memory);
        if self.reveal {
            outcome = outcome.with_events(top_cards.iter().filter_map(|card_id| {
                let snapshot = game
                    .object(*card_id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game));
                Some(crate::triggers::TriggerEvent::new_with_provenance(
                    crate::events::CardRevealedEvent::new(
                        player_id,
                        *card_id,
                        crate::zone::Zone::Library,
                        Some(ctx.source),
                        snapshot,
                    ),
                    ctx.provenance,
                ))
            }));
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::decision::DecisionMaker;
    use crate::effect::Effect;
    use crate::effects::{ForEachObject, ForPlayersEffect};
    use crate::ids::{CardId, PlayerId};
    use crate::mana::ManaSymbol;
    use crate::tag::TagKey;
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::test_prelude::*;
    use crate::types::CardType;
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

    fn add_typed_card_to_library(
        game: &mut GameState,
        owner: PlayerId,
        name: &str,
        id: u32,
        card_types: Vec<CardType>,
    ) {
        let card = CardBuilder::new(CardId::from_raw(id), name)
            .card_types(card_types)
            .build();
        game.create_object_from_card(&card, owner, Zone::Library);
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
    fn revealing_top_cards_accumulates_shared_tags_across_each_player() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let source = game.new_object_id();
        add_typed_card_to_library(
            &mut game,
            alice,
            "Alice Revealed Creature",
            20_001,
            vec![CardType::Creature],
        );
        add_typed_card_to_library(
            &mut game,
            bob,
            "Bob Revealed Creature",
            20_002,
            vec![CardType::Creature],
        );

        let tag = TagKey::from("revealed_this_way");
        let mut ctx = ExecutionContext::new_default(source, alice);
        let reveal_each = ForPlayersEffect::new(
            PlayerFilter::Any,
            vec![Effect::reveal_top_cards(
                PlayerFilter::IteratedPlayer,
                1,
                tag.clone(),
            )],
        );
        reveal_each
            .execute(&mut game, &mut ctx)
            .expect("execute each-player reveal");

        assert_eq!(
            ctx.get_tagged_all(&tag).map(|snapshots| snapshots.len()),
            Some(2)
        );

        let mut filter = ObjectFilter::default();
        filter.excluded_card_types.push(CardType::Land);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: tag.clone(),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let parley_reward = ForEachObject::new(
            filter,
            vec![
                Effect::add_mana(vec![ManaSymbol::Green]),
                Effect::gain_life(1),
            ],
        );
        parley_reward
            .execute(&mut game, &mut ctx)
            .expect("execute parley reward");

        assert_eq!(game.player(alice).expect("alice").mana_pool.green, 2);
        assert_eq!(game.player(alice).expect("alice").life, 22);
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
