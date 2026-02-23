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

#[cfg(all(test, feature = "parser-tests-full"))]
mod parse_snapshots;

#[allow(dead_code)]
mod generated_registry {
    include!(concat!(env!("OUT_DIR"), "/generated_registry.rs"));
}

use crate::ability::{Ability, AbilityKind};
use crate::alternative_cast::AlternativeCastingMethod;
use crate::card::Card;
use crate::cost::OptionalCost;
use crate::effect::Effect;
use crate::static_abilities::StaticAbilityId;
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
        if !generated_definition_is_supported(&def) {
            return;
        }
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

const UNSUPPORTED_PARSER_LINE_FALLBACK_PREFIX: &str = "Unsupported parser line fallback:";

/// Returns true if a parsed definition still contains unimplemented mechanics/effects.
///
/// This is used by generated registries and reporting utilities to keep support
/// classification consistent.
pub fn generated_definition_has_unimplemented_content(definition: &CardDefinition) -> bool {
    let has_custom_static = definition.abilities.iter().any(|ability| {
        matches!(
            &ability.kind,
            AbilityKind::Static(static_ability) if static_ability.id() == StaticAbilityId::Custom
        )
    });
    if has_custom_static {
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
                if static_ability.id() == StaticAbilityId::Custom
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
    fn generated_definition_support_rejects_parser_fallback_markers() {
        let card = CardBuilder::new(CardId::new(), "Fallback Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let fallback = Ability::static_ability(StaticAbility::custom(
            "unsupported_line",
            "Unsupported parser line fallback: probe text (ParseError(\"mock\"))".to_string(),
        ))
        .with_text("probe text");
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        assert!(!generated_definition_is_supported(&definition));
    }

    #[test]
    fn generated_definition_support_rejects_custom_static_abilities() {
        let card = CardBuilder::new(CardId::new(), "Custom Probe")
            .card_types(vec![CardType::Creature])
            .build();
        let custom = Ability::static_ability(StaticAbility::custom(
            "probe_custom",
            "Probe custom rule text".to_string(),
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
        let fallback = Ability::static_ability(StaticAbility::custom(
            "unsupported_line",
            "Unsupported parser line fallback: skip me (ParseError(\"mock\"))".to_string(),
        ));
        let mut definition = CardDefinition::new(card);
        definition.abilities.push(fallback);

        let mut registry = CardRegistry::new();
        registry.register(definition);

        assert_eq!(registry.len(), 0);
        assert!(registry.get("Skipped Fallback").is_none());
    }
}
