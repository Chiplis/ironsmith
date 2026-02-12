use std::collections::HashMap;

use crate::ability::Ability;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::{Card, PtValue};
use crate::color::ColorSet;
use crate::cost::{OptionalCost, OptionalCostsPaid};
use crate::ids::{CardId, ObjectId, PlayerId, StableId};
use crate::mana::ManaCost;
use crate::player::ManaPool;
use crate::types::{CardType, Subtype, Supertype};
use crate::zone::Zone;

/// Types of counters that can be placed on objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterType {
    // === P/T modifying counters ===
    PlusOnePlusOne,
    MinusOneMinusOne,

    // === Ability-granting counters (from Ikoria and other sets) ===
    Deathtouch,
    DoubleStrike,
    FirstStrike,
    Flying,
    Haste,
    Hexproof,
    Indestructible,
    Lifelink,
    Menace,
    Reach,
    Trample,
    Vigilance,

    // === Resource/tracking counters ===
    Loyalty,
    Charge,
    Age,
    Aim,
    Arrow,
    Awakening,
    Blood,
    Brain,
    Bounty,
    Brick,
    Corpse,
    Credit,
    Crystal,
    Cube,
    Currency,
    Death,
    Depletion,
    Despair,
    Devotion,
    Divinity,
    Doom,
    Dream,
    Echo,
    Egg,
    Energy,
    Enlightened,
    Eon,
    Experience,
    Eyeball,
    Fade,
    Fate,
    Feather,
    Filibuster,
    Flame,
    Flood,
    Foreshadow,
    Fungus,
    Fuse,
    Gem,
    Glyph,
    Gold,
    Growth,
    Hatchling,
    Healing,
    Hit,
    Hoofprint,
    Hour,
    Hunger,
    Ice,
    Incarnation,
    Infection,
    Intervention,
    Isolation,
    Javelin,
    Ki,
    Keyword,
    Knowledge,
    Level,
    Lore,
    Luck,
    Magnet,
    Manifestation,
    Mannequin,
    Matrix,
    Mine,
    Mining,
    Mire,
    Music,
    Muster,
    Net,
    Night,
    Oil,
    Omen,
    Ore,
    Page,
    Pain,
    Paralyzation,
    Petal,
    Petrification,
    Phylactery,
    Pin,
    Plague,
    Plot,
    Polyp,
    Poison,
    Pressure,
    Prey,
    Pupa,
    Quest,
    Rad,
    Scream,
    Shield,
    Silver,
    Sleep,
    Slime,
    Slumber,
    Soot,
    Soul,
    Spore,
    Storage,
    Strife,
    Study,
    Stun,
    Task,
    Theft,
    Tide,
    Time,
    Tower,
    Training,
    Trap,
    Treasure,
    Unity,
    Velocity,
    Verse,
    Vitality,
    Volatile,
    Voyage,
    Wage,
    Winch,
    Wind,
    Wish,
}

/// The kind of game object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// A physical card
    Card,
    /// A token permanent
    Token,
    /// A copy of a spell on the stack
    SpellCopy,
    /// An emblem (from planeswalker ultimates)
    Emblem,
}

/// Runtime representation of a game object.
/// Contains both copiable values (layer 1) and non-copiable state.
#[derive(Debug, Clone)]
pub struct Object {
    // Identity
    pub id: ObjectId,
    /// Stable identifier that persists across zone changes.
    /// Unlike `id` which changes when an object moves zones (per MTG rule 400.7),
    /// `stable_id` stays constant for the lifetime of this card/token instance.
    /// Useful for tracking "this specific card" for display and triggered abilities.
    pub stable_id: StableId,
    pub kind: ObjectKind,
    /// Reference to the original card definition (None for pure tokens)
    pub card: Option<CardId>,
    pub zone: Zone,

    // Ownership (owner never changes, controller can)
    pub owner: PlayerId,
    pub controller: PlayerId,

    // Copiable values (what Clone effects copy)
    pub name: String,
    pub mana_cost: Option<ManaCost>,
    pub color_override: Option<ColorSet>,
    pub supertypes: Vec<Supertype>,
    pub card_types: Vec<CardType>,
    pub subtypes: Vec<Subtype>,
    pub oracle_text: String,
    pub base_power: Option<PtValue>,
    pub base_toughness: Option<PtValue>,
    pub base_loyalty: Option<u32>,
    /// Abilities this object has (copiable)
    pub abilities: Vec<Ability>,

    // Non-copiable values (kept on Object)
    pub counters: HashMap<CounterType, u32>,
    pub attached_to: Option<ObjectId>,
    pub attachments: Vec<ObjectId>,

    // Spell-related state
    /// Spell effects (for instants/sorceries)
    pub spell_effect: Option<Vec<crate::effect::Effect>>,
    /// For Auras: what this card can enchant (used for non-target attachments)
    pub aura_attach_filter: Option<crate::target::ObjectFilter>,
    /// Alternative casting methods (flashback, escape, etc.)
    pub alternative_casts: Vec<AlternativeCastingMethod>,
    /// Optional costs (kicker, buyback, etc.)
    pub optional_costs: Vec<OptionalCost>,
    /// Which optional costs were paid when this spell was cast (for ETB triggers)
    pub optional_costs_paid: OptionalCostsPaid,
    /// Mana actually spent to cast this object while it was a spell.
    /// Used by conditional text like "if at least three blue mana was spent to cast this spell".
    pub mana_spent_to_cast: ManaPool,
    /// Cost effects (new unified model) - effects executed as part of paying costs.
    pub cost_effects: Vec<crate::effect::Effect>,

    // === Saga fields ===
    /// For sagas: the maximum chapter number (typically 3)
    pub max_saga_chapter: Option<u32>,
    // Note: The following fields have been moved to GameState extension maps:
    // - tapped -> GameState::tapped_permanents
    // - flipped -> GameState::flipped
    // - face_down -> GameState::face_down
    // - phased_out -> GameState::phased_out
    // - damage_marked -> GameState::damage_marked
    // - summoning_sick -> GameState::summoning_sick
    // - is_monstrous -> GameState::monstrous
    // - regeneration_shields -> GameState::regeneration_shields
    // - madness_exiled -> GameState::madness_exiled
    // - final_chapter_resolved -> GameState::saga_final_chapter_resolved
    // - is_commander -> GameState::commanders
}

impl Object {
    /// Creates a new object from a card definition.
    pub fn from_card(id: ObjectId, card: &Card, owner: PlayerId, zone: Zone) -> Self {
        let (base_power, base_toughness) = card
            .power_toughness
            .map(|pt| (Some(pt.power), Some(pt.toughness)))
            .unwrap_or((None, None));

        Self {
            id,
            stable_id: StableId::from(id), // Set to same as id initially; preserved across zone changes
            kind: ObjectKind::Card,
            card: Some(card.id),
            zone,
            owner,
            controller: owner,
            name: card.name.clone(),
            mana_cost: card.mana_cost.clone(),
            color_override: card.color_indicator,
            supertypes: card.supertypes.clone(),
            card_types: card.card_types.clone(),
            subtypes: card.subtypes.clone(),
            oracle_text: card.oracle_text.clone(),
            base_power,
            base_toughness,
            base_loyalty: card.loyalty,
            abilities: Vec::new(),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            cost_effects: Vec::new(),
            max_saga_chapter: None,
        }
    }

    /// Creates a new object from a CardDefinition (card + abilities + spell effects).
    pub fn from_card_definition(
        id: ObjectId,
        def: &crate::cards::CardDefinition,
        owner: PlayerId,
        zone: Zone,
    ) -> Self {
        let mut obj = Self::from_card(id, &def.card, owner, zone);
        obj.abilities = def.abilities.clone();
        obj.spell_effect = def.spell_effect.clone();
        obj.aura_attach_filter = def.aura_attach_filter.clone();
        obj.alternative_casts = def.alternative_casts.clone();
        obj.optional_costs = def.optional_costs.clone();
        obj.max_saga_chapter = def.max_saga_chapter;
        obj.cost_effects = def.cost_effects.clone();
        obj
    }

    /// Creates a new token.
    #[allow(clippy::too_many_arguments)]
    pub fn new_token(
        id: ObjectId,
        owner: PlayerId,
        name: String,
        card_types: Vec<CardType>,
        subtypes: Vec<Subtype>,
        power: Option<i32>,
        toughness: Option<i32>,
        color: ColorSet,
    ) -> Self {
        Self {
            id,
            stable_id: StableId::from(id), // New token gets its own stable_id
            kind: ObjectKind::Token,
            card: None,
            zone: Zone::Battlefield,
            owner,
            controller: owner,
            name,
            mana_cost: None,
            color_override: Some(color),
            supertypes: Vec::new(),
            card_types,
            subtypes,
            oracle_text: String::new(),
            base_power: power.map(PtValue::Fixed),
            base_toughness: toughness.map(PtValue::Fixed),
            base_loyalty: None,
            abilities: Vec::new(),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            cost_effects: Vec::new(),
            max_saga_chapter: None,
        }
    }

    /// Creates a token that's a copy of another object.
    /// Per MTG rules, tokens copy copiable values but not non-copiable state.
    /// Note: Battlefield state (tapped, summoning_sick, etc.) is managed via GameState extension maps.
    pub fn token_copy_of(source: &Object, id: ObjectId, owner: PlayerId) -> Self {
        let mut token = Self {
            id,
            stable_id: StableId::from(id), // Token copy is a new instance
            kind: ObjectKind::Token,
            card: None,
            zone: Zone::Battlefield,
            owner,
            controller: owner,
            // Copiable values from source
            name: source.name.clone(),
            mana_cost: source.mana_cost.clone(),
            color_override: source.color_override,
            supertypes: source.supertypes.clone(),
            card_types: source.card_types.clone(),
            subtypes: source.subtypes.clone(),
            oracle_text: source.oracle_text.clone(),
            base_power: source.base_power,
            base_toughness: source.base_toughness,
            base_loyalty: source.base_loyalty,
            abilities: source.abilities.clone(),
            // Non-copiable values reset to defaults
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            // Note: spell_effect is copiable for spell copies
            spell_effect: source.spell_effect.clone(),
            aura_attach_filter: source.aura_attach_filter.clone(),
            // Alternative casts are copiable (though tokens rarely use them)
            alternative_casts: source.alternative_casts.clone(),
            // Optional costs are copiable
            optional_costs: source.optional_costs.clone(),
            // Optional costs paid is non-copiable (tokens weren't cast)
            optional_costs_paid: OptionalCostsPaid::default(),
            // Tokens are never cast.
            mana_spent_to_cast: ManaPool::default(),
            // Cost effects are copiable
            cost_effects: source.cost_effects.clone(),
            // Saga fields - copiable (a token copy of a saga is also a saga)
            max_saga_chapter: source.max_saga_chapter,
        };
        // Planeswalker tokens enter with loyalty counters equal to base loyalty
        if let Some(loyalty) = source.base_loyalty {
            token.add_counters(CounterType::Loyalty, loyalty);
        }
        token
    }

    /// Creates a new emblem in the command zone.
    ///
    /// Emblems are permanent game objects created by planeswalker ultimates.
    /// They exist in the command zone and cannot be interacted with by most
    /// game mechanics (they have no controller change, can't be destroyed, etc.)
    pub fn new_emblem(
        id: ObjectId,
        owner: PlayerId,
        name: String,
        abilities: Vec<Ability>,
    ) -> Self {
        Self {
            id,
            stable_id: StableId::from(id), // Emblems get their own stable_id
            kind: ObjectKind::Emblem,
            card: None,
            zone: Zone::Command,
            owner,
            controller: owner,
            name,
            mana_cost: None,
            color_override: None,
            supertypes: Vec::new(),
            card_types: Vec::new(),
            subtypes: Vec::new(),
            oracle_text: String::new(),
            base_power: None,
            base_toughness: None,
            base_loyalty: None,
            abilities,
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            cost_effects: Vec::new(),
            max_saga_chapter: None,
        }
    }

    /// Copies copiable values from another object (for Clone effects).
    /// Per MTG rule 707.2, copiable values are: name, mana cost, color, card types,
    /// subtypes, supertypes, rules text, power, toughness, loyalty, and abilities.
    /// Non-copiable state (counters, damage, etc.) is NOT copied.
    pub fn copy_copiable_values_from(&mut self, source: &Object) {
        self.name = source.name.clone();
        self.mana_cost = source.mana_cost.clone();
        self.color_override = source.color_override;
        self.supertypes = source.supertypes.clone();
        self.card_types = source.card_types.clone();
        self.subtypes = source.subtypes.clone();
        self.oracle_text = source.oracle_text.clone();
        self.base_power = source.base_power;
        self.base_toughness = source.base_toughness;
        self.base_loyalty = source.base_loyalty;
        self.abilities = source.abilities.clone();
        self.aura_attach_filter = source.aura_attach_filter.clone();
    }

    /// Returns the colors of this object.
    pub fn colors(&self) -> ColorSet {
        // Devoid applies in all functional zones of the ability.
        if self.abilities.iter().any(|ability| {
            ability.functions_in(&self.zone)
                && matches!(
                    &ability.kind,
                    crate::ability::AbilityKind::Static(static_ability) if static_ability.is_devoid()
                )
        }) {
            return ColorSet::COLORLESS;
        }

        if let Some(override_colors) = self.color_override {
            return override_colors;
        }

        let Some(mana_cost) = &self.mana_cost else {
            return ColorSet::COLORLESS;
        };

        use crate::color::Color;
        use crate::mana::ManaSymbol;

        let mut colors = ColorSet::COLORLESS;
        for pip in mana_cost.pips() {
            for symbol in pip {
                match symbol {
                    ManaSymbol::White => colors = colors.with(Color::White),
                    ManaSymbol::Blue => colors = colors.with(Color::Blue),
                    ManaSymbol::Black => colors = colors.with(Color::Black),
                    ManaSymbol::Red => colors = colors.with(Color::Red),
                    ManaSymbol::Green => colors = colors.with(Color::Green),
                    _ => {}
                }
            }
        }
        colors
    }

    /// Returns the color identity of this object (for Commander format).
    /// Color identity includes colors from:
    /// - Mana cost
    /// - Color indicator/override
    /// - Mana symbols in rules text (e.g., "{T}: Add {G}")
    pub fn color_identity(&self) -> ColorSet {
        use crate::color::Color;
        use crate::mana::ManaSymbol;

        let mut identity = ColorSet::COLORLESS;

        // Add colors from mana cost
        if let Some(mana_cost) = &self.mana_cost {
            for pip in mana_cost.pips() {
                for symbol in pip {
                    match symbol {
                        ManaSymbol::White => identity = identity.with(Color::White),
                        ManaSymbol::Blue => identity = identity.with(Color::Blue),
                        ManaSymbol::Black => identity = identity.with(Color::Black),
                        ManaSymbol::Red => identity = identity.with(Color::Red),
                        ManaSymbol::Green => identity = identity.with(Color::Green),
                        _ => {}
                    }
                }
            }
        }

        // Add colors from color indicator/override
        if let Some(override_colors) = self.color_override {
            identity = identity.union(override_colors);
        }

        // Parse oracle text for mana symbols
        identity = identity.union(Self::parse_colors_from_text(&self.oracle_text));

        identity
    }

    /// Parses mana symbols from rules text and returns the colors found.
    fn parse_colors_from_text(text: &str) -> ColorSet {
        use crate::color::Color;

        let mut colors = ColorSet::COLORLESS;
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            if chars[i] == '{' {
                // Find the closing brace
                if let Some(end) = chars[i..].iter().position(|&c| c == '}') {
                    let symbol: String = chars[i + 1..i + end].iter().collect();
                    // Check for color symbols (including in hybrid like "W/U")
                    for c in symbol.chars() {
                        match c {
                            'W' => colors = colors.with(Color::White),
                            'U' => colors = colors.with(Color::Blue),
                            'B' => colors = colors.with(Color::Black),
                            'R' => colors = colors.with(Color::Red),
                            'G' => colors = colors.with(Color::Green),
                            _ => {}
                        }
                    }
                    i += end + 1;
                    continue;
                }
            }
            i += 1;
        }
        colors
    }

    /// Returns the current power of this creature.
    /// Returns None if this is not a creature.
    pub fn power(&self) -> Option<i32> {
        // Check for level abilities first - they can override base P/T
        let base = if let Some((power, _)) = self.level_ability_pt() {
            power
        } else {
            self.base_power?.base_value()
        };
        let counter_bonus = self
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0) as i32;
        let counter_penalty = self
            .counters
            .get(&CounterType::MinusOneMinusOne)
            .copied()
            .unwrap_or(0) as i32;
        Some(base + counter_bonus - counter_penalty)
    }

    /// Returns the current toughness of this creature.
    /// Returns None if this is not a creature.
    pub fn toughness(&self) -> Option<i32> {
        // Check for level abilities first - they can override base P/T
        let base = if let Some((_, toughness)) = self.level_ability_pt() {
            toughness
        } else {
            self.base_toughness?.base_value()
        };
        let counter_bonus = self
            .counters
            .get(&CounterType::PlusOnePlusOne)
            .copied()
            .unwrap_or(0) as i32;
        let counter_penalty = self
            .counters
            .get(&CounterType::MinusOneMinusOne)
            .copied()
            .unwrap_or(0) as i32;
        Some(base + counter_bonus - counter_penalty)
    }

    /// Returns the P/T override from level abilities if applicable.
    /// Returns None if there are no level abilities or the current level tier has no P/T override.
    fn level_ability_pt(&self) -> Option<(i32, i32)> {
        use crate::ability::AbilityKind;

        let level_count = self.counters.get(&CounterType::Level).copied().unwrap_or(0);

        for ability in &self.abilities {
            if let AbilityKind::Static(s) = &ability.kind
                && let Some(levels) = s.level_abilities()
            {
                // Find the matching tier (highest tier that applies)
                for tier in levels.iter().rev() {
                    if level_count >= tier.min_level
                        && tier.max_level.is_none_or(|max| level_count <= max)
                    {
                        return tier.power_toughness;
                    }
                }
            }
        }
        None
    }

    /// Returns all static abilities granted by the current level tier.
    pub fn level_granted_abilities(&self) -> Vec<crate::static_abilities::StaticAbility> {
        use crate::ability::AbilityKind;

        let level_count = self.counters.get(&CounterType::Level).copied().unwrap_or(0);

        for ability in &self.abilities {
            if let AbilityKind::Static(s) = &ability.kind
                && let Some(levels) = s.level_abilities()
            {
                // Find the matching tier (highest tier that applies)
                for tier in levels.iter().rev() {
                    if level_count >= tier.min_level
                        && tier.max_level.is_none_or(|max| level_count <= max)
                    {
                        // Abilities are now stored as the new type directly
                        return tier.abilities.clone();
                    }
                }
            }
        }
        Vec::new()
    }

    /// Returns the current loyalty of this planeswalker.
    pub fn loyalty(&self) -> Option<u32> {
        let base = self.base_loyalty?;
        let loyalty_counters = self
            .counters
            .get(&CounterType::Loyalty)
            .copied()
            .unwrap_or(0);
        Some(base + loyalty_counters)
    }

    /// Adds counters of the specified type.
    pub fn add_counters(&mut self, counter_type: CounterType, amount: u32) {
        *self.counters.entry(counter_type).or_insert(0) += amount;
    }

    /// Removes counters of the specified type. Returns the number actually removed.
    pub fn remove_counters(&mut self, counter_type: CounterType, amount: u32) -> u32 {
        let current = self.counters.entry(counter_type).or_insert(0);
        let removed = (*current).min(amount);
        *current -= removed;
        if *current == 0 {
            self.counters.remove(&counter_type);
        }
        removed
    }

    /// Returns true if this creature has taken lethal damage.
    /// `damage_marked` should be obtained from GameState::damage_on(id).
    pub fn has_lethal_damage(&self, damage_marked: u32) -> bool {
        if let Some(toughness) = self.toughness() {
            toughness <= 0 || damage_marked >= toughness as u32
        } else {
            false
        }
    }

    /// Returns true if this object has the given card type.
    pub fn has_card_type(&self, card_type: CardType) -> bool {
        self.card_types.contains(&card_type)
    }

    /// Returns true if this object has the given supertype.
    pub fn has_supertype(&self, supertype: Supertype) -> bool {
        self.supertypes.contains(&supertype)
    }

    /// Returns true if this object has the given subtype.
    ///
    /// If the object has Changeling and is a creature, it has all creature types.
    pub fn has_subtype(&self, subtype: Subtype) -> bool {
        if self.subtypes.contains(&subtype) {
            return true;
        }

        // Changeling means this creature is every creature type
        if subtype.is_creature_type() && self.is_creature() && self.has_changeling() {
            return true;
        }

        false
    }

    /// Returns true if this object has the Changeling ability.
    pub fn has_changeling(&self) -> bool {
        use crate::ability::AbilityKind;
        self.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s.is_changeling()
            } else {
                false
            }
        })
    }

    /// Returns true if this is a creature.
    pub fn is_creature(&self) -> bool {
        self.has_card_type(CardType::Creature)
    }

    /// Returns true if this is a land.
    pub fn is_land(&self) -> bool {
        self.has_card_type(CardType::Land)
    }

    /// Returns true if this is a permanent type.
    pub fn is_permanent(&self) -> bool {
        self.has_card_type(CardType::Creature)
            || self.has_card_type(CardType::Artifact)
            || self.has_card_type(CardType::Enchantment)
            || self.has_card_type(CardType::Land)
            || self.has_card_type(CardType::Planeswalker)
            || self.has_card_type(CardType::Battle)
    }

    /// Returns true if this is legendary.
    pub fn is_legendary(&self) -> bool {
        self.has_supertype(Supertype::Legendary)
    }

    /// Returns true if this object has the given static ability.
    /// This includes abilities granted by level tiers.
    pub fn has_static_ability(&self, ability: &crate::static_abilities::StaticAbility) -> bool {
        use crate::ability::AbilityKind;

        // Check regular static abilities
        let has_regular = self.abilities.iter().any(|a| {
            if let AbilityKind::Static(s) = &a.kind {
                s == ability
            } else {
                false
            }
        });

        if has_regular {
            return true;
        }

        // Check level-granted abilities
        self.level_granted_abilities().iter().any(|a| a == ability)
    }

    /// Returns true if this object has indestructible.
    pub fn has_indestructible(&self) -> bool {
        self.has_static_ability(&crate::static_abilities::StaticAbility::indestructible())
    }

    /// Creates a token from a CardDefinition.
    ///
    /// The CardDefinition should have been built with `.token()` to mark it as a token.
    /// This is the preferred way to create tokens - use CardDefinitionBuilder with all
    /// the normal ability methods instead of the deprecated TokenDescription.
    /// Note: Battlefield state (summoning_sick, etc.) is managed via GameState extension maps.
    pub fn from_token_definition(
        id: ObjectId,
        def: &crate::cards::CardDefinition,
        controller: PlayerId,
    ) -> Self {
        Self {
            id,
            stable_id: StableId::from(id),
            kind: ObjectKind::Token,
            card: None,
            zone: Zone::Battlefield,
            owner: controller,
            controller,
            name: def.card.name.clone(),
            mana_cost: None,                          // Tokens don't have mana costs
            color_override: def.card.color_indicator, // Use color indicator if set
            supertypes: def.card.supertypes.clone(),
            card_types: def.card.card_types.clone(),
            subtypes: def.card.subtypes.clone(),
            oracle_text: def.card.oracle_text.clone(),
            base_power: def.card.power_toughness.map(|pt| pt.power),
            base_toughness: def.card.power_toughness.map(|pt| pt.toughness),
            base_loyalty: def.card.loyalty,
            abilities: def.abilities.clone(),
            counters: HashMap::new(),
            attached_to: None,
            attachments: Vec::new(),
            spell_effect: def.spell_effect.clone(),
            aura_attach_filter: def.aura_attach_filter.clone(),
            alternative_casts: def.alternative_casts.clone(),
            optional_costs: def.optional_costs.clone(),
            optional_costs_paid: OptionalCostsPaid::default(),
            mana_spent_to_cast: ManaPool::default(),
            cost_effects: def.cost_effects.clone(),
            max_saga_chapter: def.max_saga_chapter,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Ability;
    use crate::card::CardBuilder;
    use crate::color::Color;
    use crate::mana::ManaSymbol;
    use crate::static_abilities::StaticAbility;
    use crate::target::ObjectFilter;

    #[test]
    fn test_object_from_card() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        assert_eq!(obj.name, "Grizzly Bears");
        assert_eq!(obj.power(), Some(2));
        assert_eq!(obj.toughness(), Some(2));
        assert!(obj.is_creature());
        assert!(obj.colors().contains(Color::Green));
    }

    #[test]
    fn test_token_creation() {
        let token = Object::new_token(
            ObjectId::from_raw(1),
            PlayerId::from_index(0),
            "Soldier".to_string(),
            vec![CardType::Creature],
            vec![Subtype::Soldier],
            Some(1),
            Some(1),
            ColorSet::WHITE,
        );

        assert_eq!(token.name, "Soldier");
        assert_eq!(token.kind, ObjectKind::Token);
        assert_eq!(token.power(), Some(1));
        assert_eq!(token.toughness(), Some(1));
        assert!(token.colors().contains(Color::White));
        // Note: summoning_sick is now tracked in GameState::summoning_sick
    }

    #[test]
    fn test_devoid_applies_in_hand() {
        let card = CardBuilder::new(CardId::from_raw(1), "Devoid Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 1))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Hand,
        );
        obj.abilities.push(
            Ability::static_ability(StaticAbility::make_colorless(ObjectFilter::source())).in_zones(
                vec![
                    Zone::Battlefield,
                    Zone::Stack,
                    Zone::Hand,
                    Zone::Library,
                    Zone::Graveyard,
                    Zone::Exile,
                    Zone::Command,
                ],
            ),
        );

        assert!(obj.colors().is_empty(), "devoid object in hand should be colorless");
    }

    #[test]
    fn test_make_colorless_ability_respects_functional_zone() {
        let card = CardBuilder::new(CardId::from_raw(1), "Color Probe")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Blue],
            ]))
            .card_types(vec![CardType::Creature])
            .power_toughness(crate::card::PowerToughness::fixed(2, 1))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Hand,
        );
        obj.abilities
            .push(Ability::static_ability(StaticAbility::make_colorless(
                ObjectFilter::source(),
            )));

        assert!(
            obj.colors().contains(Color::Blue),
            "battlefield-only make-colorless should not apply in hand"
        );
    }

    #[test]
    fn test_counters() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        // Add +1/+1 counters
        obj.add_counters(CounterType::PlusOnePlusOne, 3);
        assert_eq!(obj.power(), Some(5));
        assert_eq!(obj.toughness(), Some(5));

        // Remove some counters
        let removed = obj.remove_counters(CounterType::PlusOnePlusOne, 2);
        assert_eq!(removed, 2);
        assert_eq!(obj.power(), Some(3));
        assert_eq!(obj.toughness(), Some(3));
    }

    #[test]
    fn test_lethal_damage() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        // damage_marked is now tracked in GameState::damage_marked
        assert!(!obj.has_lethal_damage(0));
        assert!(!obj.has_lethal_damage(1));
        assert!(obj.has_lethal_damage(2));
    }

    #[test]
    fn test_minus_counters() {
        let card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let mut obj = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        obj.add_counters(CounterType::MinusOneMinusOne, 1);
        assert_eq!(obj.power(), Some(1));
        assert_eq!(obj.toughness(), Some(1));

        // With enough -1/-1 counters, toughness goes to 0 or below
        obj.add_counters(CounterType::MinusOneMinusOne, 1);
        assert_eq!(obj.toughness(), Some(0));
        assert!(obj.has_lethal_damage(0)); // 0 toughness = lethal even with no damage
    }

    #[test]
    fn test_token_copy_of() {
        let card = CardBuilder::new(CardId::from_raw(1), "Serra Angel")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::White],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .oracle_text("Flying, vigilance")
            .power_toughness(crate::card::PowerToughness::fixed(4, 4))
            .build();

        let mut original = Object::from_card(
            ObjectId::from_raw(1),
            &card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );

        // Add some non-copiable state to the original
        original.add_counters(CounterType::PlusOnePlusOne, 2);
        // Note: tapped, damage_marked, summoning_sick are now in GameState extension maps

        // Create a token copy
        let token =
            Object::token_copy_of(&original, ObjectId::from_raw(2), PlayerId::from_index(1));

        // Copiable values should match
        assert_eq!(token.name, "Serra Angel");
        assert_eq!(token.base_power, Some(PtValue::Fixed(4)));
        assert_eq!(token.base_toughness, Some(PtValue::Fixed(4)));
        assert!(token.has_subtype(Subtype::Angel));
        assert_eq!(token.oracle_text, "Flying, vigilance");

        // Non-copiable state should NOT be copied
        assert_eq!(token.counters.get(&CounterType::PlusOnePlusOne), None);
        // Note: damage_marked, tapped, summoning_sick are now in GameState extension maps

        // Token-specific properties
        assert_eq!(token.kind, ObjectKind::Token);
        assert_eq!(token.owner, PlayerId::from_index(1));
        assert_eq!(token.controller, PlayerId::from_index(1));
    }

    #[test]
    fn test_copy_copiable_values_from() {
        let bear_card = CardBuilder::new(CardId::from_raw(1), "Grizzly Bears")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(1)],
                vec![ManaSymbol::Green],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Bear])
            .power_toughness(crate::card::PowerToughness::fixed(2, 2))
            .build();

        let angel_card = CardBuilder::new(CardId::from_raw(2), "Serra Angel")
            .mana_cost(ManaCost::from_pips(vec![
                vec![ManaSymbol::Generic(3)],
                vec![ManaSymbol::White],
                vec![ManaSymbol::White],
            ]))
            .card_types(vec![CardType::Creature])
            .subtypes(vec![Subtype::Angel])
            .oracle_text("Flying, vigilance")
            .power_toughness(crate::card::PowerToughness::fixed(4, 4))
            .build();

        // Create a Clone creature that enters as a copy of Serra Angel
        let mut clone = Object::from_card(
            ObjectId::from_raw(1),
            &bear_card,
            PlayerId::from_index(0),
            Zone::Battlefield,
        );
        clone.add_counters(CounterType::PlusOnePlusOne, 1);

        let angel = Object::from_card(
            ObjectId::from_raw(2),
            &angel_card,
            PlayerId::from_index(1),
            Zone::Battlefield,
        );

        // Clone copies the angel
        clone.copy_copiable_values_from(&angel);

        // Copiable values now match the angel
        assert_eq!(clone.name, "Serra Angel");
        assert_eq!(clone.power(), Some(5)); // 4 base + 1 counter
        assert_eq!(clone.toughness(), Some(5));
        assert!(clone.has_subtype(Subtype::Angel));
        assert!(!clone.has_subtype(Subtype::Bear));

        // But identity fields remain unchanged
        assert_eq!(clone.id, ObjectId::from_raw(1));
        assert_eq!(clone.owner, PlayerId::from_index(0));
        assert_eq!(clone.controller, PlayerId::from_index(0));

        // And counters are preserved (non-copiable)
        assert_eq!(clone.counters.get(&CounterType::PlusOnePlusOne), Some(&1));
    }
}
