use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::continuous::{CalculatedCharacteristics, ContinuousEffect};
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::object_query::candidate_ids_for_zone;
use crate::player::ManaPool;
use crate::static_abilities::StaticAbility;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype};
use crate::zone::Zone;

/// Read-only, pass-local cache for derived game state.
///
/// This is intentionally ephemeral. It avoids repeated continuous-effect
/// collection, characteristic calculation, candidate zone scans, and potential
/// mana computation inside one legality/trigger/SBA pass without introducing
/// global invalidation concerns on `GameState`.
pub(crate) struct DerivedGameView<'a> {
    game: &'a GameState,
    all_effects: Vec<ContinuousEffect>,
    characteristics: RefCell<HashMap<ObjectId, Option<CalculatedCharacteristics>>>,
    zone_candidates: RefCell<HashMap<Option<Zone>, Vec<ObjectId>>>,
    potential_mana: RefCell<HashMap<PlayerId, ManaPool>>,
}

impl<'a> DerivedGameView<'a> {
    pub(crate) fn new(game: &'a GameState) -> Self {
        Self::from_effects(game, game.all_continuous_effects())
    }

    /// Build a derived view from the state populated by `refresh_continuous_state`.
    ///
    /// Callers should only use this when they know the cached static-ability
    /// effects on `GameState` are current for the state they are about to read.
    pub(crate) fn from_refreshed_state(game: &'a GameState) -> Self {
        Self::from_effects(game, game.cached_continuous_effects_snapshot())
    }

    pub(crate) fn from_effects(game: &'a GameState, all_effects: Vec<ContinuousEffect>) -> Self {
        Self {
            game,
            all_effects,
            characteristics: RefCell::new(HashMap::new()),
            zone_candidates: RefCell::new(HashMap::new()),
            potential_mana: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn game(&self) -> &'a GameState {
        self.game
    }

    pub(crate) fn effects(&self) -> &[ContinuousEffect] {
        &self.all_effects
    }

    pub(crate) fn calculated_characteristics(
        &self,
        object_id: ObjectId,
    ) -> Option<CalculatedCharacteristics> {
        if let Some(cached) = self.characteristics.borrow().get(&object_id) {
            return cached.clone();
        }

        let calculated = self
            .game
            .calculated_characteristics_with_effects(object_id, &self.all_effects);
        self.characteristics
            .borrow_mut()
            .insert(object_id, calculated.clone());
        calculated
    }

    pub(crate) fn calculated_power(&self, object_id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(object_id)
            .and_then(|chars| chars.power)
    }

    pub(crate) fn calculated_toughness(&self, object_id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(object_id)
            .and_then(|chars| chars.toughness)
    }

    pub(crate) fn calculated_card_types(&self, object_id: ObjectId) -> Vec<CardType> {
        self.calculated_characteristics(object_id)
            .map(|chars| chars.card_types)
            .unwrap_or_default()
    }

    pub(crate) fn calculated_subtypes(&self, object_id: ObjectId) -> Vec<Subtype> {
        self.calculated_characteristics(object_id)
            .map(|chars| chars.subtypes)
            .unwrap_or_default()
    }

    pub(crate) fn static_abilities(&self, object_id: ObjectId) -> Vec<StaticAbility> {
        self.calculated_characteristics(object_id)
            .map(|chars| chars.static_abilities)
            .unwrap_or_default()
    }

    pub(crate) fn abilities(&self, object_id: ObjectId) -> Option<Vec<crate::ability::Ability>> {
        self.calculated_characteristics(object_id)
            .map(|chars| chars.abilities)
    }

    pub(crate) fn object_has_card_type(&self, object_id: ObjectId, card_type: CardType) -> bool {
        self.calculated_characteristics(object_id)
            .is_some_and(|chars| chars.card_types.contains(&card_type))
    }

    pub(crate) fn object_has_static_ability_id(
        &self,
        object_id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        self.calculated_characteristics(object_id)
            .is_some_and(|chars| {
                chars
                    .static_abilities
                    .iter()
                    .any(|ability| ability.id() == ability_id)
            })
    }

    pub(crate) fn candidate_ids_for_zone(&self, zone: Option<Zone>) -> Vec<ObjectId> {
        if let Some(cached) = self.zone_candidates.borrow().get(&zone) {
            return cached.clone();
        }

        let ids = candidate_ids_for_zone(self.game, zone);
        self.zone_candidates.borrow_mut().insert(zone, ids.clone());
        ids
    }

    pub(crate) fn candidate_ids_for_filter(&self, filter: &ObjectFilter) -> Vec<ObjectId> {
        if let Some(zone) = filter.zone {
            return self.candidate_ids_for_zone(Some(zone));
        }

        if filter.any_of.is_empty() {
            return self.candidate_ids_for_zone(None);
        }

        let mut ids = HashSet::new();
        for nested in &filter.any_of {
            for id in self.candidate_ids_for_zone(nested.zone) {
                ids.insert(id);
            }
        }

        if ids.is_empty() {
            self.candidate_ids_for_zone(None)
        } else {
            ids.into_iter().collect()
        }
    }

    pub(crate) fn potential_mana(&self, player: PlayerId) -> ManaPool {
        if let Some(cached) = self.potential_mana.borrow().get(&player) {
            return cached.clone();
        }

        let pool = crate::decision::compute_potential_mana(self.game, player);
        self.potential_mana
            .borrow_mut()
            .insert(player, pool.clone());
        pool
    }

    pub(crate) fn can_potentially_pay(
        &self,
        player: PlayerId,
        cost: &ManaCost,
        x_value: u32,
    ) -> bool {
        self.potential_mana(player).can_pay(cost, x_value)
    }
}
