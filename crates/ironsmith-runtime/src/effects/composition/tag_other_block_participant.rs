//! Tag the combat participant opposite the resolving ability's source.

use crate::effect::EffectOutcome;
use crate::effects::{EffectExecutor, ExecutionContext, ExecutionError};
use crate::filter::ObjectFilterExt as _;
use crate::game_state::GameState;
pub use ironsmith_core::TagOtherBlockParticipantEffect;

impl EffectExecutor for TagOtherBlockParticipantEffect {
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
        let candidates = if block_context.attacker == ctx.source {
            block_context.blocker_snapshots
        } else if block_context.blockers.contains(&ctx.source) {
            block_context.attacker_snapshot.into_iter().collect()
        } else {
            return Err(ExecutionError::UnresolvableValue(
                "ability source is not a participant in the triggering block event".to_string(),
            ));
        };
        let filter_ctx = ctx.filter_context(game);
        let tagged = candidates
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
    use crate::events::combat::CreatureBlockedEvent;
    use crate::ids::{CardId, PlayerId};
    use crate::provenance::ProvNodeId;
    use crate::triggers::TriggerEvent;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn creature(game: &mut GameState, name: &str, controller: PlayerId) -> crate::ids::ObjectId {
        let card = CardBuilder::new(CardId::new(), name)
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(2, 2))
            .build();
        game.create_object_from_card(&card, controller, Zone::Battlefield)
    }

    fn tagged_other_for_source(source_is_attacker: bool) {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        let attacker = creature(&mut game, "Attacker", alice);
        let blocker = creature(&mut game, "Blocker", bob);
        let event = TriggerEvent::new_with_provenance(
            CreatureBlockedEvent::new(blocker, attacker),
            ProvNodeId::default(),
        );
        let source = if source_is_attacker {
            attacker
        } else {
            blocker
        };
        let controller = if source_is_attacker { alice } else { bob };
        let mut dm = crate::decision::AutoPassDecisionMaker;
        let mut ctx =
            ExecutionContext::new(source, controller, &mut dm).with_triggering_event(event);

        TagOtherBlockParticipantEffect::new(
            "combat_other",
            Some(crate::target::ObjectFilter::creature()),
        )
        .execute(&mut game, &mut ctx)
        .expect("opposite block participant should be taggable");

        let tagged = ctx
            .get_tagged_all("combat_other")
            .expect("combat-other tag");
        assert_eq!(tagged.len(), 1);
        assert_eq!(
            tagged[0].object_id,
            if source_is_attacker {
                blocker
            } else {
                attacker
            }
        );
    }

    #[test]
    fn attacking_source_tags_the_blocker() {
        tagged_other_for_source(true);
    }

    #[test]
    fn blocking_source_tags_the_attacker() {
        tagged_other_for_source(false);
    }
}
