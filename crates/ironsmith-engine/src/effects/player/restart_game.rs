use crate::effect::EffectOutcome;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::target::ChooseSpec;

pub use ironsmith_core::RestartGameEffect;

impl EffectExecutor for RestartGameEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let cards_left_in_exile = match &self.cards_left_in_exile {
            Some(spec) => resolve_objects_from_spec(game, spec, ctx)?,
            None => Vec::new(),
        };
        let restarted_cards = game.restart_game(ctx.controller, &cards_left_in_exile);
        Ok(EffectOutcome::resolved().with_affected_objects_from_game(game, restarted_cards))
    }

    fn decision_related_object_specs(&self) -> Vec<ChooseSpec> {
        self.cards_left_in_exile.clone().into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::effect::OutcomeStatus;
    use crate::ids::{CardId, PlayerId};
    use crate::object::ObjectKind;
    use crate::tag::{SOURCE_EXILED_TAG, TagKey};
    use crate::target::{ObjectFilter, TaggedObjectConstraint, TaggedOpbjectRelation};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    fn add_cards(game: &mut GameState, owner: PlayerId, prefix: &str, count: usize) {
        for idx in 0..count {
            let card = CardBuilder::new(
                CardId::from_raw(90_000 + owner.index() as u32 * 100 + idx as u32),
                format!("{prefix} {idx}"),
            )
            .card_types(vec![CardType::Land])
            .build();
            game.create_object_from_card(&card, owner, Zone::Library);
        }
    }

    #[test]
    fn restart_rebuilds_game_and_preserves_exempt_exiled_cards() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        add_cards(&mut game, alice, "Alice land", 8);
        add_cards(&mut game, bob, "Bob land", 8);

        let source_card = CardBuilder::new(CardId::from_raw(90_300), "Restart Source")
            .card_types(vec![CardType::Planeswalker])
            .build();
        let source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let kept_card = CardBuilder::new(CardId::from_raw(90_301), "Kept Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let kept = game.create_object_from_card(&kept_card, bob, Zone::Exile);
        game.add_exiled_with_source_link(source, kept);

        let kept_stable_id = game.object(kept).expect("kept card exists").stable_id;
        let mut ctx = ExecutionContext::new_default(source, alice);
        let snapshot = crate::snapshot::ObjectSnapshot::from_object(
            game.object(kept).expect("kept card exists"),
            &game,
        );
        ctx.tag_object(SOURCE_EXILED_TAG, snapshot);

        let mut filter = ObjectFilter::permanent_card().in_zone(Zone::Exile);
        filter.excluded_subtypes.push(Subtype::Aura);
        filter.tagged_constraints.push(TaggedObjectConstraint {
            tag: TagKey::from(SOURCE_EXILED_TAG),
            relation: TaggedOpbjectRelation::IsTaggedObject,
        });
        let result = RestartGameEffect::new(Some(ChooseSpec::All(filter)))
            .execute(&mut game, &mut ctx)
            .expect("restart resolves");

        assert_eq!(result.status, OutcomeStatus::Succeeded);
        assert_eq!(game.turn.active_player, alice);
        assert_eq!(game.turn_store.turn_order.first().copied(), Some(alice));
        assert_eq!(game.player(alice).expect("Alice exists").hand.len(), 7);
        assert_eq!(game.player(bob).expect("Bob exists").hand.len(), 7);
        assert!(game.stack.is_empty());
        assert!(game.battlefield.is_empty());
        assert!(
            game.objects_in_deterministic_order()
                .iter()
                .all(|object| { object.kind == ObjectKind::Card })
        );

        let kept_after_restart = game
            .find_object_by_stable_id(kept_stable_id)
            .expect("exempt card keeps its stable identity");
        assert_eq!(
            game.object(kept_after_restart)
                .expect("kept card exists")
                .zone,
            Zone::Exile
        );
        assert!(
            result
                .affected_objects()
                .is_some_and(|objects| objects.contains(&kept_after_restart))
        );
    }
}
