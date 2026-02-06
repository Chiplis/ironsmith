//! Effect-generating static abilities.
//!
//! These abilities generate continuous effects that modify other objects
//! through the layer system.

use super::{StaticAbility, StaticAbilityId, StaticAbilityKind};
use crate::ability::Ability;
use crate::continuous::{
    ContinuousEffect, EffectSourceType, EffectTarget, Modification, PtSublayer,
};
use crate::effect::Value;
use crate::game_state::GameState;
use crate::ids::{ObjectId, PlayerId};
use crate::object::CounterType;
use crate::target::ObjectFilter;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

/// Anthem effect: "Creatures you control get +N/+M"
#[derive(Debug, Clone, PartialEq)]
pub struct Anthem {
    /// Filter for which permanents are affected.
    pub filter: ObjectFilter,
    /// Power modification.
    pub power: i32,
    /// Toughness modification.
    pub toughness: i32,
}

impl Anthem {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter,
            power,
            toughness,
        }
    }

    /// Create a standard anthem for creatures you control.
    pub fn creatures_you_control(power: i32, toughness: i32) -> Self {
        Self::new(ObjectFilter::creature().you_control(), power, toughness)
    }
}

impl StaticAbilityKind for Anthem {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::Anthem
    }

    fn display(&self) -> String {
        let sign_p = if self.power >= 0 { "+" } else { "" };
        let sign_t = if self.toughness >= 0 { "+" } else { "" };
        format!(
            "Affected creatures get {}{}/{}{}",
            sign_p, self.power, sign_t, self.toughness
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::ModifyPowerToughness {
                    power: self.power,
                    toughness: self.toughness,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn is_anthem(&self) -> bool {
        true
    }
}

/// Grant ability: "Creatures you control have [ability]"
#[derive(Debug, Clone)]
pub struct GrantAbility {
    /// Filter for which permanents gain the ability.
    pub filter: ObjectFilter,
    /// The ability to grant.
    pub ability: StaticAbility,
}

impl GrantAbility {
    pub fn new(filter: ObjectFilter, ability: StaticAbility) -> Self {
        Self { filter, ability }
    }
}

impl PartialEq for GrantAbility {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.ability == other.ability
    }
}

impl StaticAbilityKind for GrantAbility {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::GrantAbility
    }

    fn display(&self) -> String {
        format!("Affected permanents have {}", self.ability.display())
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn grants_abilities(&self) -> bool {
        true
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::AddAbility(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn apply_restrictions(&self, game: &mut GameState, _source: ObjectId, controller: PlayerId) {
        // Find permanents matching the filter
        let filter_ctx = game.filter_context_for(controller, None);
        let matching: Vec<ObjectId> = game
            .battlefield
            .iter()
            .filter(|&&id| {
                game.object(id)
                    .map(|obj| self.filter.matches(obj, &filter_ctx, game))
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        // Apply the granted ability's restrictions to each matching permanent
        for perm_id in matching {
            self.ability.apply_restrictions(game, perm_id, controller);
        }
    }
}

/// Remove ability: "Creatures lose [ability]"
#[derive(Debug, Clone)]
pub struct RemoveAbilityForFilter {
    /// Filter for which permanents lose the ability.
    pub filter: ObjectFilter,
    /// The ability to remove.
    pub ability: StaticAbility,
}

impl RemoveAbilityForFilter {
    pub fn new(filter: ObjectFilter, ability: StaticAbility) -> Self {
        Self { filter, ability }
    }
}

impl PartialEq for RemoveAbilityForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.ability == other.ability
    }
}

impl StaticAbilityKind for RemoveAbilityForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAbilityForFilter
    }

    fn display(&self) -> String {
        format!("Affected permanents lose {}", self.ability.display())
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveAbility(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Remove all abilities: "Creatures lose all abilities"
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveAllAbilitiesForFilter {
    /// Filter for which permanents lose all abilities.
    pub filter: ObjectFilter,
}

impl RemoveAllAbilitiesForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl StaticAbilityKind for RemoveAllAbilitiesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveAllAbilitiesForFilter
    }

    fn display(&self) -> String {
        "Affected permanents lose all abilities".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveAllAbilities,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Set base P/T: "... have base power and toughness N/M"
#[derive(Debug, Clone, PartialEq)]
pub struct SetBasePowerToughnessForFilter {
    /// Filter for which permanents get base P/T set.
    pub filter: ObjectFilter,
    /// Base power value.
    pub power: i32,
    /// Base toughness value.
    pub toughness: i32,
}

impl SetBasePowerToughnessForFilter {
    pub fn new(filter: ObjectFilter, power: i32, toughness: i32) -> Self {
        Self {
            filter,
            power,
            toughness,
        }
    }
}

impl StaticAbilityKind for SetBasePowerToughnessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetBasePowerToughnessForFilter
    }

    fn display(&self) -> String {
        format!(
            "Affected permanents have base power and toughness {}/{}",
            self.power, self.toughness
        )
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::SetPowerToughness {
                    power: Value::Fixed(self.power),
                    toughness: Value::Fixed(self.toughness),
                    sublayer: PtSublayer::Setting,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Condition for CopyActivatedAbilities.
#[derive(Debug, Clone, PartialEq)]
pub enum CopyActivatedAbilitiesCondition {
    /// "As long as you own a card exiled with a <counter> counter"
    OwnsCardExiledWithCounter(CounterType),
}

/// Copy activated abilities from objects matching a filter.
#[derive(Debug, Clone, PartialEq)]
pub struct CopyActivatedAbilities {
    pub filter: ObjectFilter,
    pub counter: Option<CounterType>,
    pub include_mana: bool,
    pub exclude_source_name: bool,
    pub exclude_source_id: bool,
    pub condition: Option<CopyActivatedAbilitiesCondition>,
    pub display: String,
}

impl CopyActivatedAbilities {
    pub fn new(filter: ObjectFilter) -> Self {
        Self {
            filter,
            counter: None,
            include_mana: true,
            exclude_source_name: false,
            exclude_source_id: true,
            condition: None,
            display: "Has all activated abilities of matching objects".to_string(),
        }
    }

    pub fn with_counter(mut self, counter: CounterType) -> Self {
        self.counter = Some(counter);
        self
    }

    pub fn with_exclude_source_name(mut self, exclude: bool) -> Self {
        self.exclude_source_name = exclude;
        self
    }

    pub fn with_exclude_source_id(mut self, exclude: bool) -> Self {
        self.exclude_source_id = exclude;
        self
    }

    pub fn with_condition(mut self, condition: CopyActivatedAbilitiesCondition) -> Self {
        self.condition = Some(condition);
        self
    }

    pub fn with_display(mut self, display: String) -> Self {
        self.display = display;
        self
    }
}

impl StaticAbilityKind for CopyActivatedAbilities {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::CopyActivatedAbilities
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Source,
                Modification::CopyActivatedAbilities {
                    filter: self.filter.clone(),
                    counter: self.counter,
                    include_mana: self.include_mana,
                    exclude_source_name: self.exclude_source_name,
                    exclude_source_id: self.exclude_source_id,
                },
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }

    fn is_active(&self, game: &GameState, source: ObjectId) -> bool {
        let Some(condition) = &self.condition else {
            return true;
        };

        let Some(source_obj) = game.object(source) else {
            return false;
        };
        let controller = source_obj.controller;

        match condition {
            CopyActivatedAbilitiesCondition::OwnsCardExiledWithCounter(counter) => {
                game.exile.iter().any(|&id| {
                    game.object(id).is_some_and(|obj| {
                        obj.owner == controller
                            && obj.counters.get(counter).copied().unwrap_or(0) > 0
                    })
                })
            }
        }
    }
}

/// Equipment grant: "Equipped creature has [abilities]"
#[derive(Debug, Clone)]
pub struct EquipmentGrant {
    /// The abilities to grant to the equipped creature.
    pub abilities: Vec<StaticAbility>,
}

/// Set colors: "All creatures are black."
#[derive(Debug, Clone)]
pub struct SetColorsForFilter {
    pub filter: ObjectFilter,
    pub colors: crate::color::ColorSet,
}

impl SetColorsForFilter {
    pub fn new(filter: ObjectFilter, colors: crate::color::ColorSet) -> Self {
        Self { filter, colors }
    }
}

impl PartialEq for SetColorsForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.colors == other.colors
    }
}

impl StaticAbilityKind for SetColorsForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::SetColors
    }

    fn display(&self) -> String {
        "Permanents have their colors set".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::SetColors(self.colors),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Add card types: "All permanents are artifacts in addition to their other types."
#[derive(Debug, Clone)]
pub struct AddCardTypesForFilter {
    pub filter: ObjectFilter,
    pub card_types: Vec<CardType>,
}

impl AddCardTypesForFilter {
    pub fn new(filter: ObjectFilter, card_types: Vec<CardType>) -> Self {
        Self { filter, card_types }
    }
}

impl PartialEq for AddCardTypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.card_types == other.card_types
    }
}

impl StaticAbilityKind for AddCardTypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AddCardTypes
    }

    fn display(&self) -> String {
        "Card types are added".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::AddCardTypes(self.card_types.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Make colorless: "All permanents are colorless."
#[derive(Debug, Clone)]
pub struct MakeColorlessForFilter {
    pub filter: ObjectFilter,
}

impl MakeColorlessForFilter {
    pub fn new(filter: ObjectFilter) -> Self {
        Self { filter }
    }
}

impl PartialEq for MakeColorlessForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter
    }
}

impl StaticAbilityKind for MakeColorlessForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::MakeColorless
    }

    fn display(&self) -> String {
        "Permanents are colorless".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::MakeColorless,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Remove supertypes: "All lands are no longer snow."
#[derive(Debug, Clone)]
pub struct RemoveSupertypesForFilter {
    pub filter: ObjectFilter,
    pub supertypes: Vec<Supertype>,
}

impl RemoveSupertypesForFilter {
    pub fn new(filter: ObjectFilter, supertypes: Vec<Supertype>) -> Self {
        Self { filter, supertypes }
    }
}

impl PartialEq for RemoveSupertypesForFilter {
    fn eq(&self, other: &Self) -> bool {
        self.filter == other.filter && self.supertypes == other.supertypes
    }
}

impl StaticAbilityKind for RemoveSupertypesForFilter {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::RemoveSupertypes
    }

    fn display(&self) -> String {
        "Supertypes are removed".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(self.filter.clone()),
                Modification::RemoveSupertypes(self.supertypes.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

impl EquipmentGrant {
    pub fn new(abilities: Vec<StaticAbility>) -> Self {
        Self { abilities }
    }
}

impl PartialEq for EquipmentGrant {
    fn eq(&self, other: &Self) -> bool {
        self.abilities == other.abilities
    }
}

impl StaticAbilityKind for EquipmentGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::EquipmentGrant
    }

    fn display(&self) -> String {
        let ability_names: Vec<String> = self.abilities.iter().map(|a| a.display()).collect();
        format!("Equipped creature has {}", ability_names.join(", "))
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn grants_abilities(&self) -> bool {
        true
    }

    fn equipment_grant_abilities(&self) -> Option<&[StaticAbility]> {
        Some(&self.abilities)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        self.abilities
            .iter()
            .map(|ability| {
                ContinuousEffect::new(
                    source,
                    controller,
                    EffectTarget::AttachedTo(source),
                    Modification::AddAbility(ability.clone()),
                )
                .with_source_type(EffectSourceType::StaticAbility)
            })
            .collect()
    }
}

/// Enchanted/attached permanent has an activated or triggered ability.
#[derive(Debug, Clone, PartialEq)]
pub struct AttachedAbilityGrant {
    pub ability: Ability,
    pub display: String,
}

impl AttachedAbilityGrant {
    pub fn new(ability: Ability, display: String) -> Self {
        Self { ability, display }
    }
}

impl StaticAbilityKind for AttachedAbilityGrant {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::AttachedAbilityGrant
    }

    fn display(&self) -> String {
        self.display.clone()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(self.clone())
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::AttachedTo(source),
                Modification::AddAbilityGeneric(self.ability.clone()),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Blood Moon: "Nonbasic lands are Mountains"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BloodMoon;

impl StaticAbilityKind for BloodMoon {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::BloodMoon
    }

    fn display(&self) -> String {
        "Nonbasic lands are Mountains".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let nonbasic_land_filter = ObjectFilter {
            zone: Some(Zone::Battlefield),
            card_types: vec![CardType::Land],
            excluded_supertypes: vec![Supertype::Basic],
            ..Default::default()
        };

        vec![
            // Layer 4: Set land subtypes to Mountain
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(nonbasic_land_filter.clone()),
                Modification::SetSubtypes(vec![Subtype::Mountain]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
            // Layer 6: Remove all abilities
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(nonbasic_land_filter),
                Modification::RemoveAllAbilities,
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

/// Toph, the First Metalbender: "Nontoken artifacts you control are lands in addition to their other types."
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TophFirstMetalbender;

impl StaticAbilityKind for TophFirstMetalbender {
    fn id(&self) -> StaticAbilityId {
        StaticAbilityId::TophFirstMetalbender
    }

    fn display(&self) -> String {
        "Nontoken artifacts you control are lands in addition to their other types.".to_string()
    }

    fn clone_box(&self) -> Box<dyn StaticAbilityKind> {
        Box::new(*self)
    }

    fn generate_effects(
        &self,
        source: ObjectId,
        controller: PlayerId,
        _game: &GameState,
    ) -> Vec<ContinuousEffect> {
        let filter = ObjectFilter::artifact().you_control().nontoken();
        vec![
            ContinuousEffect::new(
                source,
                controller,
                EffectTarget::Filter(filter),
                Modification::AddCardTypes(vec![CardType::Land]),
            )
            .with_source_type(EffectSourceType::StaticAbility),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthem() {
        let anthem = Anthem::creatures_you_control(1, 1);
        assert_eq!(anthem.id(), StaticAbilityId::Anthem);
        assert!(anthem.is_anthem());
        assert_eq!(anthem.display(), "Affected creatures get +1/+1");
    }

    #[test]
    fn test_anthem_generates_effects() {
        let anthem = Anthem::creatures_you_control(2, 2);
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);

        let effects = anthem.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::ModifyPowerToughness {
                power: 2,
                toughness: 2
            }
        ));
    }

    #[test]
    fn test_blood_moon() {
        let blood_moon = BloodMoon;
        assert_eq!(blood_moon.id(), StaticAbilityId::BloodMoon);
        assert_eq!(blood_moon.display(), "Nonbasic lands are Mountains");
    }

    #[test]
    fn test_blood_moon_generates_two_effects() {
        let blood_moon = BloodMoon;
        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let source = ObjectId::from_raw(1);
        let controller = PlayerId::from_index(0);

        let effects = blood_moon.generate_effects(source, controller, &game);
        assert_eq!(effects.len(), 2);
    }

    #[test]
    fn test_grant_ability() {
        let grant = GrantAbility::new(
            ObjectFilter::creature().you_control(),
            StaticAbility::flying(),
        );
        assert_eq!(grant.id(), StaticAbilityId::GrantAbility);
        assert!(grant.grants_abilities());
    }

    #[test]
    fn test_equipment_grant() {
        let grant = EquipmentGrant::new(vec![StaticAbility::haste(), StaticAbility::shroud()]);
        assert_eq!(grant.id(), StaticAbilityId::EquipmentGrant);
        assert!(grant.grants_abilities());
        assert!(grant.display().contains("Haste"));
        assert!(grant.display().contains("Shroud"));
    }

    #[test]
    fn test_remove_all_abilities_for_filter() {
        let ability = RemoveAllAbilitiesForFilter::new(ObjectFilter::creature());
        assert_eq!(ability.id(), StaticAbilityId::RemoveAllAbilitiesForFilter);

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let effects =
            ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::RemoveAllAbilities
        ));
    }

    #[test]
    fn test_set_base_power_toughness_for_filter() {
        let ability = SetBasePowerToughnessForFilter::new(ObjectFilter::creature(), 1, 1);
        assert_eq!(
            ability.id(),
            StaticAbilityId::SetBasePowerToughnessForFilter
        );

        let game = GameState::new(vec!["Alice".to_string(), "Bob".to_string()], 20);
        let effects =
            ability.generate_effects(ObjectId::from_raw(1), PlayerId::from_index(0), &game);
        assert_eq!(effects.len(), 1);
        assert!(matches!(
            effects[0].modification,
            Modification::SetPowerToughness {
                power: Value::Fixed(1),
                toughness: Value::Fixed(1),
                sublayer: PtSublayer::Setting,
            }
        ));
    }
}
