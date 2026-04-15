//! Tag the triggering object's snapshot for later reference.

use crate::effect::EffectOutcome;
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

        if let Some(zone_change) = event.downcast::<crate::events::zones::ZoneChangeEvent>()
            && !zone_change.result_objects.is_empty()
        {
            let tagged: Vec<_> = zone_change
                .destination_objects()
                .iter()
                .filter_map(|&id| {
                    game.object(id)
                        .map(|obj| ObjectSnapshot::from_object(obj, game))
                })
                .collect();
            if !tagged.is_empty() {
                ctx.set_tagged_objects(self.tag.clone(), tagged.clone());
                return Ok(EffectOutcome::count(tagged.len() as i32));
            }
        }

        let object_id = event.object_id().ok_or_else(|| {
            ExecutionError::UnresolvableValue("triggering event missing object".to_string())
        })?;

        if let Some(obj) = game.object(object_id) {
            ctx.set_tagged_objects(
                self.tag.clone(),
                vec![ObjectSnapshot::from_object(obj, game)],
            );
            return Ok(EffectOutcome::count(1));
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
            return Ok(EffectOutcome::count(1));
        }

        Ok(EffectOutcome::count(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::{CardBuilder, PowerToughness};
    use crate::executor::ExecutionContext;
    use crate::ids::{CardId, ObjectId, PlayerId, StableId};
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
            .move_object_by_effect(creature_id, Zone::Graveyard)
            .expect("creature should move to graveyard");
        assert_ne!(graveyard_id, creature_id);

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_cause(
                creature_id,
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
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
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(1));

        let tagged = ctx
            .get_tagged("triggering")
            .expect("triggering tag should be present");
        assert_eq!(tagged.object_id, graveyard_id);
        assert_eq!(tagged.stable_id, snapshot.stable_id);
    }

    #[test]
    fn test_tag_triggering_object_uses_all_split_meld_result_objects() {
        let mut game = setup_game();
        let alice = PlayerId::from_index(0);

        let first = game.new_object_id();
        let first_card = make_creature_card(first.0 as u32, "Graf Rats");
        game.add_object(Object::from_card(
            first,
            &first_card,
            alice,
            Zone::Graveyard,
        ));

        let second = game.new_object_id();
        let second_card = make_creature_card(second.0 as u32, "Midnight Scavengers");
        game.add_object(Object::from_card(
            second,
            &second_card,
            alice,
            Zone::Graveyard,
        ));

        let trigger_event = crate::triggers::TriggerEvent::new_with_provenance(
            crate::events::zones::ZoneChangeEvent::with_results(
                ObjectId::from_raw(999),
                vec![first, second],
                Zone::Battlefield,
                Zone::Graveyard,
                crate::events::cause::EventCause::from_sba(),
                Some(ObjectSnapshot {
                    object_id: ObjectId::from_raw(999),
                    stable_id: StableId::from(ObjectId::from_raw(999)),
                    kind: crate::object::ObjectKind::Card,
                    card: None,
                    controller: alice,
                    owner: alice,
                    name: "Chittering Host".to_string(),
                    mana_cost: None,
                    colors: crate::color::ColorSet::default(),
                    supertypes: Vec::new(),
                    card_types: vec![CardType::Creature],
                    subtypes: Vec::new(),
                    oracle_text: String::new(),
                    other_face: None,
                    other_face_name: None,
                    linked_face_layout: crate::card::LinkedFaceLayout::TransformLike,
                    power: Some(5),
                    toughness: Some(6),
                    base_power: Some(5),
                    base_toughness: Some(6),
                    loyalty: None,
                    defense: None,
                    abilities: Vec::new(),
                    aura_attach_filter: None,
                    max_saga_chapter: None,
                    x_value: None,
                    cast_order_this_turn: None,
                    counters: std::collections::HashMap::new(),
                    is_token: false,
                    tapped: false,
                    flipped: false,
                    face_down: false,
                    transform_count: 0,
                    attached_to: None,
                    attachments: Vec::new(),
                    was_enchanted: false,
                    is_monstrous: false,
                    is_commander: false,
                    zone: Zone::Battlefield,
                }),
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
        assert_eq!(result.value, crate::effect::OutcomeValue::Count(2));

        let tagged = ctx
            .get_tagged_all("triggering")
            .expect("triggering tag should be present");
        let tagged_ids: Vec<_> = tagged.iter().map(|snapshot| snapshot.object_id).collect();
        assert_eq!(tagged_ids, vec![first, second]);
    }
}
