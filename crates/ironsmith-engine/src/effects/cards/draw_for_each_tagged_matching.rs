use super::DrawCardsEffect;
use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::tag::TagKey;
use crate::target::{ObjectFilter, PlayerFilter};

/// Draw cards equal to the number of tagged snapshots matching `filter`.
///
/// This is used for clauses like "draw a card for each card exiled from their
/// hand this way", where the counted cards are tracked as tagged snapshots from
/// an earlier zone.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawForEachTaggedMatchingEffect {
    pub player: PlayerFilter,
    pub tag: TagKey,
    pub filter: ObjectFilter,
}

impl DrawForEachTaggedMatchingEffect {
    pub fn draw_effect(&self) -> DrawCardsEffect {
        DrawCardsEffect::new(
            crate::effect::Value::TaggedSnapshotCount {
                tag: self.tag.clone(),
                filter: self.filter.clone(),
            },
            self.player.clone(),
        )
    }

    pub fn new(player: PlayerFilter, tag: impl Into<TagKey>, filter: ObjectFilter) -> Self {
        Self {
            player,
            tag: tag.into(),
            filter,
        }
    }
}

impl EffectExecutor for DrawForEachTaggedMatchingEffect {
    fn canonical_program(&self) -> Option<crate::effect::Effect> {
        Some(crate::effect::Effect::new(self.draw_effect()))
    }

    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        self.draw_effect().execute_child(game, ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::snapshot::ObjectSnapshot;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn add_library_cards(game: &mut GameState, owner: PlayerId, count: usize) {
        for idx in 0..count {
            let card = CardBuilder::new(CardId::new(), format!("Library Card {}", idx + 1))
                .card_types(vec![CardType::Sorcery])
                .build();
            game.create_object_from_card(&card, owner, Zone::Library);
        }
    }

    #[test]
    fn draw_for_each_tagged_matching_counts_snapshot_zone() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);
        add_library_cards(&mut game, alice, 3);

        let source = game.new_object_id();
        let hand_card = CardBuilder::new(CardId::new(), "Hand Card")
            .card_types(vec![CardType::Creature])
            .build();
        let graveyard_card = CardBuilder::new(CardId::new(), "Graveyard Card")
            .card_types(vec![CardType::Creature])
            .build();
        let hand_id = game.create_object_from_card(&hand_card, alice, Zone::Hand);
        let graveyard_id = game.create_object_from_card(&graveyard_card, alice, Zone::Graveyard);

        let mut ctx = ExecutionContext::new_default(source, alice);
        let hand_snapshot =
            ObjectSnapshot::from_object(game.object(hand_id).expect("hand object"), &game);
        let graveyard_snapshot = ObjectSnapshot::from_object(
            game.object(graveyard_id).expect("graveyard object"),
            &game,
        );
        // The quantity must inspect retained origins, not the current exile
        // incarnations (nor a later current-zone projection of the filter).
        game.move_object_by_effect(hand_id, Zone::Exile)
            .expect("exile hand card");
        game.move_object_by_effect(graveyard_id, Zone::Exile)
            .expect("exile graveyard card");
        ctx.set_tagged_objects(
            "searched_multi_zone",
            vec![hand_snapshot, graveyard_snapshot],
        );

        let effect = DrawForEachTaggedMatchingEffect::new(
            PlayerFilter::You,
            "searched_multi_zone",
            ObjectFilter::default().in_zone(Zone::Hand),
        );
        let outcome = effect.execute(&mut game, &mut ctx).unwrap();

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        assert_eq!(game.player(alice).unwrap().hand.len(), 1);
    }
}
