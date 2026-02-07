//! Card database module for MTG.
//!
//! This module provides a structured way to define cards with their abilities.
//! Cards are defined programmatically for type safety and LLM-friendliness.
//!
//! Each card is defined in its own file under `definitions/` for easy tracking.

pub mod builders;
pub mod definitions;
pub mod tokens;

pub use builders::{CardDefinitionBuilder, ParseAnnotations, TextSpan};
pub use definitions::*;

#[cfg(test)]
mod parse_snapshots;

mod generated_registry {
    include!(concat!(env!("OUT_DIR"), "/generated_registry.rs"));
}

use crate::ability::Ability;
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::Card;
use crate::cost::OptionalCost;
use crate::effect::Effect;
use crate::target::ObjectFilter;
use std::collections::HashMap;

/// A complete card definition including the card data and its abilities.
///
/// This combines the static card data with the structured ability definitions,
/// and optionally spell effects for instants/sorceries.
#[derive(Debug, Clone)]
pub struct CardDefinition {
    /// The static card data (name, types, P/T, etc.)
    pub card: Card,

    /// The abilities this card has
    pub abilities: Vec<Ability>,

    /// For instants/sorceries: the effects when the spell resolves
    pub spell_effect: Option<Vec<Effect>>,

    /// For Auras: what this card can enchant (used for non-target attachments)
    pub aura_attach_filter: Option<ObjectFilter>,

    /// Alternative casting methods (flashback, escape, etc.)
    pub alternative_casts: Vec<AlternativeCastingMethod>,

    /// Optional costs (kicker, buyback, etc.)
    pub optional_costs: Vec<OptionalCost>,

    /// For sagas: the maximum chapter number (typically 3)
    pub max_saga_chapter: Option<u32>,

    /// Cost effects (new unified model) - effects that are executed as part of paying costs.
    /// These run with `EventCause::from_cost()` and enable triggers on cost-related events.
    pub cost_effects: Vec<Effect>,
}

impl CardDefinition {
    /// Create a new card definition.
    pub fn new(card: Card) -> Self {
        Self {
            card,
            abilities: Vec::new(),
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            cost_effects: Vec::new(),
        }
    }

    /// Create a card definition with abilities.
    pub fn with_abilities(card: Card, abilities: Vec<Ability>) -> Self {
        Self {
            card,
            abilities,
            spell_effect: None,
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            cost_effects: Vec::new(),
        }
    }

    /// Create a spell card definition.
    pub fn spell(card: Card, effects: Vec<Effect>) -> Self {
        Self {
            card,
            abilities: Vec::new(),
            spell_effect: Some(effects),
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            cost_effects: Vec::new(),
        }
    }

    /// Create a spell card definition with additional abilities.
    pub fn spell_with_abilities(card: Card, effects: Vec<Effect>, abilities: Vec<Ability>) -> Self {
        Self {
            card,
            abilities,
            spell_effect: Some(effects),
            aura_attach_filter: None,
            alternative_casts: Vec::new(),
            optional_costs: Vec::new(),
            max_saga_chapter: None,
            cost_effects: Vec::new(),
        }
    }

    /// Get the card's name.
    pub fn name(&self) -> &str {
        &self.card.name
    }

    /// Check if this is a creature.
    pub fn is_creature(&self) -> bool {
        self.card.is_creature()
    }

    /// Check if this is an instant or sorcery.
    pub fn is_spell(&self) -> bool {
        self.card.is_instant() || self.card.is_sorcery()
    }

    /// Check if this is a permanent type.
    pub fn is_permanent(&self) -> bool {
        self.card.is_creature()
            || self.card.is_artifact()
            || self.card.is_enchantment()
            || self.card.is_land()
            || self.card.is_planeswalker()
    }
}

/// Registry of all card definitions.
///
/// Provides lookup by name and other queries.
#[derive(Debug, Clone, Default)]
pub struct CardRegistry {
    /// Cards indexed by name
    cards: HashMap<String, CardDefinition>,
}

impl CardRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
        }
    }

    /// Create a card registry.
    ///
    /// In test builds this includes hand-written definitions plus generated parser cards.
    /// In non-test builds this is populated from generated parser cards only.
    pub fn with_builtin_cards() -> Self {
        let mut registry = Self::new();

        #[cfg(test)]
        {
            // Register all cards from definitions module
            registry.register(llanowar_elves());
            registry.register(chrome_mox());
            registry.register(command_the_mind());
            registry.register(serra_angel());
            registry.register(grizzly_bears());
            registry.register(lightning_bolt());
            registry.register(doom_blade());
            registry.register(demonic_tutor());
            registry.register(enlightened_tutor());
            registry.register(emrakul_the_promised_end());
            registry.register(everflowing_chalice());
            registry.register(force_of_will());
            registry.register(giant_growth());
            registry.register(mindbreak_trap());
            registry.register(counterspell());
            registry.register(dawn_charm());
            registry.register(swords_to_plowshares());
            registry.register(basic_forest());
            registry.register(basic_island());
            registry.register(basic_mountain());
            registry.register(basic_plains());
            registry.register(basic_swamp());
            registry.register(ornithopter());
            registry.register(murder_of_crows());
            registry.register(goblin_guide());
            registry.register(typhoid_rats());
            registry.register(vampire_nighthawk());
            registry.register(silhana_ledgewalker());
            registry.register(thorn_elemental());
            registry.register(mirran_crusader());
            registry.register(crusade());
            registry.register(stormbreath_dragon());
            registry.register(geist_of_saint_traft());
            registry.register(savannah_lions());
            registry.register(saw_in_half());
            registry.register(white_knight());
            registry.register(giant_spider());
            registry.register(wall_of_omens());
            registry.register(boggart_brute());
            registry.register(darksteel_colossus());
            registry.register(snapcaster_mage());
            registry.register(underworld_breach());
            registry.register(frogmite());
            registry.register(treasure_cruise());
            registry.register(stoke_the_flames());
            registry.register(reverse_engineer());
            registry.register(the_birth_of_meletis());
            registry.register(student_of_warfare());
            registry.register(valley_floodcaller());
            registry.register(yawgmoth_thran_physician());
            registry.register(yawgmoths_will());
            registry.register(butcher_ghoul());
            registry.register(sightless_ghoul());
            registry.register(hex_parasite());
            registry.register(fireball());
            registry.register(think_twice());
            registry.register(urzas_saga());
            registry.register(fate_transfer());
            registry.register(accursed_marauder());
            registry.register(accursed_duneyard());
            registry.register(akromas_will());
            registry.register(amulet_of_vigor());
            registry.register(ancient_tomb());
            registry.register(arcane_signet());
            registry.register(arid_mesa());
            registry.register(ashnods_altar());
            registry.register(bello_bard_of_the_brambles());
            registry.register(blade_of_the_bloodchief());
            registry.register(bleachbone_verge());
            registry.register(blood_celebrant());
            registry.register(blood_artist());
            registry.register(bloodstained_mire());
            registry.register(braids_arisen_nightmare());
            registry.register(brightclimb_pathway());
            registry.register(grimclimb_pathway());
            registry.register(buried_alive());
            registry.register(cataclysm());
            registry.register(cataclysmic_gearhulk());
            registry.register(charismatic_conqueror());
            registry.register(command_tower());
            registry.register(sol_ring());
            registry.register(scrubland());
            registry.register(tainted_field());
            registry.register(high_market());
            registry.register(humility());
            registry.register(vampiric_tutor());
            registry.register(flooded_strand());
            registry.register(mana_tithe());
            registry.register(marsh_flats());
            registry.register(polluted_delta());
            registry.register(rebuff_the_wicked());
            registry.register(verdant_catacombs());
            registry.register(windswept_heath());
            registry.register(lightning_greaves());
            registry.register(selfless_spirit());
            registry.register(mother_of_runes());
            registry.register(giver_of_runes());
            registry.register(selfless_savior());
            registry.register(gods_willing());
            registry.register(kami_of_false_hope());
            registry.register(shelter());
            registry.register(mox_diamond());
            registry.register(library_of_leng());
            registry.register(invisible_stalker());
            registry.register(dauthi_slayer());
            registry.register(zodiac_rooster());
            registry.register(culling_the_weak());
            registry.register(fleshbag_marauder());
            registry.register(generous_gift());
            registry.register(godless_shrine());
            registry.register(innocent_blood());
            registry.register(mana_vault());
            registry.register(merciless_executioner());
            registry.register(phyrexian_tower());
            registry.register(shattered_sanctum());
            registry.register(stroke_of_midnight());
            registry.register(vault_of_champions());
            registry.register(tayam_luminous_enigma());
            registry.register(village_rites());
            registry.register(model_of_unity());
            registry.register(manascape_refractor());
            registry.register(squirrel_nest());
            registry.register(mycosynth_lattice());
            registry.register(toph_the_first_metalbender());
            registry.register(marneus_calgar());
            registry.register(marvin_murderous_mimic());
            registry.register(rex_cyber_hound());
            registry.register(tivit_seller_of_secrets());
            registry.register(wall_of_roots());
        }

        // Non-test builds are populated from cards.json via generated parser output.
        generated_registry::register_generated_parser_cards(&mut registry);

        registry
    }

    /// Register a card definition.
    pub fn register(&mut self, def: CardDefinition) {
        self.cards.insert(def.card.name.clone(), def);
    }

    /// Look up a card by name.
    pub fn get(&self, name: &str) -> Option<&CardDefinition> {
        self.cards.get(name)
    }

    /// Get all card definitions.
    pub fn all(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values()
    }

    /// Get the number of registered cards.
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Get all creatures.
    pub fn creatures(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values().filter(|c| c.is_creature())
    }

    /// Get all spells (instants and sorceries).
    pub fn spells(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values().filter(|c| c.is_spell())
    }

    /// Get all lands.
    pub fn lands(&self) -> impl Iterator<Item = &CardDefinition> {
        self.cards.values().filter(|c| c.card.is_land())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_card_definition_creation() {
        let def = llanowar_elves();
        assert_eq!(def.name(), "Llanowar Elves");
        assert!(def.is_creature());
        assert!(!def.abilities.is_empty());
    }

    #[test]
    fn test_spell_definition() {
        let def = lightning_bolt();
        assert_eq!(def.name(), "Lightning Bolt");
        assert!(def.is_spell());
        assert!(def.spell_effect.is_some());
    }

    #[test]
    fn test_registry_lookup() {
        let registry = CardRegistry::with_builtin_cards();

        let angel = registry.get("Serra Angel");
        assert!(angel.is_some());
        assert!(angel.unwrap().is_creature());

        let bolt = registry.get("Lightning Bolt");
        assert!(bolt.is_some());
        assert!(bolt.unwrap().is_spell());
    }

    #[test]
    fn test_registry_queries() {
        let registry = CardRegistry::with_builtin_cards();

        let creatures: Vec<_> = registry.creatures().collect();
        assert!(!creatures.is_empty());

        let spells: Vec<_> = registry.spells().collect();
        assert!(!spells.is_empty());

        let lands: Vec<_> = registry.lands().collect();
        assert!(!lands.is_empty());
    }

    #[test]
    fn test_registry_count() {
        let registry = CardRegistry::with_builtin_cards();
        assert!(registry.len() > 10);
    }
}
