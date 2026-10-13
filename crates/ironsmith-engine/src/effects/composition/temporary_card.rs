//! Own a temporary card incarnation for the lifetime of a child program.
use crate::effect::Effect;
use crate::effects::{CompletedEffectOutputs, ExecutionContext, ExecutionError};
use crate::{game_state::GameState, object::Object, tag::TagKey, zone::Zone};

pub(crate) fn with_temporary_card_copy(
    game: &mut GameState,
    ctx: &mut ExecutionContext,
    card: &Object,
    tag: TagKey,
    body: &Effect,
) -> Result<CompletedEffectOutputs, ExecutionError> {
    let previous = ctx.tagged_objects.remove(&tag);
    let result = super::execute_transaction(
        game,
        ctx,
        || CompletedEffectOutputs::aggregate_only(crate::effect::EffectOutcome::count(0)),
        |game, ctx| {
            let id = game.new_object_id();
            let mut copy = Object::token_copy_of(card, id, ctx.controller);
            copy.zone = Zone::Exile;
            game.add_object(copy);
            let snapshot = crate::snapshot::ObjectSnapshot::from_object(
                game.object(id).expect("materialized card copy"),
                game,
            );
            ctx.set_tagged_objects(tag.clone(), vec![snapshot]);
            let outputs = body.0.execute_child_with_outputs(game, ctx)?;
            // Only the unconsumed temporary incarnation belongs to this scope.
            // Casting can create a different incarnation with the same stable ID.
            if game
                .object(id)
                .is_some_and(|object| object.zone == Zone::Exile)
            {
                game.remove_object(id);
            }
            Ok(outputs)
        },
    );
    ctx.tagged_objects.remove(&tag);
    if let Some(previous) = previous {
        ctx.tagged_objects.insert(tag, previous);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        card::CardBuilder,
        ids::{CardId, PlayerId},
    };
    fn stored(game: &mut GameState) -> Object {
        let id = game.create_object_from_card(
            &CardBuilder::new(CardId::new(), "Stored card")
                .card_types(vec![crate::types::CardType::Instant])
                .mana_cost(crate::mana::ManaCost::from_pips(vec![vec![
                    crate::mana::ManaSymbol::Generic(1),
                ]]))
                .build(),
            PlayerId(0),
            Zone::Graveyard,
        );
        game.object(id).unwrap().clone()
    }
    #[derive(Default)]
    struct Pause {
        pending: bool,
    }
    impl crate::decision::DecisionMaker for Pause {
        fn decide_boolean(
            &mut self,
            _: &GameState,
            _: &crate::decisions::context::BooleanContext,
        ) -> bool {
            self.pending = true;
            false
        }
        fn awaiting_choice(&self) -> bool {
            self.pending
        }
    }
    #[test]
    fn pending_temporary_copy_rolls_back_materialization_and_restores_outer_tag() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let card = stored(&mut game);
        let tag = TagKey::from("temporary");
        let original = crate::snapshot::ObjectSnapshot::from_object(&card, &game);
        let mut dm = Pause::default();
        let mut ctx = ExecutionContext::new(card.id, PlayerId(0), &mut dm);
        ctx.set_tagged_objects(tag.clone(), vec![original]);
        let before = game.exile.len();
        with_temporary_card_copy(
            &mut game,
            &mut ctx,
            &card,
            tag.clone(),
            &Effect::may(vec![Effect::gain_life(2)]),
        )
        .unwrap();
        assert!(ctx.decision_maker.awaiting_choice());
        assert_eq!(game.exile.len(), before);
        assert_eq!(game.player(PlayerId(0)).unwrap().life, 20);
        assert_eq!(ctx.get_tagged_all(&tag).unwrap()[0].object_id, card.id);
    }
    #[test]
    fn temporary_copy_cleanup_preserves_a_consumed_new_incarnation() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let card = stored(&mut game);
        let tag = TagKey::from("temporary");
        let mut ctx = ExecutionContext::new_default(card.id, PlayerId(0));
        let body = Effect::cast_tagged(
            tag.clone(),
            crate::target::PlayerFilter::You,
            false,
            false,
            true,
            None,
        );
        let output =
            with_temporary_card_copy(&mut game, &mut ctx, &card, tag.clone(), &body).unwrap();
        let moved = output.outcome.objects().unwrap()[0];
        assert_eq!(game.object(moved).unwrap().zone, Zone::Stack);
        assert_eq!(game.object(moved).unwrap().name, "Stored card");
        assert!(game.exile.is_empty());
        assert!(ctx.get_tagged_all(&tag).is_none());
    }
    #[test]
    fn nested_temporary_copies_restore_lexical_tags_and_cleanup_unused_objects() {
        #[derive(Debug, Clone)]
        struct Nested {
            card: Object,
            tag: TagKey,
        }
        impl crate::effects::EffectExecutor for Nested {
            fn execute(
                &self,
                game: &mut GameState,
                ctx: &mut ExecutionContext,
            ) -> Result<crate::effect::EffectOutcome, ExecutionError> {
                let outer = ctx.get_tagged_all(&self.tag).unwrap()[0].object_id;
                with_temporary_card_copy(
                    game,
                    ctx,
                    &self.card,
                    self.tag.clone(),
                    &Effect::gain_life(1),
                )?;
                assert_eq!(ctx.get_tagged_all(&self.tag).unwrap()[0].object_id, outer);
                Ok(crate::effect::EffectOutcome::resolved())
            }
        }
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let card = stored(&mut game);
        let tag = TagKey::from("temporary");
        let mut ctx = ExecutionContext::new_default(card.id, PlayerId(0));
        with_temporary_card_copy(
            &mut game,
            &mut ctx,
            &card,
            tag.clone(),
            &Effect::new(Nested {
                card: card.clone(),
                tag: tag.clone(),
            }),
        )
        .unwrap();
        assert!(game.exile.is_empty());
        assert!(ctx.get_tagged_all(&tag).is_none());
    }
}
