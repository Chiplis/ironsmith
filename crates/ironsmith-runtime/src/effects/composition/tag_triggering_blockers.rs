//! Tag blockers from a block-related triggering event for later reference.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
pub use ironsmith_core::TagTriggeringBlockersEffect;

impl EffectExecutor for TagTriggeringBlockersEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn is_resolution_prelude(&self) -> bool {
        true
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let block_context = ctx.block_event_context(game).ok_or_else(|| {
            ExecutionError::UnresolvableValue("missing block event context".to_string())
        })?;
        let filter_ctx = ctx.filter_context(game);
        let tagged = block_context
            .blocker_snapshots
            .into_iter()
            .filter(|snapshot| {
                self.filter
                    .as_ref()
                    .is_none_or(|filter| filter.matches_snapshot(snapshot, &filter_ctx, game))
            })
            .collect::<Vec<_>>();
        let count = tagged.len() as i32;
        ctx.set_tagged_objects(self.tag.clone(), tagged);
        Ok(EffectOutcome::count(count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::events::combat::CreatureBecameBlockedEvent;
    use crate::ids::{CardId, PlayerId};
    use crate::provenance::ProvNodeId;
    use crate::target::ObjectFilter;
    use crate::triggers::TriggerEvent;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn create_permanent(
        game: &mut GameState,
        name: &str,
        controller: PlayerId,
        card_types: Vec<CardType>,
    ) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::from_raw(game.new_object_id().0 as u32), name)
            .card_types(card_types)
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    #[test]
    fn tags_matching_blockers_from_became_blocked_event() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = create_permanent(&mut game, "Attacker", alice, vec![CardType::Creature]);
        let artifact_blocker = create_permanent(
            &mut game,
            "Artifact Blocker",
            bob,
            vec![CardType::Artifact, CardType::Creature],
        );
        let creature_blocker =
            create_permanent(&mut game, "Creature Blocker", bob, vec![CardType::Creature]);

        let event = TriggerEvent::new_with_provenance(
            CreatureBecameBlockedEvent::with_target_and_blockers(
                attacker,
                vec![artifact_blocker, creature_blocker],
                None,
                None,
                Vec::new(),
            ),
            ProvNodeId::default(),
        );
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx = ExecutionContext::new(attacker, alice, &mut dm).with_triggering_event(event);
        let mut filter = ObjectFilter::default();
        filter.all_card_types = vec![CardType::Artifact, CardType::Creature];
        let effect = TagTriggeringBlockersEffect::new("blocking", Some(filter));

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("blocker tag prelude should execute");

        assert_eq!(outcome.value, crate::effect::OutcomeValue::Count(1));
        let tagged = ctx
            .get_tagged_all("blocking")
            .expect("blocking tag should be present");
        assert_eq!(tagged.len(), 1);
        assert_eq!(tagged[0].object_id, artifact_blocker);
    }
}
