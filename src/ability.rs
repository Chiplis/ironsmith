//! Ability system for permanents and spells.
//!
//! MTG has four types of abilities:
//! - Static: Always active while the source is in the appropriate zone
//! - Triggered: Go on the stack when a condition is met
//! - Activated: Can be activated by paying a cost
//! - Mana: Special activated abilities that produce mana (don't use the stack)

use crate::color::ColorSet;
use crate::cost::TotalCost;
use crate::effect::Effect;
use crate::filter::{AlternativeCastKind, Comparison};
use crate::ids::{ObjectId, PlayerId};
use crate::mana::ManaSymbol;
use crate::static_abilities::StaticAbility as NewStaticAbility;
use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
use crate::triggers::Trigger;
use crate::types::CardType;
use crate::zone::Zone;

/// Merge effect-backed costs into a `TotalCost`.
pub fn merge_cost_effects(cost: TotalCost, effects: Vec<Effect>) -> TotalCost {
    let mut merged = cost.costs().to_vec();
    merged.extend(effects.into_iter().map(crate::costs::Cost::effect));
    TotalCost::from_costs(merged)
}

/// A complete ability definition.
#[derive(Debug, Clone, PartialEq)]
pub struct Ability {
    /// The kind of ability (static, triggered, activated, or mana)
    pub kind: AbilityKind,

    /// Zones where this ability functions (default: depends on ability type)
    pub functional_zones: Vec<Zone>,

    /// Optional text description (for display purposes)
    pub text: Option<String>,
}

impl Ability {
    /// Create a static ability.
    pub fn static_ability(effect: NewStaticAbility) -> Self {
        Self {
            kind: AbilityKind::Static(effect),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        }
    }

    /// Create a triggered ability.
    pub fn triggered(trigger: Trigger, effects: Vec<Effect>) -> Self {
        Self {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects,
                choices: vec![],
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        }
    }

    /// Create an optional triggered ability ("you may" effects).
    pub fn triggered_optional(trigger: Trigger, effects: Vec<Effect>) -> Self {
        Self {
            kind: AbilityKind::Triggered(TriggeredAbility {
                trigger,
                effects,
                choices: vec![],
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        }
    }

    /// Create an activated ability.
    pub fn activated(mana_cost: TotalCost, effects: Vec<Effect>) -> Self {
        Self::activated_with_timing(mana_cost, effects, ActivationTiming::AnyTime)
    }

    /// Create an activated ability with an explicit timing restriction.
    pub fn activated_with_timing(
        mana_cost: TotalCost,
        effects: Vec<Effect>,
        timing: ActivationTiming,
    ) -> Self {
        Self {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost,
                effects,
                choices: vec![],
                timing,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        }
    }

    /// Create an activated ability with cost effects.
    pub fn activated_with_cost_effects(
        cost: TotalCost,
        cost_effects: Vec<Effect>,
        effects: Vec<Effect>,
    ) -> Self {
        Self {
            kind: AbilityKind::Activated(ActivatedAbility {
                mana_cost: merge_cost_effects(cost, cost_effects),
                effects,
                choices: vec![],
                timing: ActivationTiming::AnyTime,
                additional_restrictions: vec![],
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        }
    }

    /// Create a mana ability that taps for mana.
    /// Cost includes tap as a cost component.
    pub fn mana(cost: TotalCost, mana: Vec<ManaSymbol>) -> Self {
        let mut costs = cost.costs().to_vec();
        if !costs.iter().any(|c| c.requires_tap()) {
            costs.push(crate::costs::Cost::effect(Effect::tap_source()));
        }
        Self {
            kind: AbilityKind::Mana(ManaAbility {
                mana_cost: TotalCost::from_costs(costs),
                mana,
                effects: None,
                activation_condition: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        }
    }

    /// Create a mana ability with variable output (uses effects instead of fixed mana).
    pub fn mana_with_effects(cost: TotalCost, effects: Vec<Effect>) -> Self {
        Self {
            kind: AbilityKind::Mana(ManaAbility::with_effects(cost, effects)),
            functional_zones: vec![Zone::Battlefield],
            text: None,
        }
    }

    /// Set the zones where this ability functions.
    pub fn in_zones(mut self, zones: Vec<Zone>) -> Self {
        self.functional_zones = zones;
        self
    }

    /// Set the ability text.
    pub fn with_text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    /// Check if this ability is a mana ability.
    pub fn is_mana_ability(&self) -> bool {
        matches!(self.kind, AbilityKind::Mana(_))
    }

    /// Check if this ability functions in the given zone.
    pub fn functions_in(&self, zone: &Zone) -> bool {
        self.functional_zones.contains(zone)
    }
}

/// The kind of ability.
#[derive(Debug, Clone, PartialEq)]
pub enum AbilityKind {
    /// Static ability (always active)
    Static(NewStaticAbility),

    /// Triggered ability (triggers on events)
    Triggered(TriggeredAbility),

    /// Activated ability (pay cost to activate)
    Activated(ActivatedAbility),

    /// Mana ability (special activated ability that produces mana)
    Mana(ManaAbility),
}

/// Protection from something.
#[derive(Debug, Clone, PartialEq)]
pub enum ProtectionFrom {
    /// Protection from a color
    Color(crate::color::ColorSet),

    /// Protection from colorless (Giver of Runes, etc.)
    Colorless,

    /// Protection from all colors (progenitus)
    AllColors,

    /// Protection from creatures
    Creatures,

    /// Protection from a card type
    CardType(CardType),

    /// Protection from permanents matching a filter
    Permanents(ObjectFilter),

    /// Protection from everything
    Everything,
}

/// A level ability tier - applies when level counter count is in range.
///
/// Level-up creatures have multiple tiers of abilities that activate based
/// on the number of level counters they have.
#[derive(Debug, Clone, PartialEq)]
pub struct LevelAbility {
    /// Minimum level for this tier (inclusive, 0 = base level with no counters)
    pub min_level: u32,
    /// Maximum level for this tier (None = no upper bound)
    pub max_level: Option<u32>,
    /// Power/toughness at this level (None = use base P/T)
    pub power_toughness: Option<(i32, i32)>,
    /// Static abilities granted at this level (uses new trait-based type)
    pub abilities: Vec<NewStaticAbility>,
}

impl LevelAbility {
    /// Create a new level ability tier.
    pub fn new(min_level: u32, max_level: Option<u32>) -> Self {
        Self {
            min_level,
            max_level,
            power_toughness: None,
            abilities: Vec::new(),
        }
    }

    /// Set the power/toughness for this level tier.
    pub fn with_pt(mut self, power: i32, toughness: i32) -> Self {
        self.power_toughness = Some((power, toughness));
        self
    }

    /// Add a static ability to this level tier.
    pub fn with_ability(mut self, ability: NewStaticAbility) -> Self {
        self.abilities.push(ability);
        self
    }

    /// Add multiple static abilities to this level tier.
    pub fn with_abilities(mut self, abilities: Vec<NewStaticAbility>) -> Self {
        self.abilities.extend(abilities);
        self
    }

    /// Check if this level tier applies for the given level count.
    pub fn applies_at_level(&self, level_count: u32) -> bool {
        level_count >= self.min_level && self.max_level.is_none_or(|max| level_count <= max)
    }
}

/// Filter for spells (for cost modification).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpellFilter {
    /// Card types the spell must have
    pub card_types: Vec<CardType>,

    /// Card types the spell must not have
    pub excluded_card_types: Vec<CardType>,

    /// Subtypes the spell must have
    pub subtypes: Vec<crate::types::Subtype>,

    /// Colors the spell must have
    pub colors: Option<ColorSet>,

    /// Power comparison for creature spells (if applicable).
    pub power: Option<Comparison>,

    /// Toughness comparison for creature spells (if applicable).
    pub toughness: Option<Comparison>,

    /// Mana value comparison.
    pub mana_value: Option<Comparison>,

    /// If set, only match spells that target a player matching this filter.
    pub targets_player: Option<PlayerFilter>,

    /// If set, only match spells that target an object matching this filter.
    pub targets_object: Option<ObjectFilter>,

    /// Controller filter
    pub controller: Option<PlayerFilter>,

    /// Restrict to spells cast with a specific alternative casting method.
    pub alternative_cast: Option<AlternativeCastKind>,
}

impl SpellFilter {
    pub fn description(&self) -> String {
        let mut filter = ObjectFilter::default();
        filter.zone = Some(Zone::Stack);
        filter.card_types = self.card_types.clone();
        filter.excluded_card_types = self.excluded_card_types.clone();
        filter.subtypes = self.subtypes.clone();
        filter.colors = self.colors;
        filter.power = self.power.clone();
        filter.toughness = self.toughness.clone();
        filter.mana_value = self.mana_value.clone();
        filter.controller = self.controller.clone();
        filter.alternative_cast = self.alternative_cast;
        filter.targets_player = self.targets_player.clone();
        filter.targets_object = self.targets_object.clone().map(Box::new);
        filter.description()
    }
}

// === Triggered Abilities ===

/// A triggered ability that fires when a condition is met.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggeredAbility {
    /// What triggers this ability (trait-based matcher)
    pub trigger: Trigger,

    /// Effects that occur when triggered
    pub effects: Vec<Effect>,

    /// Chosen entities required when the ability goes on the stack/resolves/is paid for
    pub choices: Vec<ChooseSpec>,

    /// Intervening-if condition (if any).
    ///
    /// Per MTG rules, an "intervening if" clause is a condition that:
    /// 1. Must be true when the trigger condition is met, OR the ability doesn't trigger
    /// 2. Must be true when the ability would resolve, OR the ability does nothing
    ///
    /// Example: "When this creature dies, if it was enchanted, draw a card"
    /// - If not enchanted when it dies, doesn't trigger at all
    /// - If enchanted when it dies but not when resolving (somehow), does nothing
    pub intervening_if: Option<InterveningIfCondition>,
}

impl TriggeredAbility {
    /// Add targets to this trigger.
    pub fn with_targets(mut self, targets: Vec<ChooseSpec>) -> Self {
        self.choices = targets;
        self
    }

    /// Add an intervening-if condition.
    pub fn with_intervening_if(mut self, condition: InterveningIfCondition) -> Self {
        self.intervening_if = Some(condition);
        self
    }
}

/// Condition that must be true both when a trigger fires and when it resolves.
///
/// These are "intervening if" clauses in MTG rules terminology.
/// If the condition is false when the trigger would fire, the ability doesn't trigger.
/// If the condition is false when the ability would resolve, it does nothing.
#[derive(Debug, Clone, PartialEq)]
pub enum InterveningIfCondition {
    /// "if you control [filter]" - controller must control matching permanent(s)
    YouControl(ObjectFilter),

    /// "if an opponent controls [filter]"
    OpponentControls(ObjectFilter),

    /// "if your life total is at least N"
    LifeTotalAtLeast(i32),

    /// "if your life total is at most N"
    LifeTotalAtMost(i32),

    /// "if no creature died this turn"
    NoCreaturesDiedThisTurn,

    /// "if a creature died this turn"
    CreatureDiedThisTurn,

    /// "if this is the first time this ability triggered this turn"
    FirstTimeThisTurn,

    /// "if this ability has triggered at most N times this turn"
    MaxTimesEachTurn(u32),

    /// "if this creature was enchanted" (uses snapshot from LTB/death trigger)
    WasEnchanted,

    /// "if this creature had N or more counters" (uses snapshot)
    HadCounters(crate::object::CounterType, u32),
}

// === Activated Abilities ===

/// An activated ability that can be activated by paying a cost.
#[derive(Debug, Clone, PartialEq)]
pub struct ActivatedAbility {
    /// Cost to activate
    pub mana_cost: TotalCost,

    /// Effects that occur when the ability resolves
    pub effects: Vec<Effect>,

    /// Targets required when activating
    pub choices: Vec<ChooseSpec>,

    /// Timing restriction
    pub timing: ActivationTiming,

    /// Additional textual activation restrictions not modeled by `timing`.
    pub additional_restrictions: Vec<String>,
}

impl ActivatedAbility {
    /// Add targets to this ability.
    pub fn with_targets(mut self, targets: Vec<ChooseSpec>) -> Self {
        self.choices = targets;
        self
    }

    /// Add effect-backed cost components.
    pub fn with_cost_effects(mut self, effects: Vec<Effect>) -> Self {
        self.mana_cost = merge_cost_effects(self.mana_cost, effects);
        self
    }

    /// Restrict to sorcery speed.
    pub fn sorcery_speed(mut self) -> Self {
        self.timing = ActivationTiming::SorcerySpeed;
        self
    }

    /// Restrict to once per turn.
    pub fn once_per_turn(mut self) -> Self {
        self.timing = ActivationTiming::OncePerTurn;
        self
    }

    /// Returns true if this activated ability requires tapping the source.
    ///
    /// This checks if any cost component taps the source.
    pub fn has_tap_cost(&self) -> bool {
        self.mana_cost.costs().iter().any(|c| c.requires_tap())
    }

    /// Returns true if this activated ability requires sacrificing the source.
    ///
    /// This checks if any cost component sacrifices the source.
    pub fn has_sacrifice_self_cost(&self) -> bool {
        self.mana_cost.costs().iter().any(|c| c.is_sacrifice_self())
    }

    /// Returns the life cost amount if this ability requires paying life.
    ///
    /// This checks cost components for a life payment effect and returns the amount.
    pub fn life_cost_amount(&self) -> Option<u32> {
        self.mana_cost.costs().iter().find_map(|c| c.life_amount())
    }

    /// Returns a per-turn activation cap from `timing` and textual restrictions,
    /// if one is present.
    ///
    /// "Activate only once each turn" maps to `Some(1)` via `timing`.
    /// "Activate only twice each turn" maps to `Some(2)` via restriction text.
    pub fn max_activations_per_turn(&self) -> Option<u32> {
        if self.timing == ActivationTiming::OncePerTurn {
            return Some(1);
        }

        self.additional_restrictions
            .iter()
            .find_map(|restriction| parse_activation_max_times_per_turn(restriction))
    }
}

fn parse_activation_max_times_per_turn(restriction: &str) -> Option<u32> {
    let normalized = restriction
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c.is_ascii_whitespace() {
                c
            } else {
                ' '
            }
        })
        .collect::<String>();

    let words: Vec<&str> = normalized.split_whitespace().collect();
    if words.len() < 4 || !words.starts_with(&["activate", "only"]) {
        return None;
    }

    let each_turn_pos = words
        .windows(2)
        .position(|window| window[0] == "each" && window[1] == "turn")?;

    let count_pos = each_turn_pos.checked_sub(2)?;
    let mut count_word = words[count_pos];
    if count_word == "time" || count_word == "times" {
        let count_pos = each_turn_pos.checked_sub(3)?;
        count_word = words[count_pos];
    }

    parse_named_count_word(count_word)
}

fn parse_named_count_word(word: &str) -> Option<u32> {
    if let Ok(value) = word.parse::<u32>() {
        return Some(value);
    }

    match word {
        "once" => Some(1),
        "twice" => Some(2),
        "three" => Some(3),
        "four" => Some(4),
        "five" => Some(5),
        "six" => Some(6),
        "seven" => Some(7),
        "eight" => Some(8),
        "nine" => Some(9),
        "ten" => Some(10),
        _ => None,
    }
}

/// When an activated ability can be activated.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ActivationTiming {
    /// Can be activated any time you have priority
    #[default]
    AnyTime,

    /// Only when you could cast a sorcery
    SorcerySpeed,

    /// Only during combat
    DuringCombat,

    /// Only once per turn
    OncePerTurn,

    /// Only during your turn
    DuringYourTurn,

    /// Only during an opponent's turn
    DuringOpponentsTurn,
}

// === Mana Abilities ===

/// A mana ability (doesn't use the stack).
#[derive(Debug, Clone, PartialEq)]
pub struct ManaAbility {
    /// Cost to activate (usually just tap)
    pub mana_cost: TotalCost,

    /// Fixed mana produced (use this for simple abilities like basic lands)
    pub mana: Vec<ManaSymbol>,

    /// Effects to execute for variable mana (e.g., add mana equal to counters)
    /// If present, these are used instead of `mana`.
    pub effects: Option<Vec<Effect>>,

    /// Condition that must be true to activate this ability.
    /// Used for lands like Bleachbone Verge with "Activate only if you control a Plains or a Swamp."
    pub activation_condition: Option<ManaAbilityCondition>,
}

/// Condition for activating a mana ability.
///
/// Used for abilities like "Activate only if you control a Plains or a Swamp."
#[derive(Debug, Clone, PartialEq)]
pub enum ManaAbilityCondition {
    /// Controller must control a land with at least one of these subtypes.
    /// Used for verge lands (e.g., "Activate only if you control a Plains or a Swamp").
    ControlLandWithSubtype(Vec<crate::types::Subtype>),

    /// Controller must control at least N artifacts.
    /// Used for metalcraft-style mana abilities (e.g., Mox Opal).
    ControlAtLeastArtifacts(u32),

    /// Controller must control at least N lands.
    /// Used for conditions like "Activate only if you control five or more lands."
    ControlAtLeastLands(u32),

    /// Controller must control a creature with power at least N.
    /// Used for ferocious-style mana activation checks.
    ControlCreatureWithPowerAtLeast(u32),

    /// Controller's creatures must have total power at least N.
    /// Used for formidable-style mana activation checks.
    ControlCreaturesTotalPowerAtLeast(u32),

    /// Controller must have a matching card in their graveyard.
    /// Used for clauses like "Activate only if there is an Elf card in your graveyard."
    CardInYourGraveyard {
        card_types: Vec<crate::types::CardType>,
        subtypes: Vec<crate::types::Subtype>,
    },

    /// Activation timing restriction for mana abilities.
    Timing(ActivationTiming),

    /// Maximum number of activations allowed in one turn.
    /// Used for clauses like "Activate only twice each turn".
    MaxActivationsPerTurn(u32),

    /// Conjunction of multiple activation restrictions.
    All(Vec<ManaAbilityCondition>),

    /// A supported-but-unmodeled restriction condition.
    ///
    /// This is used when a known activation restriction pattern is not
    /// modeled in detail yet but should still be represented in compiled text.
    Unmodeled(String),
}

impl ManaAbility {
    /// Create a basic land mana ability ({T}: Add [mana]).
    pub fn basic(mana: ManaSymbol) -> Self {
        Self {
            mana_cost: TotalCost::from_cost(crate::costs::Cost::effect(Effect::tap_source())),
            mana: vec![mana],
            effects: None,
            activation_condition: None,
        }
    }

    /// Create a mana ability with variable output (uses effects).
    /// Includes tap as a cost effect by default.
    pub fn with_effects(cost: TotalCost, effects: Vec<Effect>) -> Self {
        let mut costs = cost.costs().to_vec();
        costs.push(crate::costs::Cost::effect(Effect::tap_source()));
        Self {
            mana_cost: TotalCost::from_costs(costs),
            mana: Vec::new(),
            effects: Some(effects),
            activation_condition: None,
        }
    }

    /// Create a mana ability with cost effects (e.g., sacrifice a creature: Add {C}{C}).
    pub fn with_cost_effects(
        cost: TotalCost,
        cost_effects: Vec<Effect>,
        mana: Vec<ManaSymbol>,
    ) -> Self {
        Self {
            mana_cost: merge_cost_effects(cost, cost_effects),
            mana,
            effects: None,
            activation_condition: None,
        }
    }

    /// Create a conditional mana ability that requires controlling a land with certain subtypes.
    pub fn conditional(mana: ManaSymbol, required_subtypes: Vec<crate::types::Subtype>) -> Self {
        Self {
            mana_cost: TotalCost::from_cost(crate::costs::Cost::effect(Effect::tap_source())),
            mana: vec![mana],
            effects: None,
            activation_condition: Some(ManaAbilityCondition::ControlLandWithSubtype(
                required_subtypes,
            )),
        }
    }

    /// Add an activation condition to this mana ability.
    pub fn with_condition(mut self, condition: ManaAbilityCondition) -> Self {
        self.activation_condition = Some(condition);
        self
    }

    /// Add cost effects to this mana ability (e.g., sacrifice).
    pub fn with_cost_effects_builder(mut self, cost_effects: Vec<Effect>) -> Self {
        self.mana_cost = merge_cost_effects(self.mana_cost, cost_effects);
        self
    }

    /// Returns true if this mana ability requires tapping the source.
    ///
    /// This checks if any cost component taps the source.
    pub fn has_tap_cost(&self) -> bool {
        self.mana_cost.costs().iter().any(|c| c.requires_tap())
    }

    /// Returns the life cost amount if this ability requires paying life.
    ///
    /// This checks cost components for a pay life effect and returns the amount.
    pub fn life_cost_amount(&self) -> Option<u32> {
        self.mana_cost.costs().iter().find_map(|c| c.life_amount())
    }

    /// Returns true if this mana ability requires sacrificing the source.
    ///
    /// This checks if any cost component sacrifices the source.
    pub fn has_sacrifice_self_cost(&self) -> bool {
        self.mana_cost.costs().iter().any(|c| c.is_sacrifice_self())
    }
}

// === Ability on Stack ===

/// An ability instance on the stack (triggered or activated).
#[derive(Debug, Clone)]
pub struct AbilityOnStack {
    /// The source object this ability came from
    pub source: ObjectId,

    /// The controller of this ability
    pub controller: PlayerId,

    /// The kind of ability
    pub kind: StackedAbilityKind,

    /// Resolved targets
    pub targets: Vec<crate::game_state::Target>,

    /// X value if applicable
    pub x_value: Option<u32>,

    /// The effects to execute
    pub effects: Vec<Effect>,
}

/// The kind of ability on the stack.
#[derive(Debug, Clone, PartialEq)]
pub enum StackedAbilityKind {
    Triggered,
    Activated,
}

// === Builder functions for common abilities ===

/// Create a flying ability.
pub fn flying() -> Ability {
    Ability::static_ability(NewStaticAbility::flying()).with_text("Flying")
}

/// Create a first strike ability.
pub fn first_strike() -> Ability {
    Ability::static_ability(NewStaticAbility::first_strike()).with_text("First strike")
}

/// Create a double strike ability.
pub fn double_strike() -> Ability {
    Ability::static_ability(NewStaticAbility::double_strike()).with_text("Double strike")
}

/// Create a deathtouch ability.
pub fn deathtouch() -> Ability {
    Ability::static_ability(NewStaticAbility::deathtouch()).with_text("Deathtouch")
}

/// Create a lifelink ability.
pub fn lifelink() -> Ability {
    Ability::static_ability(NewStaticAbility::lifelink()).with_text("Lifelink")
}

/// Create a vigilance ability.
pub fn vigilance() -> Ability {
    Ability::static_ability(NewStaticAbility::vigilance()).with_text("Vigilance")
}

/// Create a trample ability.
pub fn trample() -> Ability {
    Ability::static_ability(NewStaticAbility::trample()).with_text("Trample")
}

/// Create a haste ability.
pub fn haste() -> Ability {
    Ability::static_ability(NewStaticAbility::haste()).with_text("Haste")
}

/// Create a reach ability.
pub fn reach() -> Ability {
    Ability::static_ability(NewStaticAbility::reach()).with_text("Reach")
}

/// Create a defender ability.
pub fn defender() -> Ability {
    Ability::static_ability(NewStaticAbility::defender()).with_text("Defender")
}

/// Create a hexproof ability.
pub fn hexproof() -> Ability {
    Ability::static_ability(NewStaticAbility::hexproof()).with_text("Hexproof")
}

/// Create an indestructible ability.
pub fn indestructible() -> Ability {
    Ability::static_ability(NewStaticAbility::indestructible()).with_text("Indestructible")
}

/// Create a menace ability.
pub fn menace() -> Ability {
    Ability::static_ability(NewStaticAbility::menace()).with_text("Menace")
}

/// Create a flash ability.
pub fn flash() -> Ability {
    Ability::static_ability(NewStaticAbility::flash()).with_text("Flash")
}

/// Create an "enters the battlefield" triggered ability.
pub fn etb_trigger(effects: Vec<Effect>) -> Ability {
    Ability::triggered(Trigger::this_enters_battlefield(), effects)
}

/// Create a "when this dies" triggered ability.
pub fn dies_trigger(effects: Vec<Effect>) -> Ability {
    Ability::triggered(Trigger::this_dies(), effects)
}

/// Create an "at the beginning of your upkeep" triggered ability.
pub fn upkeep_trigger(effects: Vec<Effect>) -> Ability {
    Ability::triggered(Trigger::beginning_of_upkeep(PlayerFilter::You), effects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effect::Effect;
    use crate::static_abilities::StaticAbilityId;

    #[test]
    fn test_static_ability() {
        let ability = flying();
        if let AbilityKind::Static(s) = &ability.kind {
            assert_eq!(s.id(), StaticAbilityId::Flying);
        } else {
            panic!("Expected static ability");
        }
        assert_eq!(ability.text, Some("Flying".to_string()));
    }

    #[test]
    fn test_mana_ability() {
        let tap_for_green = Ability::mana(TotalCost::free(), vec![ManaSymbol::Green]);
        assert!(tap_for_green.is_mana_ability());
    }

    #[test]
    fn test_activated_ability() {
        // {2}{B}, {T}: Draw a card
        let mana_cost = crate::mana::ManaCost::from_pips(vec![
            vec![ManaSymbol::Generic(2)],
            vec![ManaSymbol::Black],
        ]);
        let cost = TotalCost::mana(mana_cost);
        let cost_effects = vec![Effect::tap_source()];
        let ability =
            Ability::activated_with_cost_effects(cost, cost_effects, vec![Effect::draw(1)]);

        assert!(matches!(ability.kind, AbilityKind::Activated(_)));
        assert!(!ability.is_mana_ability());
    }

    #[test]
    fn test_triggered_ability() {
        // When this creature enters the battlefield, draw a card
        let ability = etb_trigger(vec![Effect::draw(1)]);

        if let AbilityKind::Triggered(triggered) = &ability.kind {
            assert!(
                triggered
                    .trigger
                    .display()
                    .contains("enters the battlefield")
            );
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_anthem_ability() {
        // Other creatures you control get +1/+1
        use crate::target::ObjectFilter;
        let anthem = crate::static_abilities::StaticAbility::anthem(
            ObjectFilter::creature().you_control().other(),
            1,
            1,
        );
        let ability = Ability::static_ability(anthem);

        if let AbilityKind::Static(s) = &ability.kind {
            assert_eq!(s.id(), StaticAbilityId::Anthem);
        } else {
            panic!("Expected anthem static ability");
        }
    }

    #[test]
    fn test_ward_ability() {
        // Ward {2}
        let mana_cost = crate::mana::ManaCost::from_pips(vec![vec![ManaSymbol::Generic(2)]]);
        let ward = crate::static_abilities::StaticAbility::ward(TotalCost::mana(mana_cost));
        let ability = Ability::static_ability(ward);

        if let AbilityKind::Static(s) = &ability.kind {
            assert_eq!(s.id(), StaticAbilityId::Ward);
        } else {
            panic!("Expected ward static ability");
        }
    }

    #[test]
    fn test_protection() {
        use crate::color::{Color, ColorSet};

        // Protection from red
        let prot_red = crate::static_abilities::StaticAbility::protection(ProtectionFrom::Color(
            ColorSet::from(Color::Red),
        ));
        let ability = Ability::static_ability(prot_red);

        if let AbilityKind::Static(s) = &ability.kind {
            assert_eq!(s.id(), StaticAbilityId::Protection);
            if let Some(ProtectionFrom::Color(colors)) = s.protection_from() {
                assert!(colors.contains(Color::Red));
            } else {
                panic!("Expected protection from color");
            }
        } else {
            panic!("Expected protection ability");
        }
    }

    #[test]
    fn test_dies_trigger() {
        // When this creature dies, each opponent loses 2 life
        let ability = dies_trigger(vec![Effect::for_each_opponent(vec![Effect::lose_life(2)])]);

        if let AbilityKind::Triggered(triggered) = &ability.kind {
            assert!(triggered.trigger.display().contains("dies"));
        } else {
            panic!("Expected triggered ability");
        }
    }

    #[test]
    fn test_blood_artist_ability() {
        use crate::target::{ChooseSpec, ObjectFilter, PlayerFilter};
        use crate::triggers::Trigger;
        use crate::zone::Zone;

        // Blood Artist: "Whenever Blood Artist or another creature dies,
        // target player loses 1 life and you gain 1 life."
        let blood_artist_ability = Ability {
            kind: AbilityKind::Triggered(TriggeredAbility {
                // Triggers on ANY creature dying (including itself)
                trigger: Trigger::dies(ObjectFilter::creature()),
                effects: vec![
                    // "target player loses 1 life"
                    Effect::lose_life_target(1),
                    // "you gain 1 life"
                    Effect::gain_life(1),
                ],
                choices: vec![ChooseSpec::target_player()],
                intervening_if: None,
            }),
            functional_zones: vec![Zone::Battlefield],
            text: Some(
                "Whenever Blood Artist or another creature dies, \
                 target player loses 1 life and you gain 1 life."
                    .to_string(),
            ),
        };

        // Verify it's a triggered ability
        let AbilityKind::Triggered(triggered) = &blood_artist_ability.kind else {
            panic!("Expected triggered ability");
        };

        // Verify trigger condition is creature dies
        assert!(triggered.trigger.display().contains("creature dies"));

        // Verify it has two effects (LoseLifeEffect and GainLifeEffect)
        assert_eq!(triggered.effects.len(), 2);
        let debug_str_0 = format!("{:?}", &triggered.effects[0]);
        let debug_str_1 = format!("{:?}", &triggered.effects[1]);
        assert!(debug_str_0.contains("LoseLifeEffect") || debug_str_0.contains("GainLifeEffect"));
        assert!(debug_str_1.contains("LoseLifeEffect") || debug_str_1.contains("GainLifeEffect"));

        // Verify it requires a target (ChooseSpec::target_player() = Target(Player(Any)))
        assert_eq!(triggered.choices.len(), 1);
        assert!(triggered.choices[0].is_target());
        assert!(matches!(
            triggered.choices[0].inner(),
            ChooseSpec::Player(PlayerFilter::Any)
        ));

        // Verify it functions on the battlefield
        assert!(blood_artist_ability.functions_in(&Zone::Battlefield));
        assert!(!blood_artist_ability.functions_in(&Zone::Graveyard));
    }
}
