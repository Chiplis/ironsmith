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

#[allow(dead_code)]
mod generated_registry {
    include!(concat!(env!("OUT_DIR"), "/generated_registry.rs"));
}

use crate::ability::{Ability, AbilityKind};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::Card;
use crate::cost::{OptionalCost, TotalCost};
use crate::effect::Effect;
use crate::ids::CardId;
use crate::static_abilities::StaticAbilityId;
use crate::target::ObjectFilter;
use std::collections::HashMap;
use std::sync::OnceLock;

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

    /// Additional non-printed costs paid while casting this spell.
    ///
    /// This is modeled as a full `TotalCost` so non-mana components can be paid
    /// through the unified cost pipeline.
    pub additional_cost: TotalCost,
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
            additional_cost: TotalCost::free(),
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
            additional_cost: TotalCost::free(),
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
            additional_cost: TotalCost::free(),
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
            additional_cost: TotalCost::free(),
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

    /// Returns non-mana additional cost components represented as effects.
    pub fn additional_cost_effects(&self) -> Vec<Effect> {
        self.additional_cost
            .costs()
            .iter()
            .filter_map(|component| component.effect_ref().cloned())
            .collect()
    }
}

/// Registry of all card definitions.
///
/// Provides lookup by name and other queries.
#[derive(Debug, Clone, Default)]
pub struct CardRegistry {
    /// Cards indexed by name
    cards: HashMap<String, CardDefinition>,
    /// Mapping for looking up cards by CardId without duplicating CardDefinition storage.
    names_by_id: HashMap<CardId, String>,
    /// Alias name -> canonical name (used for card-face layouts where Scryfall's
    /// `name` is "Front // Back" but the playable card name is the front face).
    aliases: HashMap<String, String>,
}

impl CardRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            cards: HashMap::new(),
            names_by_id: HashMap::new(),
            aliases: HashMap::new(),
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
            registry.register_builtin_handwritten_cards_if(|_| true);
        }

        // Non-test builds are populated from cards.json via generated parser output.
        generated_registry::register_generated_parser_cards(&mut registry);

        registry
    }

    /// Ensure cards with any of the requested names are loaded into this registry.
    ///
    /// Matching is case-insensitive and ignores surrounding whitespace.
    pub fn ensure_cards_loaded<'a>(&mut self, names: impl IntoIterator<Item = &'a str>) {
        let requested_names = names.into_iter().collect::<Vec<_>>();
        if requested_names.is_empty() {
            return;
        }

        let requested_name_keys = requested_names
            .iter()
            .map(|name| normalize_card_lookup_name(name))
            .collect::<std::collections::HashSet<_>>();

        generated_registry::register_generated_parser_cards_if_name(self, |name| {
            requested_name_keys.contains(&normalize_card_lookup_name(name))
        });

        #[cfg(test)]
        {
            // In test builds, prefer handwritten definitions for overlapping cards.
            // Generated parser cards are still loaded above for all non-handwritten names.
            let requested_keys = requested_names
                .iter()
                .map(|name| normalize_card_constructor_key(name))
                .collect::<std::collections::HashSet<_>>();
            self.register_builtin_handwritten_cards_if(|constructor_key| {
                requested_keys.contains(constructor_key)
                    || constructor_key
                        .strip_prefix("basic_")
                        .is_some_and(|stripped| requested_keys.contains(stripped))
            });
        }
    }

    /// Ensure every generated parser definition is loaded into this registry.
    pub fn ensure_all_generated_cards_loaded(&mut self) {
        #[cfg(test)]
        {
            self.register_builtin_handwritten_cards_if(|_| true);
        }
        generated_registry::register_generated_parser_cards(self);
    }

    /// Number of generated registry parse entries available for chunked preload.
    pub fn generated_parser_entry_count() -> usize {
        generated_registry::generated_parser_entry_count()
    }

    /// Generated parser card names without forcing all definitions to parse/register.
    pub fn generated_parser_card_names() -> Vec<String> {
        generated_registry::generated_parser_card_names()
    }

    /// Semantic fidelity score for a generated parser card name.
    pub fn generated_parser_semantic_score(name: &str) -> Option<f32> {
        generated_registry::generated_parser_semantic_score(name)
    }

    /// Precomputed counts of cards meeting each integer threshold from 1%..=100%.
    pub fn generated_parser_semantic_threshold_counts() -> [usize; 100] {
        generated_registry::generated_parser_semantic_threshold_counts()
    }

    /// Number of generated parser card names that have an embedded semantic score.
    pub fn generated_parser_semantic_scored_count() -> usize {
        generated_registry::generated_parser_semantic_scored_count()
    }

    /// Incrementally parse/register generated cards and return the next cursor position.
    pub fn preload_generated_cards_chunk(&mut self, cursor: usize, chunk_size: usize) -> usize {
        #[cfg(test)]
        {
            self.register_builtin_handwritten_cards_if(|_| true);
        }
        generated_registry::register_generated_parser_cards_chunk(self, cursor, chunk_size)
    }

    /// Try to compile a card by name, returning the specific error if it fails.
    ///
    /// Used to distinguish "card not in database" from "card exists but failed to compile".
    pub fn try_compile_card(name: &str) -> Result<CardDefinition, String> {
        generated_registry::try_compile_card_by_name(name)
    }

    /// Create a card registry with only the requested hand-written cards plus generated parser cards.
    #[cfg(test)]
    pub fn with_builtin_cards_for_names<'a>(names: impl IntoIterator<Item = &'a str>) -> Self {
        let mut registry = Self::new();
        registry.ensure_cards_loaded(names);
        registry
    }

    #[cfg(test)]
    fn register_builtin_handwritten_cards_if<F>(&mut self, mut include_constructor_key: F)
    where
        F: FnMut(&str) -> bool,
    {
        macro_rules! maybe_register {
            ($constructor:ident) => {
                if include_constructor_key(stringify!($constructor)) {
                    self.register($constructor());
                }
            };
        }

        maybe_register!(llanowar_elves);
        maybe_register!(chrome_mox);
        maybe_register!(command_the_mind);
        maybe_register!(serra_angel);
        maybe_register!(grizzly_bears);
        maybe_register!(lightning_bolt);
        maybe_register!(doom_blade);
        maybe_register!(demonic_tutor);
        maybe_register!(enlightened_tutor);
        maybe_register!(emrakul_the_promised_end);
        maybe_register!(everflowing_chalice);
        maybe_register!(force_of_will);
        maybe_register!(giant_growth);
        maybe_register!(mindbreak_trap);
        maybe_register!(counterspell);
        maybe_register!(dawn_charm);
        maybe_register!(swords_to_plowshares);
        maybe_register!(basic_forest);
        maybe_register!(basic_island);
        maybe_register!(basic_mountain);
        maybe_register!(basic_plains);
        maybe_register!(basic_swamp);
        maybe_register!(ornithopter);
        maybe_register!(murder_of_crows);
        maybe_register!(goblin_guide);
        maybe_register!(typhoid_rats);
        maybe_register!(vampire_nighthawk);
        maybe_register!(silhana_ledgewalker);
        maybe_register!(thorn_elemental);
        maybe_register!(mirran_crusader);
        maybe_register!(crusade);
        maybe_register!(stormbreath_dragon);
        maybe_register!(geist_of_saint_traft);
        maybe_register!(savannah_lions);
        maybe_register!(saw_in_half);
        maybe_register!(white_knight);
        maybe_register!(giant_spider);
        maybe_register!(wall_of_omens);
        maybe_register!(boggart_brute);
        maybe_register!(darksteel_colossus);
        maybe_register!(snapcaster_mage);
        maybe_register!(underworld_breach);
        maybe_register!(frogmite);
        maybe_register!(treasure_cruise);
        maybe_register!(stoke_the_flames);
        maybe_register!(reverse_engineer);
        maybe_register!(the_birth_of_meletis);
        maybe_register!(student_of_warfare);
        maybe_register!(valley_floodcaller);
        maybe_register!(yawgmoth_thran_physician);
        maybe_register!(yawgmoths_will);
        maybe_register!(butcher_ghoul);
        maybe_register!(sightless_ghoul);
        maybe_register!(hex_parasite);
        maybe_register!(fireball);
        maybe_register!(think_twice);
        maybe_register!(urzas_saga);
        maybe_register!(fate_transfer);
        maybe_register!(accursed_marauder);
        maybe_register!(accursed_duneyard);
        maybe_register!(akromas_will);
        maybe_register!(amulet_of_vigor);
        maybe_register!(ancient_tomb);
        maybe_register!(arcane_signet);
        maybe_register!(arid_mesa);
        maybe_register!(ashnods_altar);
        maybe_register!(bello_bard_of_the_brambles);
        maybe_register!(blade_of_the_bloodchief);
        maybe_register!(bleachbone_verge);
        maybe_register!(blood_celebrant);
        maybe_register!(blood_artist);
        maybe_register!(bloodstained_mire);
        maybe_register!(bosh_iron_golem);
        maybe_register!(braids_arisen_nightmare);
        maybe_register!(brightclimb_pathway);
        maybe_register!(grimclimb_pathway);
        maybe_register!(buried_alive);
        maybe_register!(cataclysm);
        maybe_register!(cataclysmic_gearhulk);
        maybe_register!(charismatic_conqueror);
        maybe_register!(command_tower);
        maybe_register!(sol_ring);
        maybe_register!(scrubland);
        maybe_register!(tainted_field);
        maybe_register!(high_market);
        maybe_register!(humility);
        maybe_register!(vampiric_tutor);
        maybe_register!(flooded_strand);
        maybe_register!(mana_tithe);
        maybe_register!(marsh_flats);
        maybe_register!(polluted_delta);
        maybe_register!(rebuff_the_wicked);
        maybe_register!(verdant_catacombs);
        maybe_register!(windswept_heath);
        maybe_register!(lightning_greaves);
        maybe_register!(selfless_spirit);
        maybe_register!(mother_of_runes);
        maybe_register!(giver_of_runes);
        maybe_register!(selfless_savior);
        maybe_register!(gods_willing);
        maybe_register!(kami_of_false_hope);
        maybe_register!(shelter);
        maybe_register!(mox_diamond);
        maybe_register!(library_of_leng);
        maybe_register!(invisible_stalker);
        maybe_register!(dauthi_slayer);
        maybe_register!(zodiac_rooster);
        maybe_register!(culling_the_weak);
        maybe_register!(fleshbag_marauder);
        maybe_register!(generous_gift);
        maybe_register!(godless_shrine);
        maybe_register!(innocent_blood);
        maybe_register!(mana_vault);
        maybe_register!(merciless_executioner);
        maybe_register!(phyrexian_tower);
        maybe_register!(shattered_sanctum);
        maybe_register!(stroke_of_midnight);
        maybe_register!(vault_of_champions);
        maybe_register!(tayam_luminous_enigma);
        maybe_register!(village_rites);
        maybe_register!(model_of_unity);
        maybe_register!(manascape_refractor);
        maybe_register!(squirrel_nest);
        maybe_register!(mycosynth_lattice);
        maybe_register!(nest_of_scarabs);
        maybe_register!(toph_the_first_metalbender);
        maybe_register!(marneus_calgar);
        maybe_register!(marvin_murderous_mimic);
        maybe_register!(rex_cyber_hound);
        maybe_register!(tivit_seller_of_secrets);
        maybe_register!(wall_of_roots);
    }

    /// Register a card definition.
    pub fn register(&mut self, def: CardDefinition) {
        if !generated_definition_is_supported(&def) {
            return;
        }
        let name = def.card.name.clone();
        self.names_by_id
            .entry(def.card.id)
            .or_insert_with(|| name.clone());
        self.cards.insert(name, def);
    }

    /// Look up a card by name.
    pub fn get(&self, name: &str) -> Option<&CardDefinition> {
        if let Some(def) = self.cards.get(name) {
            return Some(def);
        }
        let canonical = self.aliases.get(name)?;
        self.cards.get(canonical)
    }

    /// Register an alternate name for an existing definition.
    pub fn register_alias(&mut self, alias: impl Into<String>, canonical: impl Into<String>) {
        self.aliases.insert(alias.into(), canonical.into());
    }

    /// Look up a card by CardId.
    pub fn get_by_id(&self, id: CardId) -> Option<&CardDefinition> {
        let name = self.names_by_id.get(&id)?;
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
fn normalize_card_constructor_key(name: &str) -> String {
    let mut normalized = String::with_capacity(name.len());
    let mut previous_was_separator = false;

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            previous_was_separator = false;
        } else if ch == '\'' {
            // Keep possessive words aligned with constructor names:
            // "Akroma's Will" -> "akromas_will".
        } else if !previous_was_separator {
            normalized.push('_');
            previous_was_separator = true;
        }
    }

    normalized.trim_matches('_').to_string()
}

fn normalize_card_lookup_name(name: &str) -> String {
    name.trim().to_lowercase()
}

/// A lazily-constructed singleton registry for effect/runtime lookups.
///
/// Most engine logic avoids needing the registry at runtime, but mechanics like
/// flip cards need to resolve the other face's definition.
pub fn builtin_registry() -> &'static CardRegistry {
    static REGISTRY: OnceLock<CardRegistry> = OnceLock::new();
    REGISTRY.get_or_init(CardRegistry::with_builtin_cards)
}

const UNSUPPORTED_PARSER_LINE_FALLBACK_PREFIX: &str = "Unsupported parser line fallback:";

/// Returns true if a parsed definition still contains unimplemented mechanics/effects.
///
/// This is used by generated registries and reporting utilities to keep support
/// classification consistent.
pub fn generated_definition_has_unimplemented_content(definition: &CardDefinition) -> bool {
    let has_placeholder_static = definition.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if matches!(
                    static_ability.id(),
                    StaticAbilityId::KeywordMarker
                        | StaticAbilityId::RuleTextPlaceholder
                        | StaticAbilityId::KeywordFallbackText
                        | StaticAbilityId::RuleFallbackText
                        | StaticAbilityId::UnsupportedParserLine
                )
        )
    });
    if has_placeholder_static {
        return true;
    }

    // Some parsed definitions still carry raw "unimplemented_*" internals
    // (for example, fallback custom triggers).
    let raw_debug = format!("{definition:#?}").to_ascii_lowercase();
    raw_debug.contains("unimplemented")
}

/// Returns true when a generated parser definition can be safely included in the registry.
///
/// Generated wasm/demo registries should not include parser fallback placeholders that only
/// exist because unsupported mode swallowed a real parse failure.
pub(crate) fn generated_definition_is_supported(definition: &CardDefinition) -> bool {
    let has_parser_fallback_marker = definition.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability)
                if static_ability.id() == StaticAbilityId::UnsupportedParserLine
                    && static_ability
                        .display()
                        .starts_with(UNSUPPORTED_PARSER_LINE_FALLBACK_PREFIX)
        )
    });

    if has_parser_fallback_marker {
        return false;
    }

    !generated_definition_has_unimplemented_content(definition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::card::CardBuilder;
    use crate::ids::CardId;
    use crate::static_abilities::StaticAbility;
    use crate::types::CardType;
    use crate::zone::Zone;

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
        let registry =
            CardRegistry::with_builtin_cards_for_names(["Serra Angel", "Lightning Bolt", "Forest"]);

        let angel = registry.get("Serra Angel");
        assert!(angel.is_some());
        assert!(angel.unwrap().is_creature());

        let bolt = registry.get("Lightning Bolt");
        assert!(bolt.is_some());
        assert!(bolt.unwrap().is_spell());
    }

    #[test]
    fn test_registry_queries() {
        let registry =
            CardRegistry::with_builtin_cards_for_names(["Serra Angel", "Lightning Bolt", "Forest"]);

        let creatures: Vec<_> = registry.creatures().collect();
        assert!(!creatures.is_empty());

        let spells: Vec<_> = registry.spells().collect();
        assert!(!spells.is_empty());

        let lands: Vec<_> = registry.lands().collect();
        assert!(!lands.is_empty());
    }

    #[test]
    fn test_registry_count() {
        let registry =
            CardRegistry::with_builtin_cards_for_names(["Serra Angel", "Lightning Bolt", "Forest"]);
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn ensure_cards_loaded_is_incremental() {
        let mut registry = CardRegistry::new();
        assert_eq!(registry.len(), 0);

        registry.ensure_cards_loaded(["Lightning Bolt"]);
        assert_eq!(registry.len(), 1);
        assert!(registry.get("Lightning Bolt").is_some());
        assert!(registry.get("Serra Angel").is_none());

        registry.ensure_cards_loaded(["Serra Angel"]);
        assert_eq!(registry.len(), 2);
        assert!(registry.get("Serra Angel").is_some());
    }

    #[test]
    fn ensure_cards_loaded_normalizes_input_names() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["  lightning bolt  ", " FoReSt "]);

        assert!(registry.get("Lightning Bolt").is_some());
        assert!(registry.get("Forest").is_some());
        assert_eq!(registry.len(), 2);
    }

    #[cfg(feature = "generated-registry")]
    #[test]
    fn ensure_cards_loaded_can_load_generated_cards() {
        let mut registry = CardRegistry::new();
        registry.ensure_cards_loaded(["Conclave Evangelist"]);
        assert!(registry.get("Conclave Evangelist").is_some());
    }

    #[test]
    fn generated_definition_support_accepts_regular_definition() {
        let card = CardBuilder::new(CardId::new(), "Support Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let mut definition = CardDefinition::new(card);
        definition
            .abilities
            .push(Ability::static_ability(StaticAbility::flying()));

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn parse_discard_this_card_activated_ability_as_hand_zone_ability() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Bloodrush Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{R}, Discard this card: Target attacking creature gets +3/+3 until end of turn",
            )
            .expect("discard-this-card activated ability should parse");

        let (ability, activated) = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some((ability, activated)),
                _ => None,
            })
            .expect("expected an activated ability");

        assert!(
            ability.functions_in(&Zone::Hand),
            "expected discard-this-card ability to function in hand"
        );
        assert!(
            !ability.functions_in(&Zone::Battlefield),
            "expected discard-this-card ability to not function on battlefield"
        );

        let costs = activated.mana_cost.display().to_ascii_lowercase();
        assert!(
            costs.contains("discard this card"),
            "expected activated cost to include discard-this-card, got: {costs}"
        );
    }

    #[test]
    fn parse_if_this_is_tapped_predicate_as_intervening_if() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Storage Land Probe")
            .card_types(vec![CardType::Land])
            .parse_text("At the beginning of your upkeep, if this land is tapped, put a storage counter on it.")
            .expect("tapped predicate trigger should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected a triggered ability");

        assert_eq!(
            triggered.intervening_if,
            Some(crate::ConditionExpr::SourceIsTapped),
            "expected intervening-if to be SourceIsTapped"
        );
    }

    #[test]
    fn parse_if_there_are_no_counters_on_this_predicate() {
        let def = CardDefinitionBuilder::new(CardId::new(), "Depletion Land Probe")
            .card_types(vec![CardType::Land])
            .parse_text("If there are no depletion counters on this land, sacrifice it.")
            .expect("no-counters predicate should parse");

        // Ensure we actually produced an effect (not a dropped sentence).
        assert!(
            def.spell_effect
                .as_ref()
                .is_some_and(|effects| !effects.is_empty())
                || !def.abilities.is_empty(),
            "expected parsed effects or abilities"
        );
    }

    #[test]
    fn parse_add_mana_for_each_counter_removed_this_way_uses_x_value() {
        use crate::ability::AbilityKind;
        use crate::effect::Value;
        use crate::effects::mana::AddScaledManaEffect;

        let def = CardDefinitionBuilder::new(CardId::new(), "Storage Land Probe")
            .card_types(vec![CardType::Land])
            .parse_text("{1}, Remove any number of storage counters from this land: Add {W} for each storage counter removed this way.")
            .expect("storage land mana scaling should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected an activated ability");

        let scaled = activated
            .effects
            .iter()
            .find_map(|effect| effect.downcast_ref::<AddScaledManaEffect>())
            .expect("expected scaled mana effect");

        assert_eq!(scaled.amount, Value::X);
    }

    #[test]
    fn parse_activate_no_more_than_twice_each_turn_as_activation_limit() {
        use crate::ability::AbilityKind;

        let def = CardDefinitionBuilder::new(CardId::new(), "Activation Limit Probe")
            .card_types(vec![CardType::Creature])
            .parse_text("{B}: This creature gets +0/+1 until end of turn. Activate no more than twice each turn.")
            .expect("activation limit clause should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(activated) => Some(activated),
                _ => None,
            })
            .expect("expected an activated ability");

        assert_eq!(
            activated.activation_condition,
            Some(crate::ConditionExpr::MaxActivationsPerTurn(2))
        );
    }

    #[test]
    fn parse_flip_it_clause_as_flip_effect() {
        use crate::ability::AbilityKind;
        use crate::effects::{FlipEffect, MayEffect, SequenceEffect, TaggedEffect, WithIdEffect};

        fn contains_flip(effect: &crate::effect::Effect) -> bool {
            if effect.downcast_ref::<FlipEffect>().is_some() {
                return true;
            }
            if let Some(may) = effect.downcast_ref::<MayEffect>() {
                return may.effects.iter().any(contains_flip);
            }
            if let Some(seq) = effect.downcast_ref::<SequenceEffect>() {
                return seq.effects.iter().any(contains_flip);
            }
            if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
                return contains_flip(&tagged.effect);
            }
            if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
                return contains_flip(&with_id.effect);
            }
            false
        }

        let def = CardDefinitionBuilder::new(CardId::new(), "Flip Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "At the beginning of the end step, if there are two or more ki counters on this creature, you may flip it.",
            )
            .expect("flip clause should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected a triggered ability");

        assert!(
            triggered.effects.iter().any(contains_flip),
            "expected FlipEffect in triggered effects"
        );
    }

    #[test]
    fn parse_assigns_no_combat_damage_clause_as_combat_prevention() {
        use crate::ability::AbilityKind;
        use crate::effects::{
            IfEffect, MayEffect, PreventAllCombatDamageFromEffect, SequenceEffect, TaggedEffect,
            WithIdEffect,
        };

        fn contains_prevent(effect: &crate::effect::Effect) -> bool {
            if effect
                .downcast_ref::<PreventAllCombatDamageFromEffect>()
                .is_some()
            {
                return true;
            }
            if let Some(may) = effect.downcast_ref::<MayEffect>() {
                return may.effects.iter().any(contains_prevent);
            }
            if let Some(if_effect) = effect.downcast_ref::<IfEffect>() {
                return if_effect.then.iter().any(contains_prevent)
                    || if_effect.else_.iter().any(contains_prevent);
            }
            if let Some(seq) = effect.downcast_ref::<SequenceEffect>() {
                return seq.effects.iter().any(contains_prevent);
            }
            if let Some(tagged) = effect.downcast_ref::<TaggedEffect>() {
                return contains_prevent(&tagged.effect);
            }
            if let Some(with_id) = effect.downcast_ref::<WithIdEffect>() {
                return contains_prevent(&with_id.effect);
            }
            false
        }

        let def = CardDefinitionBuilder::new(CardId::new(), "Laccolith Probe")
            .card_types(vec![CardType::Creature])
            .parse_text("Whenever this creature becomes blocked, you may have it deal damage equal to its power to target creature. If you do, this creature assigns no combat damage this turn.")
            .expect("assigns-no-combat-damage clause should parse");

        let triggered = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Triggered(triggered) => Some(triggered),
                _ => None,
            })
            .expect("expected a triggered ability");

        assert!(
            triggered.effects.iter().any(contains_prevent),
            "expected PreventAllCombatDamageFromEffect in triggered effects"
        );
    }

    #[test]
    fn parse_look_at_top_then_put_some_into_hand_rest_into_graveyard() {
        use crate::effects::{ChooseObjectsEffect, LookAtTopCardsEffect};

        let def = CardDefinitionBuilder::new(CardId::new(), "Ancestral Memories Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text(
                "Look at the top seven cards of your library. Put two of them into your hand and the rest into your graveyard.",
            )
            .expect("look/put partition clause should parse");

        let effects = def.spell_effect.as_ref().expect("expected spell effects");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<LookAtTopCardsEffect>().is_some()),
            "expected LookAtTopCardsEffect in compiled effects"
        );
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ChooseObjectsEffect>().is_some()),
            "expected ChooseObjectsEffect in compiled effects"
        );
    }

    #[test]
    fn parse_look_at_top_then_put_them_back_in_any_order() {
        use crate::effects::{LookAtTopCardsEffect, ReorderLibraryTopEffect};

        let def = CardDefinitionBuilder::new(CardId::new(), "Look Reorder Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Look at the top three cards of your library. Put them back in any order.")
            .expect("look/reorder clause should parse");

        let effects = def.spell_effect.as_ref().expect("expected spell effects");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<LookAtTopCardsEffect>().is_some()),
            "expected LookAtTopCardsEffect in compiled effects"
        );
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<ReorderLibraryTopEffect>().is_some()),
            "expected ReorderLibraryTopEffect in compiled effects"
        );
    }

    #[test]
    fn parse_discover_keyword_action_clause() {
        use crate::effects::DiscoverEffect;

        let def = CardDefinitionBuilder::new(CardId::new(), "Discover Probe")
            .card_types(vec![CardType::Sorcery])
            .parse_text("Discover 4.")
            .expect("discover clause should parse");

        let effects = def.spell_effect.as_ref().expect("expected spell effects");
        assert!(
            effects
                .iter()
                .any(|e| e.downcast_ref::<DiscoverEffect>().is_some()),
            "expected DiscoverEffect in compiled effects"
        );
    }

    #[test]
    fn parse_become_basic_land_type_of_your_choice_until_eot() {
        use crate::effects::BecomeBasicLandTypeChoiceEffect;

        let def = CardDefinitionBuilder::new(CardId::new(), "Become Land Type Probe")
            .card_types(vec![CardType::Creature])
            .parse_text(
                "{T}: Target land becomes the basic land type of your choice until end of turn.",
            )
            .expect("basic land type choice become clause should parse");

        let activated = def
            .abilities
            .iter()
            .find_map(|ability| match &ability.kind {
                AbilityKind::Activated(act) => Some(act),
                _ => None,
            })
            .expect("expected an activated ability");

        assert!(
            activated.effects.iter().any(|e| e
                .downcast_ref::<BecomeBasicLandTypeChoiceEffect>()
                .is_some()),
            "expected BecomeBasicLandTypeChoiceEffect in activated effects"
        );
    }

    #[test]
    fn parse_can_block_additional_creature_each_combat_static_ability() {
        use crate::static_abilities::StaticAbilityId;

        let def = CardDefinitionBuilder::new(CardId::new(), "Extra Block Probe")
            .card_types(vec![CardType::Creature])
            .parse_text("This creature can block an additional creature each combat.")
            .expect("extra block static ability should parse");

        let has = def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Static(sa) => {
                sa.id() == StaticAbilityId::CanBlockAdditionalCreatureEachCombat
            }
            _ => false,
        });
        assert!(
            has,
            "expected CanBlockAdditionalCreatureEachCombat static ability"
        );
    }

    #[test]
    fn parse_enchanted_creature_cant_attack_or_block_static() {
        use crate::static_abilities::StaticAbilityId;

        let def = CardDefinitionBuilder::new(CardId::new(), "Aura Probe")
            .card_types(vec![CardType::Enchantment])
            .parse_text("Enchant creature\nEnchanted creature can't attack or block.")
            .expect("attached cant attack/block line should parse");

        assert!(
            def.aura_attach_filter.is_some(),
            "expected aura attach filter from 'Enchant creature' line"
        );

        let has = def.abilities.iter().any(|ability| match &ability.kind {
            AbilityKind::Static(sa) => sa.id() == StaticAbilityId::AttachedAbilityGrant,
            _ => false,
        });
        assert!(has, "expected AttachedAbilityGrant static ability on aura");
    }

    #[test]
    fn generated_definition_support_rejects_parser_fallback_markers() {
        let card = CardBuilder::new(CardId::new(), "Fallback Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let fallback = Ability::static_ability(StaticAbility::unsupported_parser_line(
            "probe text",
            "ParseError(\"mock\")",
        ))
        .with_text("probe text");
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        assert!(!generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_rejects_placeholder_static_abilities() {
        let card = CardBuilder::new(CardId::new(), "Custom Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let custom = Ability::static_ability(StaticAbility::rule_text_placeholder(
            "Probe custom rule text",
        ));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(custom);

        assert!(!generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_parsed_prowess() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Prowess Probe")
            .parse_text("Prowess")
            .expect("prowess parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_rejects_parsed_cipher_marker() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Cipher Probe")
            .parse_text("Cipher")
            .expect("cipher parse should succeed");

        assert!(!generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_parsed_split_second() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Split Second Probe")
            .parse_text("Split second\nDraw a card.")
            .expect("split second parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_parsed_riot() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Riot Probe")
            .parse_text("Riot")
            .expect("riot parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_parsed_unleash() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Unleash Probe")
            .parse_text("Unleash")
            .expect("unleash parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_parsed_unearth() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Unearth Probe")
            .parse_text(
                "Mana cost: {1}{B}\nType: Creature — Zombie\nPower/Toughness: 2/1\nUnearth {2}{B}",
            )
            .expect("unearth parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_parsed_rebound() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Rebound Probe")
            .parse_text("Mana cost: {1}{U}\nType: Instant\nGain 1 life.\nRebound")
            .expect("rebound parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_parsed_cascade() {
        let definition = CardDefinitionBuilder::new(CardId::new(), "Cascade Probe")
            .parse_text("Mana cost: {2}{R}\nType: Sorcery\nDraw a card.\nCascade")
            .expect("cascade parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_saruman_of_many_colors() {
        let text = "Ward—Discard an enchantment, instant, or sorcery card.\nWhenever you cast your second spell each turn, each opponent mills two cards. When one or more cards are milled this way, exile target enchantment, instant, or sorcery card with equal or lesser mana value than that spell from an opponent's graveyard. Copy the exiled card. You may cast the copy without paying its mana cost.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "Saruman of Many Colors")
            .parse_text(text)
            .expect("saruman parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_a_good_day_to_pie() {
        let text = "Tap up to two target creatures.\nWhenever you put a name sticker on a creature, you may return this card from your graveyard to your hand.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A Good Day to Pie")
            .parse_text(text)
            .expect("a good day to pie parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_a_asari_captain() {
        let text = "Trample, haste\nWhenever a Samurai or Warrior you control attacks alone, it gets +1/+0 until end of turn for each Samurai or Warrior you control.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Asari Captain")
            .parse_text(text)
            .expect("a-asari captain parse should succeed");

        assert!(generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_accepts_a_brine_comber() {
        let text = "Mana cost: {1}{W}{U}\nType: Creature — Spirit // Enchantment — Aura\nPower/Toughness: 2/2\nWhenever this creature enters or becomes the target of an Aura spell, create a 1/1 white Spirit creature token with flying.\nDisturb {W}{U} (You may cast this card from your graveyard transformed for its disturb cost.)";
        let definition =
            CardDefinitionBuilder::new(CardId::new(), "A-Brine Comber // A-Brinebound Gift")
                .parse_text(text)
                .expect("a-brine comber parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_devoted_grafkeeper() {
        let text = "Mana cost: {W}{U}\nType: Creature — Human Peasant // Creature — Spirit\nPower/Toughness: 2/2\nWhen Devoted Grafkeeper enters, mill four cards.\nWhenever you cast a spell from your graveyard, tap target creature you don't control.\nDisturb {1}{W}{U} (You may cast this card from your graveyard transformed for its disturb cost.)";
        let definition = CardDefinitionBuilder::new(
            CardId::new(),
            "A-Devoted Grafkeeper // A-Departed Soulkeeper",
        )
        .parse_text(text)
        .expect("a-devoted grafkeeper parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_dokuchi_silencer() {
        let text = "Mana cost: {1}{B}\nType: Creature — Human Ninja\nPower/Toughness: 2/1\nNinjutsu {1}{B} ({1}{B}, Return an unblocked attacker you control to hand: Put this card onto the battlefield from your hand tapped and attacking.)\nWhenever Dokuchi Silencer deals combat damage to a player, you may discard a card. When you do, destroy target creature or planeswalker that player controls.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Dokuchi Silencer")
            .parse_text(text)
            .expect("a-dokuchi silencer parse should succeed");

        assert!(generated_definition_is_supported(&definition));

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_deepcavern_imp() {
        let text = "Mana cost: {2}{B}\nType: Creature — Imp Rebel\nPower/Toughness: 2/2\nFlying, haste\nEcho—Discard a card. (At the beginning of your upkeep, if this came under your control since the beginning of your last upkeep, sacrifice it unless you pay its echo cost.)";
        let definition = CardDefinitionBuilder::new(CardId::new(), "Deepcavern Imp")
            .parse_text(text)
            .expect("deepcavern imp parse should succeed");

        assert!(generated_definition_is_supported(&definition));

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_metropolis_angel() {
        let text = "Mana cost: {3}{W}{U}\nType: Creature — Angel Soldier\nPower/Toughness: 3/3\nFlying\nWhenever you attack with one or more creatures with counters on them, draw a card.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Metropolis Angel")
            .parse_text(text)
            .expect("a-metropolis angel parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_nadu_winged_wisdom() {
        let text = "Mana cost: {1}{G}{U}\nType: Legendary Creature — Bird Wizard\nPower/Toughness: 3/4\nFlying\nWhenever a creature you control becomes the target of a spell or ability, reveal the top card of your library. If it's a land card, put it onto the battlefield. Otherwise, put it into your hand. This ability triggers only twice each turn.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Nadu, Winged Wisdom")
            .parse_text(text)
            .expect("a-nadu winged wisdom parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_radha_coalition_warlord() {
        let text = "Mana cost: {1}{R}{G}\nType: Legendary Creature — Elf Warrior\nPower/Toughness: 3/3\nDomain — Whenever Radha, Coalition Warlord enters or becomes tapped, another target creature you control gets +X/+X until end of turn, where X is the number of basic land types among lands you control.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Radha, Coalition Warlord")
            .parse_text(text)
            .expect("a-radha coalition warlord parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_rockslide_sorcerer() {
        let text = "Mana cost: {2}{R}\nType: Creature — Human Wizard\nPower/Toughness: 2/2\nWhenever you cast an instant, sorcery, or Wizard spell, Rockslide Sorcerer deals 1 damage to any target.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Rockslide Sorcerer")
            .parse_text(text)
            .expect("a-rockslide sorcerer parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_shipwreck_sifters() {
        let text = "Mana cost: {1}{U}\nType: Creature — Spirit\nPower/Toughness: 1/2\nWhen Shipwreck Sifters enters, draw a card, then discard a card.\nWhenever a Spirit card or a card with disturb is put into your graveyard from anywhere, put a +1/+1 counter on Shipwreck Sifters.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Shipwreck Sifters")
            .parse_text(text)
            .expect("a-shipwreck sifters parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_symmetry_sage() {
        let text = "Mana cost: {U}\nType: Creature — Human Wizard\nPower/Toughness: 0/3\nFlying\nMagecraft — Whenever you cast or copy an instant or sorcery spell, target creature you control has base power 3 until end of turn.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Symmetry Sage")
            .parse_text(text)
            .expect("a-symmetry sage parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn generated_definition_support_accepts_a_vampire_scrivener() {
        let text = "Mana cost: {3}{B}\nType: Creature — Vampire Warlock\nPower/Toughness: 2/2\nFlying\nWhenever you gain life during your turn, put a +1/+1 counter on Vampire Scrivener.\nWhenever you lose life during your turn, put a +1/+1 counter on Vampire Scrivener.";
        let definition = CardDefinitionBuilder::new(CardId::new(), "A-Vampire Scrivener")
            .parse_text(text)
            .expect("a-vampire scrivener parse should succeed");

        let debug = format!("{definition:#?}").to_ascii_lowercase();
        assert!(!debug.contains("unimplemented"));
    }

    #[test]
    fn registry_skips_parser_fallback_definitions() {
        let card = CardBuilder::new(CardId::new(), "Skipped Fallback")
            .card_types(vec![CardType::Creature])
            .build();
        let fallback = Ability::static_ability(StaticAbility::unsupported_parser_line(
            "skip me",
            "ParseError(\"mock\")",
        ));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        let mut registry = CardRegistry::new();
        registry.register(definition);

        assert_eq!(registry.len(), 0);
        assert!(registry.get("Skipped Fallback").is_none());
    }
}
