use crate::filter::ObjectFilterExt as _;
use std::cell::RefCell;
use std::collections::HashSet;

use crate::FxMap;
use std::rc::Rc;
use std::sync::Arc;

use crate::ability::{Ability, AbilityKind, ActivatedAbilityRuntimeExt as _};
use crate::continuous::{
    CalculatedCharacteristics, ContinuousEffect, EffectTarget, Layer, Modification,
};
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
    all_effects: Arc<Vec<ContinuousEffect>>,
    battlefield_characteristic_scope: BattlefieldCharacteristicScope,
    use_game_characteristics_cache: bool,
    characteristics: RefCell<FxMap<ObjectId, Option<Arc<CalculatedCharacteristics>>>>,
    abilities_cache: RefCell<FxMap<ObjectId, Rc<Vec<Ability>>>>,
    ability_index_summary_cache: RefCell<FxMap<ObjectId, Rc<AbilityIndexSummary>>>,
    static_abilities_cache:
        RefCell<FxMap<ObjectId, Rc<Vec<crate::static_abilities::StaticAbility>>>>,
    zone_candidates: RefCell<FxMap<Option<Zone>, Vec<ObjectId>>>,
    battlefield_creatures: RefCell<Option<Vec<ObjectId>>>,
    battlefield_noncreatures: RefCell<Option<Vec<ObjectId>>>,
    battlefield_controlled: RefCell<FxMap<PlayerId, Vec<ObjectId>>>,
    battlefield_controlled_creatures: RefCell<FxMap<PlayerId, Vec<ObjectId>>>,
    battlefield_opponents: RefCell<FxMap<PlayerId, Vec<ObjectId>>>,
    battlefield_opponent_creatures: RefCell<FxMap<PlayerId, Vec<ObjectId>>>,
    potential_mana: RefCell<FxMap<PlayerId, ManaPool>>,
    potential_mana_compute_ms: RefCell<f64>,
    black_mana_life_permission: RefCell<FxMap<PlayerId, bool>>,
    pay_life_cast_or_activate_restriction: RefCell<FxMap<PlayerId, bool>>,
    granted_alternative_casts:
        RefCell<FxMap<(ObjectId, Zone, PlayerId), Vec<GrantedAlternativeCast>>>,
    granted_play_from: RefCell<FxMap<(ObjectId, Zone, PlayerId), Vec<GrantedPlayFrom>>>,
    granted_static_ability_presence: RefCell<
        FxMap<
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
    active_grant_zone_presence: RefCell<FxMap<(PlayerId, Zone), bool>>,
    battlefield_spell_cost_modifier_sources: RefCell<Option<Vec<ObjectId>>>,
    activated_ability_cost_modifier_sources: RefCell<Option<Vec<ObjectId>>>,
    has_battlefield_spell_cost_modifiers: RefCell<Option<bool>>,
    has_activated_ability_cost_modifiers: RefCell<Option<bool>>,
    simple_battlefield_mana_analysis: RefCell<FxMap<PlayerId, Rc<SimpleBattlefieldManaAnalysis>>>,
    spell_target_legality: RefCell<FxMap<SpellTargetLegalityKey, bool>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SpellTargetLegalityKey {
    caster: PlayerId,
    source_id: Option<ObjectId>,
    effects_ptr: usize,
    effects_len: usize,
    chosen_modes: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SimpleBattlefieldManaAnalysis {
    relevant_source_ids: Vec<ObjectId>,
    mana_source_ids: Vec<ObjectId>,
    activatable_indices: FxMap<ObjectId, Vec<usize>>,
    mana_ability_indices: FxMap<ObjectId, Vec<usize>>,
    activated_ability_indices: FxMap<ObjectId, Vec<usize>>,
    first_output_by_permanent: FxMap<ObjectId, Vec<ManaSymbol>>,
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
    game: &GameState,
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
            EffectTarget::Specific(id) => {
                if !specific_ids.contains(id) {
                    specific_ids.push(*id);
                }
            }
            EffectTarget::AttachedTo(id) => {
                if !specific_ids.contains(id) {
                    specific_ids.push(*id);
                }
                if let Some(attached_to) = game
                    .object(*id)
                    .and_then(|object| object.attached_to)
                    .and_then(|target| target.object_id())
                    && !specific_ids.contains(&attached_to)
                {
                    specific_ids.push(attached_to);
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

fn continuous_effect_can_change_spell_cost_modifier_presence(effect: &ContinuousEffect) -> bool {
    modification_can_change_spell_cost_modifier_presence(&effect.modification)
}

fn continuous_effect_can_change_activated_ability_cost_modifier_presence(
    effect: &ContinuousEffect,
) -> bool {
    modification_can_change_activated_ability_cost_modifier_presence(&effect.modification)
}

fn continuous_effect_can_change_minimum_total_spell_mana_presence(
    effect: &ContinuousEffect,
) -> bool {
    modification_can_change_minimum_total_spell_mana_presence(&effect.modification)
}

fn modification_can_change_spell_cost_modifier_presence(modification: &Modification) -> bool {
    match modification {
        Modification::CopyOf { .. }
        | Modification::ChangeText { .. }
        | Modification::SetTextBox(_)
        | Modification::SetAbilities(_)
        | Modification::RemoveAllAbilities
        | Modification::RemoveAllAbilitiesExceptMana => true,
        Modification::AddAbility(static_ability) | Modification::RemoveAbility(static_ability) => {
            static_ability_has_spell_cost_modifier(static_ability)
        }
        Modification::AddAbilityGeneric(ability)
        | Modification::RemoveAbilityGeneric { ability, .. } => {
            ability_has_spell_cost_modifier(ability)
        }
        _ => false,
    }
}

fn modification_can_change_activated_ability_cost_modifier_presence(
    modification: &Modification,
) -> bool {
    match modification {
        Modification::CopyOf { .. }
        | Modification::ChangeText { .. }
        | Modification::SetTextBox(_)
        | Modification::SetAbilities(_)
        | Modification::RemoveAllAbilities
        | Modification::RemoveAllAbilitiesExceptMana => true,
        Modification::AddAbility(static_ability) | Modification::RemoveAbility(static_ability) => {
            static_ability_has_activated_ability_cost_modifier(static_ability)
        }
        Modification::AddAbilityGeneric(ability)
        | Modification::RemoveAbilityGeneric { ability, .. } => {
            ability_has_activated_ability_cost_modifier(ability)
        }
        _ => false,
    }
}

fn modification_can_change_minimum_total_spell_mana_presence(modification: &Modification) -> bool {
    match modification {
        Modification::CopyOf { .. }
        | Modification::ChangeText { .. }
        | Modification::SetTextBox(_)
        | Modification::SetAbilities(_)
        | Modification::RemoveAllAbilities
        | Modification::RemoveAllAbilitiesExceptMana => true,
        Modification::AddAbility(static_ability) | Modification::RemoveAbility(static_ability) => {
            static_ability_has_minimum_total_spell_mana(static_ability)
        }
        Modification::AddAbilityGeneric(ability)
        | Modification::RemoveAbilityGeneric { ability, .. } => {
            ability_has_minimum_total_spell_mana(ability)
        }
        _ => false,
    }
}

fn ability_has_spell_cost_modifier(ability: &Ability) -> bool {
    matches!(&ability.kind, AbilityKind::Static(static_ability)
        if static_ability_has_spell_cost_modifier(static_ability))
}

fn ability_has_activated_ability_cost_modifier(ability: &Ability) -> bool {
    matches!(&ability.kind, AbilityKind::Static(static_ability)
        if static_ability_has_activated_ability_cost_modifier(static_ability))
}

fn ability_has_minimum_total_spell_mana(ability: &Ability) -> bool {
    matches!(&ability.kind, AbilityKind::Static(static_ability)
        if static_ability_has_minimum_total_spell_mana(static_ability))
}

fn static_ability_has_spell_cost_modifier(
    static_ability: &crate::static_abilities::StaticAbility,
) -> bool {
    static_ability.cost_reduction().is_some()
        || static_ability.cost_increase().is_some()
        || static_ability.cost_reduction_mana_cost().is_some()
        || static_ability.cost_increase_mana_cost().is_some()
}

fn static_ability_has_activated_ability_cost_modifier(
    static_ability: &crate::static_abilities::StaticAbility,
) -> bool {
    static_ability.activated_ability_cost_reduction().is_some()
        || static_ability.activated_ability_cost_increase().is_some()
}

fn static_ability_has_minimum_total_spell_mana(
    static_ability: &crate::static_abilities::StaticAbility,
) -> bool {
    static_ability.minimum_total_spell_mana().is_some()
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
        let all_effects = game.cached_continuous_effects_snapshot_arc();
        game.count_derived_view_rebuild();
        Self {
            game,
            battlefield_characteristic_scope: battlefield_characteristic_scope(
                game,
                all_effects.as_slice(),
            ),
            all_effects,
            use_game_characteristics_cache: true,
            characteristics: RefCell::new(FxMap::default()),
            abilities_cache: RefCell::new(FxMap::default()),
            ability_index_summary_cache: RefCell::new(FxMap::default()),
            static_abilities_cache: RefCell::new(FxMap::default()),
            zone_candidates: RefCell::new(FxMap::default()),
            battlefield_creatures: RefCell::new(None),
            battlefield_noncreatures: RefCell::new(None),
            battlefield_controlled: RefCell::new(FxMap::default()),
            battlefield_controlled_creatures: RefCell::new(FxMap::default()),
            battlefield_opponents: RefCell::new(FxMap::default()),
            battlefield_opponent_creatures: RefCell::new(FxMap::default()),
            potential_mana: RefCell::new(FxMap::default()),
            potential_mana_compute_ms: RefCell::new(0.0),
            black_mana_life_permission: RefCell::new(FxMap::default()),
            pay_life_cast_or_activate_restriction: RefCell::new(FxMap::default()),
            granted_alternative_casts: RefCell::new(FxMap::default()),
            granted_play_from: RefCell::new(FxMap::default()),
            granted_static_ability_presence: RefCell::new(FxMap::default()),
            active_grants: RefCell::new(None),
            active_grant_zone_presence: RefCell::new(FxMap::default()),
            battlefield_spell_cost_modifier_sources: RefCell::new(None),
            activated_ability_cost_modifier_sources: RefCell::new(None),
            has_battlefield_spell_cost_modifiers: RefCell::new(None),
            has_activated_ability_cost_modifiers: RefCell::new(None),
            simple_battlefield_mana_analysis: RefCell::new(FxMap::default()),
            spell_target_legality: RefCell::new(FxMap::default()),
        }
    }

    pub(crate) fn from_effects(game: &'a GameState, all_effects: Vec<ContinuousEffect>) -> Self {
        game.count_derived_view_rebuild();
        let all_effects = Arc::new(all_effects);
        Self {
            game,
            battlefield_characteristic_scope: battlefield_characteristic_scope(
                game,
                all_effects.as_slice(),
            ),
            all_effects,
            use_game_characteristics_cache: false,
            characteristics: RefCell::new(FxMap::default()),
            abilities_cache: RefCell::new(FxMap::default()),
            ability_index_summary_cache: RefCell::new(FxMap::default()),
            static_abilities_cache: RefCell::new(FxMap::default()),
            zone_candidates: RefCell::new(FxMap::default()),
            battlefield_creatures: RefCell::new(None),
            battlefield_noncreatures: RefCell::new(None),
            battlefield_controlled: RefCell::new(FxMap::default()),
            battlefield_controlled_creatures: RefCell::new(FxMap::default()),
            battlefield_opponents: RefCell::new(FxMap::default()),
            battlefield_opponent_creatures: RefCell::new(FxMap::default()),
            potential_mana: RefCell::new(FxMap::default()),
            potential_mana_compute_ms: RefCell::new(0.0),
            black_mana_life_permission: RefCell::new(FxMap::default()),
            pay_life_cast_or_activate_restriction: RefCell::new(FxMap::default()),
            granted_alternative_casts: RefCell::new(FxMap::default()),
            granted_play_from: RefCell::new(FxMap::default()),
            granted_static_ability_presence: RefCell::new(FxMap::default()),
            active_grants: RefCell::new(None),
            active_grant_zone_presence: RefCell::new(FxMap::default()),
            battlefield_spell_cost_modifier_sources: RefCell::new(None),
            activated_ability_cost_modifier_sources: RefCell::new(None),
            has_battlefield_spell_cost_modifiers: RefCell::new(None),
            has_activated_ability_cost_modifiers: RefCell::new(None),
            simple_battlefield_mana_analysis: RefCell::new(FxMap::default()),
            spell_target_legality: RefCell::new(FxMap::default()),
        }
    }

    pub(crate) fn effects(&self) -> &[ContinuousEffect] {
        self.all_effects.as_slice()
    }

    pub(crate) fn effects_arc(&self) -> Arc<Vec<ContinuousEffect>> {
        Arc::clone(&self.all_effects)
    }

    pub(crate) fn calculated_characteristics_arc(
        &self,
        object_id: ObjectId,
    ) -> Option<Arc<CalculatedCharacteristics>> {
        if let Some(cached) = self.characteristics.borrow().get(&object_id) {
            return cached.clone();
        }

        let calculated = if self.use_game_characteristics_cache {
            self.game.calculated_characteristics_arc(object_id)
        } else {
            self.game
                .calculated_characteristics_with_effects(object_id, self.all_effects.as_slice())
                .map(Arc::new)
        };
        self.characteristics
            .borrow_mut()
            .insert(object_id, calculated.clone());
        calculated
    }

    pub(crate) fn calculated_characteristics(
        &self,
        object_id: ObjectId,
    ) -> Option<CalculatedCharacteristics> {
        self.calculated_characteristics_arc(object_id)
            .map(|chars| chars.as_ref().clone())
    }

    /// Return current characteristics for objects in any zone while retaining
    /// this view's pass-local cache. Nonbattlefield changeling expansion
    /// mirrors `GameState::current_characteristics`.
    pub(crate) fn current_characteristics_arc(
        &self,
        object_id: ObjectId,
    ) -> Option<Arc<CalculatedCharacteristics>> {
        let object = self.game.object(object_id)?;
        let chars = self.calculated_characteristics_arc(object_id)?;
        if object.zone == Zone::Battlefield {
            return Some(chars);
        }

        let has_changeling = chars
            .static_abilities
            .iter()
            .any(|ability| ability.id() == crate::static_abilities::StaticAbilityId::Changeling);
        let can_have_creature_subtypes = chars
            .card_types
            .iter()
            .any(|card_type| matches!(card_type, CardType::Creature | CardType::Kindred));
        if !has_changeling || !can_have_creature_subtypes {
            return Some(chars);
        }

        let mut expanded = chars.as_ref().clone();
        let mut changed = false;
        for subtype in Subtype::all_creature_types() {
            if !expanded.subtypes.contains(subtype) {
                expanded.subtypes.push(*subtype);
                changed = true;
            }
        }
        if changed {
            Some(Arc::new(expanded))
        } else {
            Some(chars)
        }
    }

    pub(crate) fn prewarm_characteristics(&self, ids: &[ObjectId]) {
        let required: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| self.requires_battlefield_characteristic_calculation(*id))
            .collect();
        self.prewarm_characteristics_forced(&required);
    }

    /// Batch explicitly requested objects even when their printed
    /// characteristics would normally be enough for other DerivedGameView
    /// helpers. This is used when a caller truly needs current characteristics
    /// for a nonbattlefield object and wants to avoid a singleton full-board
    /// baseline calculation.
    pub(crate) fn prewarm_characteristics_forced(&self, ids: &[ObjectId]) {
        let missing: Vec<_> = {
            let cache = self.characteristics.borrow();
            ids.iter()
                .copied()
                .filter(|id| !cache.contains_key(id))
                .collect()
        };
        if missing.is_empty() {
            return;
        }

        if self.use_game_characteristics_cache {
            self.game.prewarm_calculated_characteristics(&missing);
            let mut cache = self.characteristics.borrow_mut();
            for id in missing {
                cache.insert(id, self.game.calculated_characteristics_arc(id));
            }
            return;
        }

        let calculated = self
            .game
            .calculated_characteristics_batch_with_effects(&missing, self.all_effects.as_slice());
        let mut cache = self.characteristics.borrow_mut();
        for id in missing {
            cache.insert(id, calculated.get(&id).cloned().map(Arc::new));
        }
    }

    pub(crate) fn calculated_toughness(&self, object_id: ObjectId) -> Option<i32> {
        self.calculated_characteristics_arc(object_id)
            .and_then(|chars| chars.toughness)
    }

    pub(crate) fn calculated_subtypes(&self, object_id: ObjectId) -> Vec<Subtype> {
        self.calculated_characteristics_arc(object_id)
            .map(|chars| chars.subtypes.to_vec())
            .unwrap_or_default()
    }

    pub(crate) fn object_colors(&self, object_id: ObjectId) -> crate::color::ColorSet {
        let Some(object) = self.game.object(object_id) else {
            return crate::color::ColorSet::default();
        };
        if !self.requires_battlefield_characteristic_calculation(object_id) {
            return object.colors();
        }

        self.calculated_characteristics_arc(object_id)
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
        let needs_calculated_abilities = self
            .requires_battlefield_characteristic_calculation(object_id)
            || (self.game.deploy_creatures_enabled() && object.zone == Zone::Battlefield);
        let abilities = if !needs_calculated_abilities {
            object.abilities_vec()
        } else {
            self.calculated_characteristics_arc(object_id)?
                .abilities
                .to_vec()
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
        let controller = self
            .current_controller(object_id)
            .unwrap_or_else(|| self.game.controller_of(object));
        for (ability_index, ability) in abilities.iter().enumerate() {
            if !ability.functions_in(&object.zone) {
                continue;
            }
            if let AbilityKind::Activated(activated) = &ability.kind {
                if activated.is_runtime_mana_ability(self.game, object_id, controller) {
                    summary.mana_ability_indices.push(ability_index);
                } else {
                    summary.activated_ability_indices.push(ability_index);
                }
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
            self.calculated_characteristics_arc(object_id)?
                .static_abilities
                .to_vec()
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

        self.calculated_characteristics_arc(object_id)
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
                    if ability.functions_in(&object.zone)
                        && static_ability.id() == ability_id
                        && static_ability.is_active(self.game, object_id))
            });
        }

        self.calculated_characteristics_arc(object_id)
            .is_some_and(|chars| {
                chars.static_abilities.iter().any(|ability| {
                    ability.id() == ability_id && ability.is_active(self.game, object_id)
                })
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
        let mana_spend_policy = self.game.mana_spend_policy(player, source);
        let allow_black_life = crate::decision::mana_cost_has_black_symbol(cost)
            && self.player_can_pay_black_with_life_for_reason(player, reason);
        crate::decision::can_pay_mana_cost_with_available_sources(
            self.game,
            player,
            source,
            cost,
            x_value,
            reason,
            &mana_spend_policy,
            allow_black_life,
            self,
        )
    }

    pub(crate) fn player_can_pay_black_with_life_for_reason(
        &self,
        payer: PlayerId,
        reason: crate::costs::PaymentReason,
    ) -> bool {
        self.player_can_pay_black_with_life(payer)
            && (!reason.is_cast_or_ability_payment()
                || !self.player_cant_pay_life_to_cast_or_activate(payer))
    }

    fn player_can_pay_black_with_life(&self, payer: PlayerId) -> bool {
        if let Some(cached) = self.black_mana_life_permission.borrow().get(&payer) {
            return *cached;
        }

        self.prewarm_characteristics(&self.game.battlefield);
        let result = self.game.battlefield.iter().copied().any(|perm_id| {
            self.current_controller(perm_id) == Some(payer)
                && self.static_abilities_rc(perm_id).is_some_and(|abilities| {
                    abilities.iter().any(|ability| {
                        ability.black_mana_may_be_paid_with_life()
                            && ability.is_active(self.game, perm_id)
                    })
                })
        });
        self.black_mana_life_permission
            .borrow_mut()
            .insert(payer, result);
        result
    }

    pub(crate) fn simple_battlefield_mana_analysis(
        &self,
        player: PlayerId,
    ) -> Rc<SimpleBattlefieldManaAnalysis> {
        if let Some(cached) = self.simple_battlefield_mana_analysis.borrow().get(&player) {
            return Rc::clone(cached);
        }

        let mut analysis = SimpleBattlefieldManaAnalysis::default();

        // This analysis may be requested immediately after a semantic mutation
        // (most notably after a spell is moved to the stack to begin payment).
        // In that state the view owns an explicit effect snapshot and individual
        // `abilities_rc` lookups would each run the full layer system.  Batch the
        // shared battlefield calculation once before classifying mana sources.
        self.prewarm_characteristics(&self.game.battlefield);

        for &perm_id in &self.game.battlefield {
            let Some(perm) = self.game.object(perm_id) else {
                continue;
            };
            if self.current_controller(perm_id) != Some(player)
                || !self.game.can_activate_abilities_of(perm_id)
            {
                continue;
            }

            let abilities = self
                .abilities_rc(perm_id)
                .unwrap_or_else(|| Rc::new(perm.abilities_vec()));
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
                    usage_limit: None,
                }),
                Grantable::DerivedAlternativeCast(spec) => {
                    materialize_derived_alternative_cast(card, spec).map(|method| {
                        GrantedAlternativeCast {
                            method,
                            source_id: grant.source.source_id(),
                            zone: grant.zone,
                            usage_limit: spec.usage_limit(),
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
                    usage_limit: grant.usage_limit,
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

    pub(crate) fn granted_alternative_casts_for_card_view(
        &self,
        card_id: ObjectId,
        card: &crate::object::Object,
        zone: Zone,
        player: PlayerId,
    ) -> Vec<GrantedAlternativeCast> {
        let ctx = self.game.filter_context_for(player, None);
        self.active_grants()
            .iter()
            .filter(|grant| grant.player == player && grant.zone == zone)
            .filter(|grant| {
                grant_applies_to_card_non_recursive(grant, card_id, card, &ctx, self.game)
            })
            .filter_map(|grant| match &grant.grantable {
                Grantable::AlternativeCast(method) => Some(GrantedAlternativeCast {
                    method: method.clone(),
                    source_id: grant.source.source_id(),
                    zone: grant.zone,
                    usage_limit: None,
                }),
                Grantable::DerivedAlternativeCast(spec) => {
                    materialize_derived_alternative_cast(card, spec).map(|method| {
                        GrantedAlternativeCast {
                            method,
                            source_id: grant.source.source_id(),
                            zone: grant.zone,
                            usage_limit: spec.usage_limit(),
                        }
                    })
                }
                Grantable::Ability(_) | Grantable::PlayFrom => None,
            })
            .collect()
    }

    pub(crate) fn granted_play_from_for_card_view(
        &self,
        card_id: ObjectId,
        card: &crate::object::Object,
        zone: Zone,
        player: PlayerId,
    ) -> Vec<GrantedPlayFrom> {
        let ctx = self.game.filter_context_for(player, None);
        self.active_grants()
            .iter()
            .filter(|grant| grant.player == player && grant.zone == zone)
            .filter(|grant| {
                grant_applies_to_card_non_recursive(grant, card_id, card, &ctx, self.game)
            })
            .filter_map(|grant| match &grant.grantable {
                Grantable::PlayFrom => Some(GrantedPlayFrom {
                    source_id: grant.source.source_id(),
                    zone: grant.zone,
                    usage_limit: grant.usage_limit,
                }),
                Grantable::Ability(_)
                | Grantable::AlternativeCast(_)
                | Grantable::DerivedAlternativeCast(_) => None,
            })
            .collect()
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

    pub(crate) fn card_view_has_granted_static_ability_id(
        &self,
        card_id: ObjectId,
        card: &crate::object::Object,
        zone: Zone,
        player: PlayerId,
        ability_id: crate::static_abilities::StaticAbilityId,
    ) -> bool {
        let ctx = self.game.filter_context_for(player, None);
        self.active_grants().iter().any(|grant| {
            grant.player == player
                && grant.zone == zone
                && grant_applies_to_card_non_recursive(grant, card_id, card, &ctx, self.game)
                && matches!(
                    &grant.grantable,
                    Grantable::Ability(ability) if ability.id() == ability_id
                )
        })
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

        let sources: Vec<_> = if self.can_scan_non_layered_spell_cost_modifiers() {
            self.game
                .battlefield
                .iter()
                .copied()
                .filter(|&perm_id| self.permanent_non_layered_has_spell_cost_modifiers(perm_id))
                .collect()
        } else {
            self.prewarm_characteristics(&self.game.battlefield);
            self.game
                .battlefield
                .iter()
                .copied()
                .filter(|&perm_id| self.permanent_has_spell_cost_modifiers(perm_id))
                .collect()
        };
        *self.has_battlefield_spell_cost_modifiers.borrow_mut() = Some(!sources.is_empty());
        *self.battlefield_spell_cost_modifier_sources.borrow_mut() = Some(sources.clone());
        sources
    }

    pub(crate) fn has_battlefield_spell_cost_modifiers(&self) -> bool {
        if let Some(cached) = *self.has_battlefield_spell_cost_modifiers.borrow() {
            return cached;
        }

        let has_modifiers = if self.can_scan_non_layered_spell_cost_modifiers() {
            self.game
                .battlefield
                .iter()
                .copied()
                .any(|perm_id| self.permanent_non_layered_has_spell_cost_modifiers(perm_id))
        } else {
            self.prewarm_characteristics(&self.game.battlefield);
            self.game
                .battlefield
                .iter()
                .copied()
                .any(|perm_id| self.permanent_has_spell_cost_modifiers(perm_id))
        };
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

        let sources: Vec<_> = if self.can_scan_non_layered_activated_ability_cost_modifiers() {
            self.game
                .battlefield
                .iter()
                .copied()
                .filter(|&perm_id| {
                    self.permanent_non_layered_has_activated_ability_cost_modifiers(perm_id)
                })
                .collect()
        } else {
            self.prewarm_characteristics(&self.game.battlefield);
            self.game
                .battlefield
                .iter()
                .copied()
                .filter(|&perm_id| self.permanent_has_activated_ability_cost_modifiers(perm_id))
                .collect()
        };
        *self.has_activated_ability_cost_modifiers.borrow_mut() = Some(!sources.is_empty());
        *self.activated_ability_cost_modifier_sources.borrow_mut() = Some(sources.clone());
        sources
    }

    pub(crate) fn has_activated_ability_cost_modifiers(&self) -> bool {
        if let Some(cached) = *self.has_activated_ability_cost_modifiers.borrow() {
            return cached;
        }

        let has_modifiers = if self.can_scan_non_layered_activated_ability_cost_modifiers() {
            self.game.battlefield.iter().copied().any(|perm_id| {
                self.permanent_non_layered_has_activated_ability_cost_modifiers(perm_id)
            })
        } else {
            self.prewarm_characteristics(&self.game.battlefield);
            self.game
                .battlefield
                .iter()
                .copied()
                .any(|perm_id| self.permanent_has_activated_ability_cost_modifiers(perm_id))
        };
        *self.has_activated_ability_cost_modifiers.borrow_mut() = Some(has_modifiers);
        has_modifiers
    }

    pub(crate) fn minimum_total_spell_mana_payment(&self) -> Option<u32> {
        let mut minimum = None;
        if self.can_scan_non_layered_minimum_total_spell_mana() {
            for &perm_id in &self.game.battlefield {
                self.for_each_active_non_layered_static_ability(perm_id, |static_ability| {
                    if let Some(candidate) = static_ability.minimum_total_spell_mana() {
                        minimum =
                            Some(minimum.map_or(candidate, |current: u32| current.max(candidate)));
                    }
                });
            }
            return minimum;
        }

        self.prewarm_characteristics(&self.game.battlefield);
        for &perm_id in &self.game.battlefield {
            let Some(static_abilities) = self.static_abilities_rc(perm_id) else {
                continue;
            };
            for static_ability in static_abilities.iter() {
                if !static_ability.is_active(self.game, perm_id) {
                    continue;
                }
                if let Some(candidate) = static_ability.minimum_total_spell_mana() {
                    minimum =
                        Some(minimum.map_or(candidate, |current: u32| current.max(candidate)));
                }
            }
        }
        minimum
    }

    pub(crate) fn player_cant_pay_life_to_cast_or_activate(&self, player: PlayerId) -> bool {
        if self.game.player(player).is_none() {
            return false;
        }
        if let Some(cached) = self
            .pay_life_cast_or_activate_restriction
            .borrow()
            .get(&player)
        {
            return *cached;
        }
        let result = if self.use_game_characteristics_cache {
            self.game.player_cant_pay_life_to_cast_or_activate(player)
        } else {
            self.game
                .player_cant_pay_life_to_cast_or_activate_with_effects(
                    player,
                    self.all_effects.as_slice(),
                )
        };
        self.pay_life_cast_or_activate_restriction
            .borrow_mut()
            .insert(player, result);
        result
    }

    pub(crate) fn spell_has_legal_targets(
        &self,
        effects: &[crate::effect::Effect],
        caster: PlayerId,
        source_id: Option<ObjectId>,
        chosen_modes: Option<&[usize]>,
    ) -> bool {
        let key = SpellTargetLegalityKey {
            caster,
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

        let grants = Rc::new(
            self.game
                .effect_store
                .grant_registry
                .active_grants(self.game),
        );
        *self.active_grants.borrow_mut() = Some(Rc::clone(&grants));
        grants
    }

    fn can_scan_non_layered_spell_cost_modifiers(&self) -> bool {
        !self
            .all_effects
            .iter()
            .any(continuous_effect_can_change_spell_cost_modifier_presence)
    }

    fn can_scan_non_layered_activated_ability_cost_modifiers(&self) -> bool {
        !self
            .all_effects
            .iter()
            .any(continuous_effect_can_change_activated_ability_cost_modifier_presence)
    }

    fn can_scan_non_layered_minimum_total_spell_mana(&self) -> bool {
        !self
            .all_effects
            .iter()
            .any(continuous_effect_can_change_minimum_total_spell_mana_presence)
    }

    fn permanent_non_layered_has_spell_cost_modifiers(&self, permanent_id: ObjectId) -> bool {
        let mut has_modifier = false;
        self.for_each_active_non_layered_static_ability(permanent_id, |static_ability| {
            has_modifier |= static_ability_has_spell_cost_modifier(static_ability);
        });
        has_modifier
    }

    fn permanent_has_spell_cost_modifiers(&self, permanent_id: ObjectId) -> bool {
        self.static_abilities_rc(permanent_id)
            .unwrap_or_default()
            .iter()
            .any(static_ability_has_spell_cost_modifier)
    }

    fn permanent_non_layered_has_activated_ability_cost_modifiers(
        &self,
        permanent_id: ObjectId,
    ) -> bool {
        let mut has_modifier = false;
        self.for_each_active_non_layered_static_ability(permanent_id, |static_ability| {
            has_modifier |= static_ability_has_activated_ability_cost_modifier(static_ability);
        });
        has_modifier
    }

    fn permanent_has_activated_ability_cost_modifiers(&self, permanent_id: ObjectId) -> bool {
        self.static_abilities_rc(permanent_id)
            .unwrap_or_default()
            .iter()
            .any(static_ability_has_activated_ability_cost_modifier)
    }

    fn for_each_active_non_layered_static_ability(
        &self,
        permanent_id: ObjectId,
        mut visit: impl FnMut(&crate::static_abilities::StaticAbility),
    ) {
        let Some(permanent) = self.game.object(permanent_id) else {
            return;
        };

        let has_level_abilities = permanent.abilities.iter().any(|ability| {
            matches!(&ability.kind, AbilityKind::Static(static_ability)
                if static_ability.level_abilities().is_some())
        });
        let tracks_grant_duplicates =
            has_level_abilities || !permanent.temporary_static_ability_grants.is_empty();
        let mut seen_abilities = tracks_grant_duplicates.then(Vec::new);

        for ability in permanent.abilities.iter() {
            let AbilityKind::Static(static_ability) = &ability.kind else {
                continue;
            };
            if let Some(seen) = seen_abilities.as_mut() {
                seen.push(static_ability.clone());
            }
            if ability.functions_in(&permanent.zone)
                && static_ability.is_active(self.game, permanent_id)
            {
                visit(static_ability);
            }
        }

        // Initial layered characteristics apply temporary grants before level
        // grants. A temporary payload is suppressed when a printed/earlier
        // temporary ability has the same ID, even when its payload differs.
        for grant in &permanent.temporary_static_ability_grants {
            if grant.is_expired(self.game.turn.turn_number) {
                continue;
            }
            let Some(static_ability) = grant.materialize() else {
                continue;
            };
            let seen = seen_abilities
                .as_mut()
                .expect("temporary grants should enable duplicate tracking");
            if seen
                .iter()
                .any(|existing| existing.id() == static_ability.id())
            {
                continue;
            }
            seen.push(static_ability.clone());
            if static_ability.is_active(self.game, permanent_id) {
                visit(&static_ability);
            }
        }

        // Level-granted abilities are appended later and use full ability
        // equality for deduplication, matching `push_static_ability_once`.
        if has_level_abilities {
            for static_ability in permanent.level_granted_abilities() {
                let seen = seen_abilities
                    .as_mut()
                    .expect("level grants should enable duplicate tracking");
                if seen.contains(&static_ability) {
                    continue;
                }
                seen.push(static_ability.clone());
                if static_ability.is_active(self.game, permanent_id) {
                    visit(&static_ability);
                }
            }
        }
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

        let uses_creature_subset = filter.all_card_types.contains(&CardType::Creature)
            || (!filter.type_or_subtype_union
                && filter.card_types.len() == 1
                && filter.card_types[0] == CardType::Creature);
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
                self.current_controller(*id)
                    .is_some_and(|controller| controller != player)
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
                self.current_controller(*id)
                    .is_some_and(|controller| controller != player)
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
                self.current_controller(*id)
                    .is_some_and(|controller| controllers.contains(&controller))
            })
            .collect()
    }

    pub(crate) fn current_controller(&self, object_id: ObjectId) -> Option<PlayerId> {
        let object = self.game.object(object_id)?;
        if !self.requires_battlefield_characteristic_calculation(object_id) {
            return Some(object.owner);
        }

        self.calculated_characteristics_arc(object_id)
            .map(|chars| chars.controller)
            .or(Some(object.owner))
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
        return target_id == card_id
            || grant
                .target_stable_id
                .is_some_and(|target_stable_id| target_stable_id == card.stable_id);
    }

    grant
        .filter
        .as_ref()
        .is_some_and(|filter| filter.matches(card, &grant_filter_context(ctx, grant, game), game))
}

fn grant_applies_to_card_non_recursive(
    grant: &Grant,
    card_id: ObjectId,
    card: &crate::object::Object,
    ctx: &crate::filter::FilterContext,
    game: &GameState,
) -> bool {
    if let Some(target_id) = grant.target_id {
        return target_id == card_id
            || grant
                .target_stable_id
                .is_some_and(|target_stable_id| target_stable_id == card.stable_id);
    }

    grant.filter.as_ref().is_some_and(|filter| {
        filter.matches_non_recursive(card, &grant_filter_context(ctx, grant, game), game)
    })
}

fn grant_filter_context(
    ctx: &crate::filter::FilterContext,
    grant: &Grant,
    game: &GameState,
) -> crate::filter::FilterContext {
    let mut ctx = ctx.clone();
    let source_id = grant.source.source_id();
    let source_exiled = game
        .get_exiled_with_source_links(source_id)
        .iter()
        .filter_map(|id| {
            game.object(*id)
                .map(|object| crate::snapshot::ObjectSnapshot::from_object(object, game))
        })
        .collect::<Vec<_>>();
    if !source_exiled.is_empty() {
        ctx.tagged_objects
            .insert(crate::tag::SOURCE_EXILED_TAG.into(), source_exiled);
    }
    ctx
}

fn materialize_derived_alternative_cast(
    card: &crate::object::Object,
    spec: &DerivedAlternativeCast,
) -> Option<crate::alternative_cast::AlternativeCastingMethod> {
    use crate::grant::DerivedAlternativeCastRuntimeExt as _;
    spec.materialize_for(card)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::continuous::{ContinuousEffect, Modification, TextBoxOverlay};
    use crate::effect::Effect;
    use crate::effect::Until;
    use crate::ids::{CardId, ObjectId, PlayerId};
    use crate::target::{ChooseSpec, ObjectFilter};
    use crate::types::{CardType, Subtype};
    use crate::zone::Zone;

    #[test]
    fn battlefield_characteristic_scope_uses_locked_targets_for_resolution_effects() {
        let game = crate::tests::test_helpers::setup_two_player_game();
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
            battlefield_characteristic_scope(&game, &effects),
            BattlefieldCharacteristicScope::Specific(vec![ObjectId::from_raw(2)]),
        );
    }

    #[test]
    fn battlefield_characteristic_scope_falls_back_to_all_battlefield_for_filter_effects() {
        let game = crate::tests::test_helpers::setup_two_player_game();
        let effects = vec![ContinuousEffect::new(
            ObjectId::from_raw(10),
            PlayerId::from_index(0),
            EffectTarget::AllCreatures,
            Modification::AddAbilityGeneric(Ability::static_ability(
                crate::static_abilities::StaticAbility::flying(),
            )),
        )];

        assert_eq!(
            battlefield_characteristic_scope(&game, &effects),
            BattlefieldCharacteristicScope::AllBattlefield,
        );
    }

    #[test]
    fn explicit_effect_view_honors_unregistered_payment_restriction_grants() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let creature = CardBuilder::new(CardId::from_raw(20_001), "Restriction Target")
            .card_types(vec![CardType::Creature])
            .build();
        let target = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        let effect = ContinuousEffect::new(
            target,
            alice,
            EffectTarget::Specific(target),
            Modification::AddAbility(
                crate::static_abilities::StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
            ),
        );
        let view = DerivedGameView::from_effects(&game, vec![effect]);

        assert!(view.player_cant_pay_life_to_cast_or_activate(alice));
        assert!(
            !game.player_cant_pay_life_to_cast_or_activate(alice),
            "an explicit view must not install its effects into the underlying game"
        );
    }

    #[test]
    fn explicit_effect_view_honors_payment_restriction_removal() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let creature = CardBuilder::new(CardId::from_raw(20_002), "Restriction Source")
            .card_types(vec![CardType::Creature])
            .build();
        let target = game.create_object_from_card(&creature, alice, Zone::Battlefield);
        game.object_mut(target)
            .expect("restriction source should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                crate::static_abilities::StaticAbility::cant_pay_life_or_sacrifice_nonland_for_cast_or_activate(),
            ));
        let effect = ContinuousEffect::new(
            target,
            alice,
            EffectTarget::Specific(target),
            Modification::RemoveAllAbilities,
        );
        let view = DerivedGameView::from_effects(&game, vec![effect]);

        assert!(!view.player_cant_pay_life_to_cast_or_activate(alice));
        assert!(game.player_cant_pay_life_to_cast_or_activate(alice));
    }

    #[test]
    fn cost_presence_set_abilities_removal_hides_printed_modifiers_and_minimum_mana() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(20_003), "Cost Modifier Source")
            .card_types(vec![CardType::Enchantment])
            .build();

        let spell_modifier_source =
            game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(spell_modifier_source)
            .expect("spell modifier source should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                crate::static_abilities::StaticAbility::new(
                    crate::static_abilities::CostReduction::new(
                        ObjectFilter::default(),
                        crate::effect::Value::Fixed(1),
                    ),
                ),
            ));

        let activation_modifier_source =
            game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(activation_modifier_source)
            .expect("activation modifier source should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                crate::static_abilities::StaticAbility::reduce_activated_ability_costs(
                    ObjectFilter::default(),
                    1,
                    None,
                ),
            ));

        let minimum_source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(minimum_source)
            .expect("minimum mana source should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                crate::static_abilities::StaticAbility::minimum_spell_total_mana(3),
            ));

        let effects = [
            spell_modifier_source,
            activation_modifier_source,
            minimum_source,
        ]
        .into_iter()
        .map(|source| {
            ContinuousEffect::new(
                source,
                alice,
                EffectTarget::Specific(source),
                Modification::SetAbilities(Vec::new()),
            )
        })
        .collect();
        let view = DerivedGameView::from_effects(&game, effects);

        assert!(!view.has_battlefield_spell_cost_modifiers());
        assert!(!view.has_activated_ability_cost_modifiers());
        assert_eq!(view.minimum_total_spell_mana_payment(), None);
    }

    #[test]
    fn cost_presence_ability_removal_modifications_force_layered_scans() {
        use crate::static_abilities::StaticAbility;

        let spell_modifier = StaticAbility::new(crate::static_abilities::CostReduction::new(
            ObjectFilter::default(),
            crate::effect::Value::Fixed(1),
        ));
        let activation_modifier =
            StaticAbility::reduce_activated_ability_costs(ObjectFilter::default(), 1, None);
        let minimum_mana = StaticAbility::minimum_spell_total_mana(3);

        for modification in [
            Modification::SetAbilities(Vec::new()),
            Modification::RemoveAllAbilities,
            Modification::RemoveAllAbilitiesExceptMana,
        ] {
            assert!(modification_can_change_spell_cost_modifier_presence(
                &modification
            ));
            assert!(
                modification_can_change_activated_ability_cost_modifier_presence(&modification)
            );
            assert!(modification_can_change_minimum_total_spell_mana_presence(
                &modification
            ));
        }

        assert!(modification_can_change_spell_cost_modifier_presence(
            &Modification::RemoveAbility(spell_modifier.clone())
        ));
        assert!(
            modification_can_change_activated_ability_cost_modifier_presence(
                &Modification::RemoveAbilityGeneric {
                    ability: Ability::static_ability(activation_modifier),
                    mode: ironsmith_core::AbilityLossMode::Lose,
                }
            )
        );
        assert!(modification_can_change_minimum_total_spell_mana_presence(
            &Modification::RemoveAbility(minimum_mana)
        ));
    }

    #[test]
    fn cost_presence_non_layered_scan_includes_level_and_temporary_grants() {
        use crate::ability::LevelAbility;
        use crate::object::CounterType;
        use crate::static_abilities::{StaticAbility, StaticAbilityId};

        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let source_card = CardBuilder::new(CardId::from_raw(20_004), "Granted Cost Source")
            .card_types(vec![CardType::Creature])
            .build();

        let level_source = game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        let spell_modifier = StaticAbility::new(crate::static_abilities::CostReduction::new(
            ObjectFilter::default(),
            crate::effect::Value::Fixed(1),
        ));
        let activation_modifier =
            StaticAbility::reduce_activated_ability_costs(ObjectFilter::default(), 1, None);
        let level_tier = LevelAbility::new(1, None)
            .with_ability(spell_modifier)
            .with_ability(activation_modifier)
            .with_ability(StaticAbility::minimum_spell_total_mana(3));
        game.object_mut(level_source)
            .expect("level source should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::with_level_abilities(vec![level_tier]),
            ));
        let _ = game.add_counters(level_source, CounterType::Level, 1);

        let duplicate_temporary_source =
            game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.object_mut(duplicate_temporary_source)
            .expect("temporary source should exist")
            .abilities_mut()
            .push(Ability::static_ability(
                StaticAbility::minimum_spell_total_mana(3),
            ));
        game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
            duplicate_temporary_source,
            StaticAbilityId::MinimumSpellTotalMana,
            Some(StaticAbility::minimum_spell_total_mana(5)),
        );

        let layered_duplicate_minimum = game
            .calculated_characteristics(duplicate_temporary_source)
            .expect("temporary source should have layered characteristics")
            .static_abilities
            .iter()
            .filter_map(StaticAbility::minimum_total_spell_mana)
            .max();
        assert_eq!(layered_duplicate_minimum, Some(3));

        let view = DerivedGameView::new(&game);
        assert!(view.has_battlefield_spell_cost_modifiers());
        assert!(view.has_activated_ability_cost_modifiers());
        assert_eq!(
            view.minimum_total_spell_mana_payment(),
            Some(3),
            "a temporary payload with the same ID as a printed ability must be suppressed"
        );

        let unique_temporary_source =
            game.create_object_from_card(&source_card, alice, Zone::Battlefield);
        game.grant_temporary_static_ability_payload_to_object_until_end_of_turn(
            unique_temporary_source,
            StaticAbilityId::MinimumSpellTotalMana,
            Some(StaticAbility::minimum_spell_total_mana(5)),
        );
        let temporary_view = DerivedGameView::new(&game);
        assert_eq!(temporary_view.minimum_total_spell_mana_payment(), Some(5));

        game.turn.turn_number += 1;
        let expired_view = DerivedGameView::new(&game);
        assert_eq!(
            expired_view.minimum_total_spell_mana_payment(),
            Some(3),
            "expired temporary grants must not remain in the sparse cost scan"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn battlefield_controlled_candidates_respect_continuous_control_changes() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let stolen_creature = game.create_object_from_definition(
            &crate::cards::definitions::grizzly_bears(),
            bob,
            Zone::Battlefield,
        );
        let _alice_creature = game.create_object_from_definition(
            &crate::cards::definitions::llanowar_elves(),
            alice,
            Zone::Battlefield,
        );

        let control_effect = ContinuousEffect::new(
            ObjectId::from_raw(9000),
            alice,
            EffectTarget::Specific(stolen_creature),
            Modification::ChangeController(alice),
        )
        .until(Until::EndOfTurn);
        let view = DerivedGameView::from_effects(&game, vec![control_effect]);
        let filter = ObjectFilter::creature()
            .you_control()
            .in_zone(Zone::Battlefield);
        let filter_ctx = game.filter_context_for(alice, None);

        let ids = view.candidate_ids_for_filter_with_context(&filter, &filter_ctx);
        assert!(
            ids.contains(&stolen_creature),
            "narrowed battlefield candidates should include continuously stolen creatures"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn battlefield_permanent_candidates_include_noncreatures() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature_id = game.create_object_from_definition(
            &crate::cards::definitions::grizzly_bears(),
            bob,
            Zone::Battlefield,
        );
        let enchantment = CardBuilder::new(CardId::from_raw(90_001), "Audit Enchantment")
            .card_types(vec![CardType::Enchantment])
            .build();
        let enchantment_id = game.create_object_from_card(&enchantment, bob, Zone::Battlefield);

        let view = DerivedGameView::new(&game);
        let filter = ObjectFilter::permanent();
        let filter_ctx = game.filter_context_for(alice, None);

        let ids = view.candidate_ids_for_filter_with_context(&filter, &filter_ctx);
        assert!(
            ids.contains(&creature_id) && ids.contains(&enchantment_id),
            "permanent candidate narrowing should keep both creature and enchantment permanents"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn battlefield_union_candidates_do_not_assume_creature_only() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);

        let creature_id = game.create_object_from_definition(
            &crate::cards::definitions::grizzly_bears(),
            bob,
            Zone::Battlefield,
        );
        let aura = CardBuilder::new(CardId::from_raw(90_002), "Audit Aura")
            .card_types(vec![CardType::Enchantment])
            .subtypes(vec![Subtype::Aura])
            .build();
        let aura_id = game.create_object_from_card(&aura, bob, Zone::Battlefield);
        let artifact = CardBuilder::new(CardId::from_raw(90_003), "Audit Relic")
            .card_types(vec![CardType::Artifact])
            .build();
        let artifact_id = game.create_object_from_card(&artifact, bob, Zone::Battlefield);

        let view = DerivedGameView::new(&game);
        let filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Creature],
            subtypes: vec![Subtype::Aura],
            type_or_subtype_union: true,
            ..Default::default()
        };
        let filter_ctx = game.filter_context_for(alice, None);

        let ids = view.candidate_ids_for_filter_with_context(&filter, &filter_ctx);
        assert!(
            ids.contains(&creature_id) && ids.contains(&aura_id),
            "creature-or-Aura union should include both creature and Aura permanents"
        );
        assert!(
            ids.contains(&artifact_id),
            "candidate enumeration may stay broad, but it must not drop valid Aura matches"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn simple_mana_analysis_respects_continuous_control_changes() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.turn.phase = crate::game_state::Phase::FirstMain;
        game.turn.step = None;

        let stolen_mountain = game.create_object_from_definition(
            &crate::cards::definitions::basic_mountain(),
            bob,
            Zone::Battlefield,
        );

        let control_effect = ContinuousEffect::new(
            ObjectId::from_raw(9001),
            alice,
            EffectTarget::Specific(stolen_mountain),
            Modification::ChangeController(alice),
        )
        .until(Until::EndOfTurn);
        let view = DerivedGameView::from_effects(&game, vec![control_effect]);
        let potential = crate::decision::compute_potential_mana_with_view(&game, alice, &view);

        assert_eq!(
            potential.red, 1,
            "stolen untapped Mountains should contribute to potential mana"
        );
    }

    #[cfg(ironsmith_runtime_parser_tests)]
    #[test]
    fn spell_target_legality_cache_key_includes_caster() {
        let mut game = crate::tests::test_helpers::setup_two_player_game();
        let alice = PlayerId::from_index(0);
        let bob = PlayerId::from_index(1);
        game.create_object_from_definition(
            &crate::cards::definitions::grizzly_bears(),
            alice,
            Zone::Battlefield,
        );

        let effects = vec![Effect::destroy(ChooseSpec::target(ChooseSpec::Object(
            ObjectFilter::creature()
                .you_control()
                .in_zone(Zone::Battlefield),
        )))];
        let view = DerivedGameView::new(&game);

        assert!(
            view.spell_has_legal_targets(&effects, alice, None, None),
            "Alice should have a legal 'you control' target"
        );
        assert!(
            !view.spell_has_legal_targets(&effects, bob, None, None),
            "Bob should not reuse Alice's cached targeting answer"
        );
    }
}
