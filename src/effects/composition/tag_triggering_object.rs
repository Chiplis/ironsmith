//! Tag the triggering object's snapshot for later reference.

use crate::effect::{EffectOutcome, EffectResult};
use crate::effects::EffectExecutor;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;

/// Effect that tags the object that caused the trigger.
#[derive(Debug, Clone, PartialEq)]
pub struct TagTriggeringObjectEffect {
    /// Tag name to store the triggering object snapshot under.
    pub tag: TagKey,
}

impl TagTriggeringObjectEffect {
    /// Create a new effect that tags the triggering object.
    pub fn new(tag: impl Into<TagKey>) -> Self {
        Self { tag: tag.into() }
    }
}

impl EffectExecutor for TagTriggeringObjectEffect {
    fn clone_box(&self) -> Box<dyn EffectExecutor> {
        Box::new(self.clone())
    }

    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let event = ctx.triggering_event.as_ref().ok_or_else(|| {
            ExecutionError::UnresolvableValue("missing triggering event".to_string())
        })?;

        let object_id = event.object_id().ok_or_else(|| {
            ExecutionError::UnresolvableValue("triggering event missing object".to_string())
        })?;

        if let Some(obj) = game.object(object_id) {
            ctx.set_tagged_objects(
                self.tag.clone(),
                vec![ObjectSnapshot::from_object(obj, game)],
            );
            return Ok(EffectOutcome::from_result(EffectResult::Count(1)));
        }

        if let Some(snapshot) = event.snapshot() {
            // For zone-change triggers (e.g., dies), retarget to the immediate
            // post-change object ID when it exists so delayed effects can
            // reference that exact object instance later.
            let mut tagged = snapshot.clone();
            if let Some(current_id) = game.find_object_by_stable_id(snapshot.stable_id) {
                tagged.object_id = current_id;
            }
            ctx.set_tagged_objects(self.tag.clone(), vec![tagged]);
            return Ok(EffectOutcome::from_result(EffectResult::Count(1)));
        }

        Ok(EffectOutcome::count(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::effect::EffectResult;
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;
    use crate::zone::Zone;

    fn setup_game() -> GameState {
        crate::tests::test_helpers::setup_two_player_game()
    }

    fn make_creature_card(card_id: u32, name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(card_id), name)
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Black],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(PowerToughness::fixed(1, 1))
            .build()
    }

    #[test]
    fn test_tag_triggering_object_uses_post_zone_change_object_id() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let creature_id = game.new_object_id();
        let card = make_creature_card(creature_id.0 as u32, "Nine-Lives Familiar");
        let obj = Object::from_card(creature_id, &card, alice, Zone::Battlefield);
        game.add_object(obj);

        let snapshot = ObjectSnapshot::from_object(
            game.object(creature_id).expect("creature should exist"),
            &game,
        );
        let graveyard_id = game
            .move_object(creature_id, Zone::Graveyard)
            .expect("creature should move to graveyard");
        assert_ne!(graveyard_id, creature_id);

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::new(
                creature_id,
                Zone::Battlefield,
                Zone::Graveyard,
                Some(snapshot.clone()),
            ),
            crate::provenance::ProvNodeId::default(),
        );
        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        ctx.triggering_event = Some(trigger_event);

        let effect = TagTriggeringObjectEffect::new("triggering");
        let result = effect
            .execute(&mut game, &mut ctx)
            .expect("effect should resolve");
        assert_eq!(result.result, EffectResult::Count(1));

        let tagged = ctx
            .get_tagged("triggering")
            .expect("triggering tag should be present");
        assert_eq!(tagged.object_id, graveyard_id);
        assert_eq!(tagged.stable_id, snapshot.stable_id);
    }
}
