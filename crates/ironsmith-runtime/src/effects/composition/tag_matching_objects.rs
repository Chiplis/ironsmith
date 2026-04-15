//! Tag all objects matching a filter across one or more zones.

use crate::effect::EffectOutcome;
use crate::effects::EffectExecutor;
use crate::effects::helpers::resolve_objects_from_spec;
use crate::executor::{ExecutionContext, ExecutionError};
use crate::game_state::GameState;
use crate::snapshot::ObjectSnapshot;
use crate::tag::TagKey;
use crate::target::{ChooseSpec, ObjectFilter};
use crate::zone::Zone;

#[derive(Debug, Clone, PartialEq)]
pub struct TagMatchingObjectsEffect {
    pub filter: ObjectFilter,
    pub zone: Option<Zone>,
    pub additional_zones: Vec<Zone>,
    pub tag: TagKey,
}

impl TagMatchingObjectsEffect {
    pub fn new(filter: ObjectFilter, tag: impl Into<TagKey>) -> Self {
        Self {
            filter,
            zone: None,
            additional_zones: Vec::new(),
            tag: tag.into(),
        }
    }

    pub fn in_zone(mut self, zone: Zone) -> Self {
        self.zone = Some(zone);
        self.additional_zones.clear();
        self
    }

    pub fn in_zones(mut self, zones: Vec<Zone>) -> Self {
        let mut iter = zones.into_iter();
        if let Some(first) = iter.next() {
            self.zone = Some(first);
            self.additional_zones = iter.collect();
        } else {
            self.zone = None;
            self.additional_zones.clear();
        }
        self
    }

    fn zones(&self) -> Vec<Zone> {
        let mut zones = Vec::new();
        if let Some(zone) = self.zone.or(self.filter.zone) {
            zones.push(zone);
        }
        for zone in &self.additional_zones {
            if !zones.contains(zone) {
                zones.push(*zone);
            }
        }
        zones
    }
}

impl EffectExecutor for TagMatchingObjectsEffect {
    fn execute(
        &self,
        game: &mut GameState,
        ctx: &mut ExecutionContext,
    ) -> Result<EffectOutcome, ExecutionError> {
        let mut object_ids = Vec::new();
        for zone in self.zones() {
            let mut filter = self.filter.clone();
            filter.zone = Some(zone);
            for object_id in resolve_objects_from_spec(game, &ChooseSpec::All(filter), ctx)? {
                if !object_ids.contains(&object_id) {
                    object_ids.push(object_id);
                }
            }
        }

        let snapshots = object_ids
            .iter()
            .filter_map(|id| {
                game.object(*id)
                    .map(|obj| ObjectSnapshot::from_object(obj, game))
            })
            .collect::<Vec<_>>();
        ctx.set_tagged_objects(self.tag.clone(), snapshots);
        Ok(EffectOutcome::with_objects(object_ids))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::{CardId, PlayerId};
    use crate::mana::{ManaCost, ManaSymbol};
    use crate::object::Object;
    use crate::types::CardType;

    fn make_card(name: &str) -> crate::card::Card {
        CardBuilder::new(CardId::from_raw(99), name)
            .mana_cost(ManaCost::from_pips(vec![vec![ManaSymbol::Generic(1)]]))
            .card_types(vec![CardType::Creature])
            .build()
    }

    #[test]
    fn tags_matching_objects_across_multiple_hidden_zones() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let hand_card_id = game.new_object_id();
        let graveyard_card_id = game.new_object_id();
        let hand_card = make_card("Hand Creature");
        let graveyard_card = make_card("Graveyard Creature");
        game.add_object(Object::from_card(
            hand_card_id,
            &hand_card,
            alice,
            Zone::Hand,
        ));
        game.add_object(Object::from_card(
            graveyard_card_id,
            &graveyard_card,
            alice,
            Zone::Graveyard,
        ));
        game.player_mut(alice)
            .expect("player")
            .hand
            .push(hand_card_id);
        game.player_mut(alice)
            .expect("player")
            .graveyard
            .push(graveyard_card_id);

        let source = game.new_object_id();
        let mut ctx = ExecutionContext::new_default(source, alice);
        let effect = TagMatchingObjectsEffect::new(ObjectFilter::creature(), "matched")
            .in_zones(vec![Zone::Hand, Zone::Graveyard]);

        let outcome = effect
            .execute(&mut game, &mut ctx)
            .expect("effect resolves");
        let crate::effect::OutcomeValue::Objects(ids) = outcome.value else {
            panic!("expected tagged object ids");
        };
        assert_eq!(ids.len(), 2);

        let tagged = ctx
            .get_tagged_all("matched")
            .expect("tagged snapshots should be stored");
        assert_eq!(tagged.len(), 2);
    }
}
