use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::ability::{Ability, AbilityKind};
use crate::continuous::{CalculatedCharacteristics, ContinuousEffect, EffectTarget, Layer};
use crate::game_state::GameState;
use crate::grant::{DerivedAlternativeCast, Grantable};
use crate::grant_registry::{Grant, GrantedAlternativeCast, GrantedPlayFrom};
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaCost;
use crate::mana::ManaSymbol;
use crate::object_query::candidate_ids_for_zone;
use crate::player::ManaPool;
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
    battlefield_characteristic_scope: BattlefieldCharacteristicScope,
    use_game_characteristics_cache: bool,
    characteristics: RefCell<HashMap<ObjectId, Option<CalculatedCharacteristics>>>,
    abilities_cache: RefCell<HashMap<ObjectId, Rc<Vec<Ability>>>>,
    ability_index_summary_cache: RefCell<HashMap<ObjectId, Rc<AbilityIndexSummary>>>,
    static_abilities_cache:
        RefCell<HashMap<ObjectId, Rc<Vec<crate::static_abilities::StaticAbility>>>>,
    zone_candidates: RefCell<HashMap<Option<Zone>, Vec<ObjectId>>>,
    battlefield_creatures: RefCell<Option<Vec<ObjectId>>>,
    battlefield_noncreatures: RefCell<Option<Vec<ObjectId>>>,
    battlefield_controlled: RefCell<HashMap<PlayerId, Vec<ObjectId>>>,
    battlefield_controlled_creatures: RefCell<HashMap<PlayerId, Vec<ObjectId>>>,
    battlefield_opponents: RefCell<HashMap<PlayerId, Vec<ObjectId>>>,
    battlefield_opponent_creatures: RefCell<HashMap<PlayerId, Vec<ObjectId>>>,
    potential_mana: RefCell<HashMap<PlayerId, ManaPool>>,
    potential_mana_compute_ms: RefCell<f64>,
    granted_alternative_casts:
        RefCell<HashMap<(ObjectId, Zone, PlayerId), Vec<GrantedAlternativeCast>>>,
    granted_play_from: RefCell<HashMap<(ObjectId, Zone, PlayerId), Vec<GrantedPlayFrom>>>,
    granted_static_ability_presence: RefCell<
        HashMap<
            (
                ObjectId,
                Zone,
                PlayerId,
                crate::static_abilities::StaticAbilityId,
            ),
            bool,
        >,
    >,
    active_grants: RefCell<Option<Rc<Vec<Grant>>>>,
    active_grant_zone_presence: RefCell<HashMap<(PlayerId, Zone), bool>>,
    battlefield_spell_cost_modifier_sources: RefCell<Option<Vec<ObjectId>>>,
    activated_ability_cost_modifier_sources: RefCell<Option<Vec<ObjectId>>>,
    has_battlefield_spell_cost_modifiers: RefCell<Option<bool>>,
    has_activated_ability_cost_modifiers: RefCell<Option<bool>>,
    simple_battlefield_mana_analysis: RefCell<HashMap<PlayerId, Rc<SimpleBattlefieldManaAnalysis>>>,
    spell_target_legality: RefCell<HashMap<SpellTargetLegalityKey, bool>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpellTargetLegalityKey {
    source_id: Option<ObjectId>,
    effects_ptr: usize,
    effects_len: usize,
    chosen_modes: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SimpleBattlefieldManaAnalysis {
    relevant_source_ids: Vec<ObjectId>,
    mana_source_ids: Vec<ObjectId>,
    activatable_indices: HashMap<ObjectId, Vec<usize>>,
    mana_ability_indices: HashMap<ObjectId, Vec<usize>>,
    activated_ability_indices: HashMap<ObjectId, Vec<usize>>,
    first_output_by_permanent: HashMap<ObjectId, Vec<ManaSymbol>>,
}

impl SimpleBattlefieldManaAnalysis {
    pub(crate) fn relevant_source_ids(&self) -> &[ObjectId] {
        &self.relevant_source_ids
    }

    pub(crate) fn mana_source_ids(&self) -> &[ObjectId] {
        &self.mana_source_ids
    }

    pub(crate) fn activatable_indices_for(&self, object_id: ObjectId) -> &[usize] {
        self.activatable_indices
            .get(&object_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn mana_ability_indices_for(&self, object_id: ObjectId) -> &[usize] {
        self.mana_ability_indices
            .get(&object_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn activated_ability_indices_for(&self, object_id: ObjectId) -> &[usize] {
        self.activated_ability_indices
            .get(&object_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub(crate) fn first_output_for(&self, object_id: ObjectId) -> Option<&[ManaSymbol]> {
        self.first_output_by_permanent
            .get(&object_id)
            .map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AbilityIndexSummary {
    mana_ability_indices: Vec<usize>,
    activated_ability_indices: Vec<usize>,
}

impl AbilityIndexSummary {
    pub(crate) fn mana_ability_indices(&self) -> &[usize] {
        &self.mana_ability_indices
    }

    pub(crate) fn activated_ability_indices(&self) -> &[usize] {
        &self.activated_ability_indices
    }

    pub(crate) fn has_any_relevant_abilities(&self) -> bool {
        !self.mana_ability_indices.is_empty() || !self.activated_ability_indices.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BattlefieldCharacteristicScope {
    None,
    Specific(Vec<ObjectId>),
    AllBattlefield,
}

impl BattlefieldCharacteristicScope {
    fn includes(&self, object_id: ObjectId) -> bool {
        match self {
            Self::None => false,
            Self::Specific(ids) => ids.contains(&object_id),
            Self::AllBattlefield => true,
        }
    }
}

fn battlefield_characteristic_scope(
    effects: &[ContinuousEffect],
) -> BattlefieldCharacteristicScope {
    let mut specific_ids = Vec::new();

    for effect in effects {
        if !matches!(
            effect.modification.layer(),
            Layer::Copy
                | Layer::Control
                | Layer::Text
                | Layer::Type
                | Layer::Color
                | Layer::Ability
        ) {
            continue;
        }

        if let crate::continuous::EffectSourceType::Resolution { locked_targets } =
            &effect.source_type
            && !locked_targets.is_empty()
        {
            for &id in locked_targets {
                if !specific_ids.contains(&id) {
                    specific_ids.push(id);
                }
            }
            continue;
        }

        match &effect.applies_to {
            EffectTarget::Specific(id) | EffectTarget::AttachedTo(id) => {
                if !specific_ids.contains(id) {
                    specific_ids.push(*id);
                }
            }
            EffectTarget::Source => {
                if !specific_ids.contains(&effect.source) {
                    specific_ids.push(effect.source);
                }
            }
            EffectTarget::Filter(_) | EffectTarget::AllPermanents | EffectTarget::AllCreatures => {
                return BattlefieldCharacteristicScope::AllBattlefield;
            }
        }
    }

    if specific_ids.is_empty() {
        BattlefieldCharacteristicScope::None
    } else {
        BattlefieldCharacteristicScope::Specific(specific_ids)
    }
}

impl<'a> DerivedGameView<'a> {
    pub(crate) fn new(game: &'a GameState) -> Self {
        if game.continuous_state_is_clean() {
            Self::from_refreshed_state(game)
        } else {
            Self::from_effects(game, game.all_continuous_effects())
        }
    }

    /// Build a derived view from the state populated by `refresh_continuous_state`.
    ///
    /// Callers should only use this when they know the cached static-ability
    /// effects on `GameState` are current for the state they are about to read.
    pub(crate) fn from_refreshed_state(game: &'a GameState) -> Self {
        let all_effects = game.cached_continuous_effects_snapshot();
        Self {
            game,
            battlefield_characteristic_scope: battlefield_characteristic_scope(&all_effects),
            all_effects,
            use_game_characteristics_cache: true,
            characteristics: RefCell::new(HashMap::new()),
            abilities_cache: RefCell::new(HashMap::new()),
            ability_index_summary_cache: RefCell::new(HashMap::new()),
            static_abilities_cache: RefCell::new(HashMap::new()),
            zone_candidates: RefCell::new(HashMap::new()),
            battlefield_creatures: RefCell::new(None),
            battlefield_noncreatures: RefCell::new(None),
            battlefield_controlled: RefCell::new(HashMap::new()),
            battlefield_controlled_creatures: RefCell::new(HashMap::new()),
            battlefield_opponents: RefCell::new(HashMap::new()),
            battlefield_opponent_creatures: RefCell::new(HashMap::new()),
            potential_mana: RefCell::new(HashMap::new()),
            potential_mana_compute_ms: RefCell::new(0.0),
            granted_alternative_casts: RefCell::new(HashMap::new()),
            granted_play_from: RefCell::new(HashMap::new()),
            granted_static_ability_presence: RefCell::new(HashMap::new()),
            active_grants: RefCell::new(None),
            active_grant_zone_presence: RefCell::new(HashMap::new()),
            battlefield_spell_cost_modifier_sources: RefCell::new(None),
            activated_ability_cost_modifier_sources: RefCell::new(None),
            has_battlefield_spell_cost_modifiers: RefCell::new(None),
            has_activated_ability_cost_modifiers: RefCell::new(None),
            simple_battlefield_mana_analysis: RefCell::new(HashMap::new()),
            spell_target_legality: RefCell::new(HashMap::new()),
        }
    }

    pub(crate) fn from_effects(game: &'a GameState, all_effects: Vec<ContinuousEffect>) -> Self {
        Self {
            game,
            battlefield_characteristic_scope: battlefield_characteristic_scope(&all_effects),
            all_effects,
            use_game_characteristics_cache: false,
            characteristics: RefCell::new(HashMap::new()),
            abilities_cache: RefCell::new(HashMap::new()),
            ability_index_summary_cache: RefCell::new(HashMap::new()),
            static_abilities_cache: RefCell::new(HashMap::new()),
            zone_candidates: RefCell::new(HashMap::new()),
            battlefield_creatures: RefCell::new(None),
            battlefield_noncreatures: RefCell::new(None),
            battlefield_controlled: RefCell::new(HashMap::new()),
            battlefield_controlled_creatures: RefCell::new(HashMap::new()),
            battlefield_opponents: RefCell::new(HashMap::new()),
            battlefield_opponent_creatures: RefCell::new(HashMap::new()),
            potential_mana: RefCell::new(HashMap::new()),
            potential_mana_compute_ms: RefCell::new(0.0),
            granted_alternative_casts: RefCell::new(HashMap::new()),
            granted_play_from: RefCell::new(HashMap::new()),
            granted_static_ability_presence: RefCell::new(HashMap::new()),
            active_grants: RefCell::new(None),
            active_grant_zone_presence: RefCell::new(HashMap::new()),
            battlefield_spell_cost_modifier_sources: RefCell::new(None),
            activated_ability_cost_modifier_sources: RefCell::new(None),
            has_battlefield_spell_cost_modifiers: RefCell::new(None),
            has_activated_ability_cost_modifiers: RefCell::new(None),
            simple_battlefield_mana_analysis: RefCell::new(HashMap::new()),
            spell_target_legality: RefCell::new(HashMap::new()),
        }
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

        let calculated = if self.use_game_characteristics_cache {
            self.game.calculated_characteristics(object_id)
        } else {
            self.game
                .calculated_characteristics_with_effects(object_id, &self.all_effects)
        };
        self.characteristics
            .borrow_mut()
            .insert(object_id, calculated.clone());
        calculated
    }

    pub(crate) fn prewarm_characteristics(&self, ids: &[ObjectId]) {
        let missing: Vec<_> = {
            let cache = self.characteristics.borrow();
            ids.iter()
                .copied()
                .filter(|id| !cache.contains_key(id))
                .filter(|id| self.requires_battlefield_characteristic_calculation(*id))
                .collect()
        };
        if missing.is_empty() {
            return;
        }

        if self.use_game_characteristics_cache {
            self.game.prewarm_calculated_characteristics(&missing);
            let mut cache = self.characteristics.borrow_mut();
            for id in missing {
                cache.insert(id, self.game.calculated_characteristics(id));
            }
            return;
        }

        let calculated = self
            .game
            .calculated_characteristics_batch_with_effects(&missing, &self.all_effects);
        let mut cache = self.characteristics.borrow_mut();
        for id in missing {
            cache.insert(id, calculated.get(&id).cloned());
        }
    }

    pub(crate) fn calculated_toughness(&self, object_id: ObjectId) -> Option<i32> {
        self.calculated_characteristics(object_id)
            .and_then(|chars| chars.toughness)
    }

    pub(crate) fn calculated_subtypes(&self, object_id: ObjectId) -> Vec<Subtype> {
        self.calculated_characteristics(object_id)
            .map(|chars| chars.subtypes)
            .unwrap_or_default()
    }

    pub(crate) fn object_colors(&self, object_id: ObjectId) -> crate::color::ColorSet {
        let Some(object) = self.game.object(object_id) else {
            return crate::color::ColorSet::default();
        };
        if !self.requires_battlefield_characteristic_calculation(object_id) {
            return object.colors();
        }

        self.calculated_characteristics(object_id)
            .map(|chars| chars.colors)
            .unwrap_or_else(|| object.colors())
    }

    pub(crate) fn abilities_rc(
        &self,
        object_id: ObjectId,
    ) -> Option<Rc<Vec<crate::ability::Ability>>> {
        if let Some(cached) = self.abilities_cache.borrow().get(&object_id) {
            return Some(Rc::clone(cached));
        }

        let object = self.game.object(object_id)?;
        let abilities = if !self.requires_battlefield_characteristic_calculation(object_id) {
            object.abilities.clone()
        } else {
            self.calculated_characteristics(object_id)?.abilities
        };
        let abilities = Rc::new(abilities);
        self.abilities_cache
            .borrow_mut()
            .insert(object_id, Rc::clone(&abilities));
        Some(abilities)
    }

    pub(crate) fn ability_index_summary(
        &self,
        object_id: ObjectId,
    ) -> Option<Rc<AbilityIndexSummary>> {
        if let Some(cached) = self.ability_index_summary_cache.borrow().get(&object_id) {
            return Some(Rc::clone(cached));
        }

        let object = self.game.object(object_id)?;
        let cached_abilities = self.abilities_rc(object_id);
        let abilities = cached_abilities.as_deref().unwrap_or(&object.abilities);
        let mut summary = AbilityIndexSummary::default();
        for (ability_index, ability) in abilities.iter().enumerate() {
            if !ability.functions_in(&object.zone) {
                continue;
            }
            if ability.is_mana_ability() {
                summary.mana_ability_indices.push(ability_index);
            }
            if matches!(&ability.kind, AbilityKind::Activated(activated) if !activated.is_mana_ability())
            {
                summary.activated_ability_indices.push(ability_index);
            }
        }

        let summary = Rc::new(summary);
        self.ability_index_summary_cache
            .borrow_mut()
            .insert(object_id, Rc::clone(&summary));
        Some(summary)
    }

    pub(crate) fn static_abilities_rc(
        &self,
        object_id: ObjectId,
    ) -> Option<Rc<Vec<crate::static_abilities::StaticAbility>>> {
        if let Some(cached) = self.static_abilities_cache.borrow().get(&object_id) {
            return Some(Rc::clone(cached));
        }

        let object = self.game.object(object_id)?;
        let static_abilities = if !self.requires_battlefield_characteristic_calculation(object_id) {
            object
                .abilities
                .iter()
                .filter_map(|ability| match &ability.kind {
                    AbilityKind::Static(static_ability) if ability.functions_in(&object.zone) => {
                        Some(static_ability.clone())
                    }
                    _ => None,
                })
                .collect()
        } else {
            self.calculated_characteristics(object_id)?.static_abilities
        };
        let static_abilities = Rc::new(static_abilities);
        self.static_abilities_cache
            .borrow_mut()
            .insert(object_id, Rc::clone(&static_abilities));
        Some(static_abilities)
    }

    pub(crate) fn object_has_card_type(&self, object_id: ObjectId, card_type: CardType) -> bool {
        let Some(object) = self.game.object(object_id) else {
            return false;
        };
        if !self.requires_battlefield_characteristic_calculation(object_id) {
            return object.card_types.contains(&card_type);
        }

        self.calculated_characteristics(object_id)
            .is_some_and(|chars| chars.card_types.contains(&card_type))
    }

    pub(crate) fn object_has_static_ability_id(
        &self,
        object_id: ObjectId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        let Some(object) = self.game.object(object_id) else {
            return false;
        };
        if !self.requires_battlefield_characteristic_calculation(object_id) {
            return object.abilities.iter().any(|ability| {
                matches!(&ability.kind, AbilityKind::Static(static_ability)
                    if ability.functions_in(&object.zone) && static_ability.id() == ability_id)
            });
        }

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
            let mut ordered: Vec<_> = ids.into_iter().collect();
            ordered.sort();
            ordered
        }
    }

    pub(crate) fn candidate_ids_for_filter_with_context(
        &self,
        filter: &ObjectFilter,
        filter_ctx: &crate::filter::FilterContext,
    ) -> Vec<ObjectId> {
        if let Some(ids) = self.narrow_battlefield_candidates(filter, filter_ctx) {
            return ids;
        }

        self.candidate_ids_for_filter(filter)
    }

    pub(crate) fn potential_mana(&self, player: PlayerId) -> ManaPool {
        if let Some(cached) = self.potential_mana.borrow().get(&player) {
            return cached.clone();
        }

        let started_at = crate::perf::PerfTimer::start();
        let pool = crate::decision::compute_potential_mana_with_view(self.game, player, self);
        self.potential_mana
            .borrow_mut()
            .insert(player, pool.clone());
        *self.potential_mana_compute_ms.borrow_mut() += started_at.elapsed_ms();
        pool
    }

    pub(crate) fn potential_mana_compute_ms(&self) -> f64 {
        *self.potential_mana_compute_ms.borrow()
    }

    pub(crate) fn can_potentially_pay_with_reason(
        &self,
        player: PlayerId,
        source: Option<ObjectId>,
        cost: &ManaCost,
        x_value: u32,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        let allow_any_color = self.game.can_spend_mana_as_any_color(player, source);
        let allow_black_life = self
            .game
            .player_can_pay_black_with_life_for_reason(player, source, reason);
        let mut preview_pool = self.potential_mana(player);
        let (can_pay, life_to_pay) = preview_pool
            .try_pay_tracking_life_with_any_color_and_black_life(
                cost,
                x_value,
                allow_any_color,
                allow_black_life,
            );
        can_pay
            && self
                .game
                .can_pay_life_with_reason(player, life_to_pay, reason)
    }

    pub(crate) fn simple_battlefield_mana_analysis(
        &self,
        player: PlayerId,
    ) -> Rc<SimpleBattlefieldManaAnalysis> {
        if let Some(cached) = self.simple_battlefield_mana_analysis.borrow().get(&player) {
            return Rc::clone(cached);
        }

        let mut analysis = SimpleBattlefieldManaAnalysis::default();

        for &perm_id in &self.game.battlefield {
            let Some(perm) = self.game.object(perm_id) else {
                continue;
            };
            if perm.controller != player || !self.game.can_activate_abilities_of(perm_id) {
                continue;
            }

            let abilities = self
                .abilities_rc(perm_id)
                .unwrap_or_else(|| Rc::new(perm.abilities.clone()));
            let Some(ability_summary) = self.ability_index_summary(perm_id) else {
                continue;
            };
            if !ability_summary.has_any_relevant_abilities() {
                continue;
            }

            analysis.relevant_source_ids.push(perm_id);
            if !ability_summary.mana_ability_indices().is_empty() {
                analysis.mana_source_ids.push(perm_id);
                analysis
                    .mana_ability_indices
                    .insert(perm_id, ability_summary.mana_ability_indices().to_vec());
            }
            if !ability_summary.activated_ability_indices().is_empty() {
                analysis.activated_ability_indices.insert(
                    perm_id,
                    ability_summary.activated_ability_indices().to_vec(),
                );
            }

            let mut activatable_indices = Vec::new();
            let mut first_output = None;

            for &ability_index in ability_summary.mana_ability_indices() {
                let Some(ability) = abilities.get(ability_index) else {
                    continue;
                };
                let Some(output) = crate::decision::simple_battlefield_mana_ability_output(
                    self.game,
                    player,
                    perm_id,
                    ability_index,
                    ability,
                    self,
                ) else {
                    continue;
                };
                activatable_indices.push(ability_index);
                if first_output.is_none() {
                    first_output = Some(output);
                }
            }

            if !activatable_indices.is_empty() {
                analysis
                    .activatable_indices
                    .insert(perm_id, activatable_indices);
            }
            if let Some(output) = first_output {
                analysis.first_output_by_permanent.insert(perm_id, output);
            }
        }

        let analysis = Rc::new(analysis);
        self.simple_battlefield_mana_analysis
            .borrow_mut()
            .insert(player, Rc::clone(&analysis));
        analysis
    }

    pub(crate) fn granted_alternative_casts_for_card(
        &self,
        card_id: ObjectId,
        zone: Zone,
        player: PlayerId,
    ) -> Vec<GrantedAlternativeCast> {
        let key = (card_id, zone, player);
        if let Some(cached) = self.granted_alternative_casts.borrow().get(&key) {
            return cached.clone();
        }

        let Some(card) = self.game.object(card_id) else {
            return Vec::new();
        };
        let ctx = self.game.filter_context_for(player, None);
        let grants = self.active_grants();
        let grants: Vec<_> = grants
            .iter()
            .filter(|grant| grant.player == player && grant.zone == zone)
            .filter(|grant| grant_applies_to_card(grant, card_id, card, &ctx, self.game))
            .filter_map(|grant| match &grant.grantable {
                Grantable::AlternativeCast(method) => Some(GrantedAlternativeCast {
                    method: method.clone(),
                    source_id: grant.source.source_id(),
                    zone: grant.zone,
                }),
                Grantable::DerivedAlternativeCast(spec) => {
                    materialize_derived_alternative_cast(card, spec).map(|method| {
                        GrantedAlternativeCast {
                            method,
                            source_id: grant.source.source_id(),
                            zone: grant.zone,
                        }
                    })
                }
                Grantable::Ability(_) | Grantable::PlayFrom => None,
            })
            .collect();
        self.granted_alternative_casts
            .borrow_mut()
            .insert(key, grants.clone());
        grants
    }

    pub(crate) fn granted_play_from_for_card(
        &self,
        card_id: ObjectId,
        zone: Zone,
        player: PlayerId,
    ) -> Vec<GrantedPlayFrom> {
        let key = (card_id, zone, player);
        if let Some(cached) = self.granted_play_from.borrow().get(&key) {
            return cached.clone();
        }

        let Some(card) = self.game.object(card_id) else {
            return Vec::new();
        };
        let ctx = self.game.filter_context_for(player, None);
        let grants = self.active_grants();
        let grants: Vec<_> = grants
            .iter()
            .filter(|grant| grant.player == player && grant.zone == zone)
            .filter(|grant| grant_applies_to_card(grant, card_id, card, &ctx, self.game))
            .filter_map(|grant| match &grant.grantable {
                Grantable::PlayFrom => Some(GrantedPlayFrom {
                    source_id: grant.source.source_id(),
                    zone: grant.zone,
                }),
                Grantable::Ability(_)
                | Grantable::AlternativeCast(_)
                | Grantable::DerivedAlternativeCast(_) => None,
            })
            .collect();
        self.granted_play_from
            .borrow_mut()
            .insert(key, grants.clone());
        grants
    }

    pub(crate) fn card_has_granted_static_ability_id(
        &self,
        card_id: ObjectId,
        zone: Zone,
        player: PlayerId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        let key = (card_id, zone, player, ability_id);
        if let Some(cached) = self.granted_static_ability_presence.borrow().get(&key) {
            return *cached;
        }

        let Some(card) = self.game.object(card_id) else {
            return false;
        };
        let ctx = self.game.filter_context_for(player, None);
        let grants = self.active_grants();
        let has_ability = grants.iter().any(|grant| {
            grant.player == player
                && grant.zone == zone
                && grant_applies_to_card(grant, card_id, card, &ctx, self.game)
                && matches!(
                    &grant.grantable,
                    Grantable::Ability(ability) if ability.id() == ability_id
                )
        });
        self.granted_static_ability_presence
            .borrow_mut()
            .insert(key, has_ability);
        has_ability
    }

    pub(crate) fn player_has_active_grants_for_zone(&self, player: PlayerId, zone: Zone) -> bool {
        let key = (player, zone);
        if let Some(cached) = self.active_grant_zone_presence.borrow().get(&key) {
            return *cached;
        }

        let has_grants = self
            .active_grants()
            .iter()
            .any(|grant| grant.player == player && grant.zone == zone);
        self.active_grant_zone_presence
            .borrow_mut()
            .insert(key, has_grants);
        has_grants
    }

    pub(crate) fn battlefield_spell_cost_modifier_sources(&self) -> Vec<ObjectId> {
        if let Some(cached) = self
            .battlefield_spell_cost_modifier_sources
            .borrow()
            .as_ref()
        {
            return cached.clone();
        }

        let sources: Vec<_> = self
            .game
            .battlefield
            .iter()
            .copied()
            .filter(|&perm_id| self.permanent_has_spell_cost_modifiers(perm_id))
            .collect();
        *self.has_battlefield_spell_cost_modifiers.borrow_mut() = Some(!sources.is_empty());
        *self.battlefield_spell_cost_modifier_sources.borrow_mut() = Some(sources.clone());
        sources
    }

    pub(crate) fn has_battlefield_spell_cost_modifiers(&self) -> bool {
        if let Some(cached) = *self.has_battlefield_spell_cost_modifiers.borrow() {
            return cached;
        }

        let has_modifiers = self
            .game
            .battlefield
            .iter()
            .copied()
            .any(|perm_id| self.permanent_has_spell_cost_modifiers(perm_id));
        *self.has_battlefield_spell_cost_modifiers.borrow_mut() = Some(has_modifiers);
        has_modifiers
    }

    pub(crate) fn activated_ability_cost_modifier_sources(&self) -> Vec<ObjectId> {
        if let Some(cached) = self
            .activated_ability_cost_modifier_sources
            .borrow()
            .as_ref()
        {
            return cached.clone();
        }

        let sources: Vec<_> = self
            .game
            .battlefield
            .iter()
            .copied()
            .filter(|&perm_id| self.permanent_has_activated_ability_cost_modifiers(perm_id))
            .collect();
        *self.has_activated_ability_cost_modifiers.borrow_mut() = Some(!sources.is_empty());
        *self.activated_ability_cost_modifier_sources.borrow_mut() = Some(sources.clone());
        sources
    }

    pub(crate) fn has_activated_ability_cost_modifiers(&self) -> bool {
        if let Some(cached) = *self.has_activated_ability_cost_modifiers.borrow() {
            return cached;
        }

        let has_modifiers = self
            .game
            .battlefield
            .iter()
            .copied()
            .any(|perm_id| self.permanent_has_activated_ability_cost_modifiers(perm_id));
        *self.has_activated_ability_cost_modifiers.borrow_mut() = Some(has_modifiers);
        has_modifiers
    }

    pub(crate) fn spell_has_legal_targets(
        &self,
        effects: &[crate::effect::Effect],
        caster: PlayerId,
        source_id: Option<ObjectId>,
        chosen_modes: Option<&[usize]>,
    ) -> bool {
        let key = SpellTargetLegalityKey {
            source_id,
            effects_ptr: effects.as_ptr() as usize,
            effects_len: effects.len(),
            chosen_modes: chosen_modes.map_or_else(Vec::new, |modes| modes.to_vec()),
        };
        if let Some(cached) = self.spell_target_legality.borrow().get(&key) {
            return *cached;
        }

        let result = crate::game_loop::spell_has_legal_targets_with_modes_and_view(
            self.game,
            effects,
            caster,
            source_id,
            chosen_modes,
            self,
        );
        self.spell_target_legality.borrow_mut().insert(key, result);
        result
    }

    fn active_grants(&self) -> Rc<Vec<Grant>> {
        if let Some(cached) = self.active_grants.borrow().as_ref() {
            return Rc::clone(cached);
        }

        let grants = Rc::new(self.game.grant_registry.active_grants(self.game));
        *self.active_grants.borrow_mut() = Some(Rc::clone(&grants));
        grants
    }

    fn permanent_has_spell_cost_modifiers(&self, permanent_id: ObjectId) -> bool {
        self.static_abilities_rc(permanent_id)
            .unwrap_or_default()
            .iter()
            .any(|static_ability| {
                static_ability.cost_reduction().is_some()
                    || static_ability.cost_increase().is_some()
                    || static_ability.cost_reduction_mana_cost().is_some()
                    || static_ability.cost_increase_mana_cost().is_some()
            })
    }

    fn permanent_has_activated_ability_cost_modifiers(&self, permanent_id: ObjectId) -> bool {
        self.static_abilities_rc(permanent_id)
            .unwrap_or_default()
            .iter()
            .any(|static_ability| static_ability.activated_ability_cost_reduction().is_some())
    }

    fn narrow_battlefield_candidates(
        &self,
        filter: &ObjectFilter,
        filter_ctx: &crate::filter::FilterContext,
    ) -> Option<Vec<ObjectId>> {
        use crate::target::PlayerFilter;
        use crate::types::CardType;

        if filter.zone != Some(Zone::Battlefield) || !filter.any_of.is_empty() {
            return None;
        }

        if let Some(id) = filter.specific {
            return Some(vec![id]);
        }

        let uses_creature_subset = filter.card_types.contains(&CardType::Creature)
            || filter.all_card_types.contains(&CardType::Creature);
        let uses_noncreature_subset =
            !uses_creature_subset && filter.excluded_card_types.contains(&CardType::Creature);

        let base = if uses_creature_subset {
            self.battlefield_creature_candidates()
        } else if uses_noncreature_subset {
            self.battlefield_noncreature_candidates()
        } else {
            self.candidate_ids_for_zone(Some(Zone::Battlefield))
        };

        match filter.controller.as_ref() {
            Some(PlayerFilter::You) => filter_ctx.you.map(|player| {
                if uses_creature_subset {
                    self.battlefield_controlled_creature_candidates(player)
                } else if !uses_noncreature_subset {
                    self.battlefield_controlled_candidates(player)
                } else {
                    self.filter_candidates_by_controller(base, &[player])
                }
            }),
            Some(PlayerFilter::Specific(player)) => Some(if uses_creature_subset {
                self.battlefield_controlled_creature_candidates(*player)
            } else if !uses_noncreature_subset {
                self.battlefield_controlled_candidates(*player)
            } else {
                self.filter_candidates_by_controller(base, &[*player])
            }),
            Some(PlayerFilter::Opponent) | Some(PlayerFilter::NotYou) => {
                filter_ctx.you.map(|player| {
                    if uses_creature_subset {
                        self.battlefield_opponent_creature_candidates(player)
                    } else {
                        self.battlefield_opponent_candidates(player)
                    }
                })
            }
            _ => Some(base),
        }
    }

    fn battlefield_creature_candidates(&self) -> Vec<ObjectId> {
        if let Some(cached) = self.battlefield_creatures.borrow().as_ref() {
            return cached.clone();
        }

        let ids: Vec<_> = self
            .game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| self.object_has_card_type(id, CardType::Creature))
            .collect();
        *self.battlefield_creatures.borrow_mut() = Some(ids.clone());
        ids
    }

    fn battlefield_noncreature_candidates(&self) -> Vec<ObjectId> {
        if let Some(cached) = self.battlefield_noncreatures.borrow().as_ref() {
            return cached.clone();
        }

        let ids: Vec<_> = self
            .game
            .battlefield
            .iter()
            .copied()
            .filter(|&id| !self.object_has_card_type(id, CardType::Creature))
            .collect();
        *self.battlefield_noncreatures.borrow_mut() = Some(ids.clone());
        ids
    }

    fn battlefield_controlled_candidates(&self, player: PlayerId) -> Vec<ObjectId> {
        if let Some(cached) = self.battlefield_controlled.borrow().get(&player) {
            return cached.clone();
        }

        let ids = self.filter_candidates_by_controller(
            self.candidate_ids_for_zone(Some(Zone::Battlefield)),
            &[player],
        );
        self.battlefield_controlled
            .borrow_mut()
            .insert(player, ids.clone());
        ids
    }

    fn battlefield_controlled_creature_candidates(&self, player: PlayerId) -> Vec<ObjectId> {
        if let Some(cached) = self.battlefield_controlled_creatures.borrow().get(&player) {
            return cached.clone();
        }

        let ids =
            self.filter_candidates_by_controller(self.battlefield_creature_candidates(), &[player]);
        self.battlefield_controlled_creatures
            .borrow_mut()
            .insert(player, ids.clone());
        ids
    }

    fn battlefield_opponent_candidates(&self, player: PlayerId) -> Vec<ObjectId> {
        if let Some(cached) = self.battlefield_opponents.borrow().get(&player) {
            return cached.clone();
        }

        let ids: Vec<_> = self
            .candidate_ids_for_zone(Some(Zone::Battlefield))
            .into_iter()
            .filter(|id| {
                self.game
                    .object(*id)
                    .is_some_and(|object| object.controller != player)
            })
            .collect();
        self.battlefield_opponents
            .borrow_mut()
            .insert(player, ids.clone());
        ids
    }

    fn battlefield_opponent_creature_candidates(&self, player: PlayerId) -> Vec<ObjectId> {
        if let Some(cached) = self.battlefield_opponent_creatures.borrow().get(&player) {
            return cached.clone();
        }

        let ids: Vec<_> = self
            .battlefield_creature_candidates()
            .into_iter()
            .filter(|id| {
                self.game
                    .object(*id)
                    .is_some_and(|object| object.controller != player)
            })
            .collect();
        self.battlefield_opponent_creatures
            .borrow_mut()
            .insert(player, ids.clone());
        ids
    }

    fn filter_candidates_by_controller(
        &self,
        candidates: Vec<ObjectId>,
        controllers: &[PlayerId],
    ) -> Vec<ObjectId> {
        candidates
            .into_iter()
            .filter(|id| {
                self.game
                    .object(*id)
                    .is_some_and(|object| controllers.contains(&object.controller))
            })
            .collect()
    }

    pub(crate) fn requires_battlefield_characteristic_calculation(
        &self,
        object_id: ObjectId,
    ) -> bool {
        let Some(object) = self.game.object(object_id) else {
            return true;
        };
        if object.zone != Zone::Battlefield {
            return true;
        }
        if self.game.is_face_down(object_id) {
            return true;
        }
        self.battlefield_characteristic_scope.includes(object_id)
    }
}

fn grant_applies_to_card(
    grant: &Grant,
    card_id: ObjectId,
    card: &crate::object::Object,
    ctx: &crate::filter::FilterContext,
    game: &GameState,
) -> bool {
    if let Some(target_id) = grant.target_id {
        return target_id == card_id;
    }

    grant
        .filter
        .as_ref()
        .is_some_and(|filter| filter.matches(card, ctx, game))
}

fn materialize_derived_alternative_cast(
    card: &crate::object::Object,
    spec: &DerivedAlternativeCast,
) -> Option<crate::alternative_cast::AlternativeCastingMethod> {
    spec.materialize_for(card)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::continuous::{ContinuousEffect, Modification, TextBoxOverlay};
    use crate::effect::Until;
    use crate::ids::{ObjectId, PlayerId};

    #[test]
    fn battlefield_characteristic_scope_uses_locked_targets_for_resolution_effects() {
        let effects = vec![
            ContinuousEffect::from_resolution(
                ObjectId::from_raw(10),
                PlayerId::from_index(0),
                vec![ObjectId::from_raw(2)],
                Modification::SetTextBox(TextBoxOverlay::new(String::new(), Vec::new())),
            )
            .until(Until::EndOfTurn),
        ];

        assert_eq!(
            battlefield_characteristic_scope(&effects),
            BattlefieldCharacteristicScope::Specific(vec![ObjectId::from_raw(2)]),
        );
    }

    #[test]
    fn battlefield_characteristic_scope_falls_back_to_all_battlefield_for_filter_effects() {
        let effects = vec![ContinuousEffect::new(
            ObjectId::from_raw(10),
            PlayerId::from_index(0),
            EffectTarget::AllCreatures,
            Modification::AddAbilityGeneric(Ability::static_ability(
                crate::static_abilities::StaticAbility::flying(),
            )),
        )];

        assert_eq!(
            battlefield_characteristic_scope(&effects),
            BattlefieldCharacteristicScope::AllBattlefield,
        );
    }
}
